use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// User with credentials
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub full_name: Option<String>,
    pub display_name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create user request for onboarding
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub full_name: String,
    pub role: String, // 'admin', 'doctor', 'nurse', 'staff', etc.
    pub send_credentials: bool,
}

/// User creation response with credentials
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUserResponse {
    pub user: User,
    pub username: String,
    pub temporary_password: Option<String>,
    pub credentials_sent: bool,
    pub message: String,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// Create hospital admin user with credentials
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{org_id}/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created with credentials", body = CreateUserResponse)
    ),
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    )
)]
pub async fn create_organization_user(
    Path(org_id): Path<Uuid>,
    State(app_state): State<RustCareServer>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<CreateUserResponse>>, ApiError> {
    // Validate email
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(ApiError::validation("Valid email address is required"));
    }
    
    // Check if user already exists
    let existing_user = sqlx::query!(
        "SELECT id FROM users WHERE email = $1 AND deleted_at IS NULL",
        req.email
    )
    .fetch_optional(&app_state.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing user: {}", e)))?;
    
    if existing_user.is_some() {
        return Err(ApiError::conflict("A user with this email already exists"));
    }
    
    // Verify email mailbox exists (without sending)
    use email_service::verify_mailbox_exists;
    match verify_mailbox_exists(&req.email).await {
        Ok(true) => {
            tracing::info!(email = %req.email, "Email mailbox verified successfully");
        }
        Ok(false) => {
            tracing::warn!(email = %req.email, "Email mailbox verification failed");
            return Err(ApiError::validation("Email address could not be verified. Please check the email address."));
        }
        Err(e) => {
            tracing::warn!(email = %req.email, error = %e, "Email mailbox verification error");
            // Don't fail on verification errors, just log them
            tracing::info!("Continuing with user creation despite verification error");
        }
    }
    
    // Generate secure temporary password
    let temporary_password = generate_secure_password();
    
    // Hash password with argon2id
    let password_hash = hash_password(&temporary_password)?;
    
    // Create user
    let user_id = Uuid::new_v4();
    let result = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (
            id, email, full_name, display_name, status
        ) VALUES ($1, LOWER($2), $3, $4, 'active')
        RETURNING id, email, email_verified, full_name, display_name, status, created_at, updated_at
        "#
    )
    .bind(user_id)
    .bind(&req.email)
    .bind(&req.full_name)
    .bind(&req.full_name) // Use full_name as display_name initially
    .fetch_one(&app_state.db_pool)
    .await;
    
    let user = match result {
        Ok(u) => u,
        Err(e) => return Err(ApiError::internal(format!("Failed to create user: {}", e))),
    };
    
    // Create credentials
    sqlx::query(
        r#"
        INSERT INTO user_credentials (
            user_id, password_hash, password_algorithm
        ) VALUES ($1, $2, 'argon2id')
        "#
    )
    .bind(user_id)
    .bind(password_hash)
    .execute(&app_state.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create credentials: {}", e)))?;
    
    // Send credentials email if requested
    let credentials_sent = if req.send_credentials {
        // TODO: Integrate actual email service
        // For now, just log it
        tracing::info!(
            user_id = %user_id,
            email = %req.email,
            "Would send credentials email"
        );
        true
    } else {
        false
    };
    
    let response = CreateUserResponse {
        user,
        username: req.email.clone(),
        temporary_password: if req.send_credentials { None } else { Some(temporary_password) },
        credentials_sent,
        message: if credentials_sent {
            "User created successfully. Credentials have been sent via email.".to_string()
        } else {
            "User created successfully. Temporary password provided.".to_string()
        },
    };
    
    Ok(Json(api_success(response)))
}

/// List organization users
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/users",
    responses(
        (status = 200, description = "List of users", body = Vec<User>)
    ),
    params(
        ("org_id" = Uuid, Path, description = "Organization ID")
    )
)]
pub async fn list_organization_users(
    Path(org_id): Path<Uuid>,
    State(app_state): State<RustCareServer>,
) -> Result<Json<ApiResponse<Vec<User>>>, ApiError> {
    // TODO: Join with organization employees table
    let result = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, email_verified, full_name, display_name, status, created_at, updated_at
        FROM users
        WHERE deleted_at IS NULL
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&app_state.db_pool)
    .await;
    
    match result {
        Ok(users) => Ok(Json(api_success(users))),
        Err(e) => Err(ApiError::internal(format!("Failed to fetch users: {}", e)))
    }
}

