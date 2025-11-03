//! Secrets management API handlers
//! 
//! Provides secure secret storage, retrieval, and rotation through multiple providers

use axum::{
    extract::{Path, Query, State, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};
use secrets_service::SecretProvider;
use crate::server::RustCareServer;
use crate::middleware::AuthContext;
use crate::error::{ApiError, ApiResponse, api_success};
use crate::types::pagination::PaginationParams;

type Result<T> = std::result::Result<T, ApiError>;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SecretResponse {
    /// Secret key
    pub key: String,
    /// Secret value (only returned on get operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Secret version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Created timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Updated timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Expiration timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Rotation enabled
    pub rotation_enabled: bool,
    /// Rotation interval in days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_interval_days: Option<u32>,
    /// Tags
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSecretRequest {
    /// Secret key/name
    pub key: String,
    /// Secret value
    pub value: String,
    /// Enable automatic rotation
    #[serde(default)]
    pub rotation_enabled: bool,
    /// Rotation interval in days
    pub rotation_interval_days: Option<u32>,
    /// Expiration date (ISO 8601)
    pub expires_at: Option<String>,
    /// Tags/labels
    #[serde(default)]
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSecretRequest {
    /// New secret value
    pub value: String,
    /// Enable automatic rotation
    pub rotation_enabled: Option<bool>,
    /// Rotation interval in days
    pub rotation_interval_days: Option<u32>,
    /// Expiration date (ISO 8601)
    pub expires_at: Option<String>,
    /// Tags/labels
    pub tags: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecretListResponse {
    /// List of secret keys
    pub secrets: Vec<String>,
    /// Total count
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecretVersionsResponse {
    /// Secret key
    pub key: String,
    /// List of versions
    pub versions: Vec<String>,
    /// Total count
    pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RotateSecretResponse {
    /// Secret key
    pub key: String,
    /// New version after rotation
    pub new_version: String,
    /// Rotation timestamp
    pub rotated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthCheckResponse {
    /// Overall health status
    pub healthy: bool,
    /// Status message
    pub message: String,
    /// Response latency in milliseconds
    pub latency_ms: u64,
    /// Last check timestamp
    pub last_check: String,
}

// ============================================================================
// API Handlers
// ============================================================================

/// Query parameters for listing secrets
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSecretsParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List all secrets
/// 
/// Returns a list of all secret keys (values are not included)
#[utoipa::path(
    get,
    path = "/api/v1/secrets",
    params(ListSecretsParams),
    responses(
        (status = 200, description = "Secrets retrieved successfully", body = Vec<String>),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Service unavailable"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn list_secrets(
    State(server): State<RustCareServer>,
    Query(params): Query<ListSecretsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<String>>>> {
    // Get secrets manager
    let secrets_manager = server.secrets_manager()
        .ok_or_else(|| ApiError::service_unavailable("Secrets manager not configured"))?;
    
    let mut keys = secrets_manager.list_secrets()
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list secrets: {}", e)))?;
    
    // Apply pagination
    let total_count = keys.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_keys: Vec<String> = keys
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_keys, metadata)))
}

/// Get a secret by key
/// 
/// Retrieves the current version of a secret
#[utoipa::path(
    get,
    path = "/api/v1/secrets/{key}",
    params(("key" = String, Path, description = "Secret key")),
    responses(
        (status = 200, description = "Secret metadata retrieved", body = SecretResponse),
        (status = 404, description = "Secret not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn get_secret(
    State(server): State<RustCareServer>,
    Path(key): Path<String>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<SecretResponse>>> {
    // Get secrets manager
    let secrets_manager = server.secrets_manager()
        .ok_or_else(|| ApiError::service_unavailable("Secrets manager not configured"))?;
    
    let secret = secrets_manager.get_secret(&key)
        .await
        .map_err(|e| {
            use secrets_service::SecretsError;
            match e {
                SecretsError::NotFound(_) => ApiError::not_found("secret"),
                _ => ApiError::internal(format!("Failed to get secret '{}': {}", key, e)),
            }
        })?;
    
    let response = SecretResponse {
        key: secret.metadata.key,
        value: None, // Never return actual value in production for security
        version: secret.metadata.version,
        created_at: secret.metadata.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: secret.metadata.updated_at.map(|dt| dt.to_rfc3339()),
        expires_at: secret.metadata.expires_at.map(|dt| dt.to_rfc3339()),
        rotation_enabled: secret.metadata.rotation_enabled,
        rotation_interval_days: secret.metadata.rotation_interval_days,
        tags: secret.metadata.tags,
    };
    
    Ok(Json(api_success(response)))
}

/// Create a new secret
/// 
/// Stores a new secret with optional rotation and expiration settings
#[utoipa::path(
    post,
    path = "/api/v1/secrets",
    request_body = CreateSecretRequest,
    responses(
        (status = 201, description = "Secret created", body = SecretResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn create_secret(
    State(_server): State<RustCareServer>,
    Json(request): Json<CreateSecretRequest>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SecretResponse>>> {
    // TODO: Implement with SecretsManager
    // let secrets_manager = server.secrets_manager();
    // let metadata = SecretMetadata {
    //     key: request.key.clone(),
    //     version: None,
    //     created_at: Some(chrono::Utc::now()),
    //     updated_at: Some(chrono::Utc::now()),
    //     expires_at: request.expires_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&chrono::Utc))),
    //     rotation_enabled: request.rotation_enabled,
    //     rotation_interval_days: request.rotation_interval_days,
    //     tags: request.tags,
    // };
    // secrets_manager.set_secret(&request.key, &request.value, Some(metadata)).await
    //     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Mock response for now
    let response = SecretResponse {
        key: request.key,
        value: None, // Don't return value after creation
        version: Some("v1".to_string()),
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        expires_at: request.expires_at,
        rotation_enabled: request.rotation_enabled,
        rotation_interval_days: request.rotation_interval_days,
        tags: request.tags,
    };
    
    Ok(Json(api_success(response)))
}

/// Update an existing secret
/// 
/// Updates the value and/or settings of an existing secret
#[utoipa::path(
    put,
    path = "/api/v1/secrets/{key}",
    params(("key" = String, Path, description = "Secret key")),
    request_body = UpdateSecretRequest,
    responses(
        (status = 200, description = "Secret updated", body = SecretResponse),
        (status = 404, description = "Secret not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn update_secret(
    State(_server): State<RustCareServer>,
    Path(key): Path<String>,
    Json(request): Json<UpdateSecretRequest>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SecretResponse>>> {
    // TODO: Implement with SecretsManager
    
    // Mock response for now
    let response = SecretResponse {
        key: key.clone(),
        value: None,
        version: Some("v2".to_string()),
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        expires_at: request.expires_at,
        rotation_enabled: request.rotation_enabled.unwrap_or(false),
        rotation_interval_days: request.rotation_interval_days,
        tags: request.tags.unwrap_or_default(),
    };
    
    Ok(Json(api_success(response)))
}

/// Delete a secret
/// 
/// Permanently removes a secret and all its versions
#[utoipa::path(
    delete,
    path = "/api/v1/secrets/{key}",
    params(("key" = String, Path, description = "Secret key")),
    responses(
        (status = 204, description = "Secret deleted"),
        (status = 404, description = "Secret not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn delete_secret(
    State(_server): State<RustCareServer>,
    Path(_key): Path<String>,
    _auth: AuthContext,
) -> Result<StatusCode> {
    // TODO: Implement with SecretsManager
    // let secrets_manager = server.secrets_manager();
    // secrets_manager.delete_secret(&key).await.map_err(|e| match e {
    //     SecretsError::NotFound(_) => StatusCode::NOT_FOUND,
    //     _ => StatusCode::INTERNAL_SERVER_ERROR,
    // })?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Query parameters for listing secret versions
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSecretVersionsParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List secret versions
/// 
/// Returns all available versions for a specific secret
#[utoipa::path(
    get,
    path = "/api/v1/secrets/{key}/versions",
    params(
        ("key" = String, Path, description = "Secret key"),
        ListSecretVersionsParams
    ),
    responses(
        (status = 200, description = "Secret versions retrieved", body = Vec<String>),
        (status = 404, description = "Secret not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn list_secret_versions(
    State(_server): State<RustCareServer>,
    Path(key): Path<String>,
    Query(params): Query<ListSecretVersionsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<String>>>> {
    // TODO: Implement with SecretsManager
    
    // Mock response for now
    let mut versions = vec!["v3".to_string(), "v2".to_string(), "v1".to_string()];
    
    // Apply pagination
    let total_count = versions.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_versions: Vec<String> = versions
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_versions, metadata)))
}

/// Get a specific version of a secret
/// 
/// Retrieves a historical version of a secret
#[utoipa::path(
    get,
    path = "/api/v1/secrets/{key}/versions/{version}",
    params(
        ("key" = String, Path, description = "Secret key"),
        ("version" = String, Path, description = "Secret version"),
    ),
    responses(
        (status = 200, description = "Secret version retrieved", body = SecretResponse),
        (status = 404, description = "Secret/version not found"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn get_secret_version(
    State(_server): State<RustCareServer>,
    Path((key, version)): Path<(String, String)>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SecretResponse>>> {
    // TODO: Implement with SecretsManager
    
    // Mock response for now
    let response = SecretResponse {
        key: key.clone(),
        value: Some("********".to_string()),
        version: Some(version),
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        expires_at: None,
        rotation_enabled: true,
        rotation_interval_days: Some(90),
        tags: std::collections::HashMap::new(),
    };
    
    Ok(Json(api_success(response)))
}

/// Rotate a secret
/// 
/// Generates a new value for the secret and creates a new version
#[utoipa::path(
    post,
    path = "/api/v1/secrets/{key}/rotate",
    params(("key" = String, Path, description = "Secret key")),
    responses(
        (status = 200, description = "Secret rotated", body = RotateSecretResponse),
        (status = 404, description = "Secret not found"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "secrets",
    security(("bearer_auth" = []))
)]
pub async fn rotate_secret(
    State(_server): State<RustCareServer>,
    Path(key): Path<String>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<RotateSecretResponse>>> {
    // TODO: Implement with SecretsManager
    // let secrets_manager = server.secrets_manager();
    // let new_version = secrets_manager.rotate_secret(&key).await
    //     .map_err(|e| match e {
    //         SecretsError::NotFound(_) => StatusCode::NOT_FOUND,
    //         _ => StatusCode::INTERNAL_SERVER_ERROR,
    //     })?;
    
    // Mock response for now
    let response = RotateSecretResponse {
        key,
        new_version: "v4".to_string(),
        rotated_at: chrono::Utc::now().to_rfc3339(),
    };
    
    Ok(Json(api_success(response)))
}

/// Check secrets service health
/// 
/// Verifies connectivity to all configured secret providers
#[utoipa::path(
    get,
    path = "/api/v1/secrets/health",
    responses(
        (status = 200, description = "Secrets health", body = HealthCheckResponse),
        (status = 503, description = "Service unavailable"),
        (status = 500, description = "Internal server error")
    ),
    tag = "secrets"
)]
pub async fn secrets_health_check(
    State(server): State<RustCareServer>,
) -> Result<Json<ApiResponse<HealthCheckResponse>>> {
    // Get secrets manager
    let secrets_manager = server.secrets_manager()
        .ok_or_else(|| ApiError::service_unavailable("Secrets manager not configured"))?;
    
    let status = secrets_manager.health_check()
        .await
        .map_err(|e| ApiError::internal(format!("Secrets health check failed: {}", e)))?;
    
    let response = HealthCheckResponse {
        healthy: status.healthy,
        message: status.message,
        latency_ms: status.latency_ms,
        last_check: status.last_check.to_rfc3339(),
    };
    
    Ok(Json(api_success(response)))
}