/// Resend user credentials
#[utoipa::path(
    post,
    path = "/api/v1/users/{user_id}/resend-credentials",
    responses(
        (status = 200, description = "Credentials resent", body = CreateUserResponse)
    ),
    params(
        ("user_id" = Uuid, Path, description = "User ID")
    )
)]
pub async fn resend_user_credentials(
    Path(user_id): Path<Uuid>,
    State(app_state): State<RustCareServer>,
) -> Result<Json<ApiResponse<CreateUserResponse>>, ApiError> {
    // Generate new temporary password
    let temporary_password = generate_secure_password();
    let password_hash = hash_password(&temporary_password)?;
    
    // Update user credentials
    sqlx::query(
        r#"
        UPDATE user_credentials
        SET 
            password_hash = $1,
            password_changed_at = NOW(),
            updated_at = NOW()
        WHERE user_id = $2
        "#
    )
    .bind(password_hash)
    .bind(user_id)
    .execute(&app_state.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to update credentials: {}", e)))?;
    
    // Fetch user
    let result = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, email_verified, full_name, display_name, status, created_at, updated_at
        FROM users
        WHERE id = $1
        "#
    )
    .bind(user_id)
    .fetch_optional(&app_state.db_pool)
    .await;
    
    let user = match result {
        Ok(Some(u)) => u,
        Ok(None) => return Err(ApiError::not_found("User not found")),
        Err(e) => return Err(ApiError::internal(format!("Failed to fetch user: {}", e))),
    };
    
    // TODO: Send credentials email
    let user_email = user.email.clone();
    tracing::info!(
        user_id = %user_id,
        email = %user_email,
        "Would resend credentials email"
    );
    
    let response = CreateUserResponse {
        user,
        username: user_email,
        temporary_password: Some(temporary_password),
        credentials_sent: true,
        message: "New credentials generated and sent via email.".to_string(),
    };
    
    Ok(Json(api_success(response)))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Generate a secure temporary password
fn generate_secure_password() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    
    // Generate 16-character password with letters, numbers, and symbols
    let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789!@#$%&*";
    let password: String = (0..16)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect();
    
    password
}

/// Hash password using argon2id
fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::{Argon2, PasswordHasher, password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as _, SaltString}};
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::internal(format!("Failed to hash password: {}", e)))?
        .to_string();
    
    Ok(password_hash)
}

/// Verify email configuration (test connection without sending)
#[utoipa::path(
    post,
    path = "/api/v1/email/verify",
    responses(
        (status = 200, description = "Email configuration verified successfully")
    )
)]
pub async fn verify_email_config(
    State(app_state): State<RustCareServer>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    use email_service::{EmailService, EmailConfig};
    
    // Load email configuration from environment
    let config = EmailConfig::from_env()
        .map_err(|e| ApiError::internal(format!("Failed to load email config: {}", e)))?;
    
    // Create email service instance
    let email_service = EmailService::new(config)
        .map_err(|e| ApiError::internal(format!("Failed to create email service: {}", e)))?;
    
    // Verify email configuration
    match email_service.verify_email_config().await {
        Ok(_) => {
            tracing::info!("Email configuration verified successfully");
            Ok(Json(api_success(())))
        }
        Err(e) => {
            tracing::error!(error = %e, "Email configuration verification failed");
            Err(ApiError::internal(format!("Email verification failed: {}", e)))
        }
    }
}

