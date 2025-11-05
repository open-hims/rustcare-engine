use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};
use crate::utils::query_builder::PaginatedQuery;
use crate::types::pagination::PaginationParams;
use crate::middleware::AuthContext;
use crate::validation::RequestValidation;
use crate::services::AuditService;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Notification structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    
    // Notification content
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub priority: String,
    
    // Metadata
    pub category: Option<String>,
    pub action_url: Option<String>,
    pub action_label: Option<String>,
    
    // Tracking
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    
    // Rich content
    pub icon: Option<String>,
    pub image_url: Option<String>,
    
    // Expiration
    pub expires_at: Option<DateTime<Utc>>,
    
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Notification audit log structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct NotificationAuditLog {
    pub id: Uuid,
    pub notification_id: Uuid,
    pub organization_id: Option<Uuid>,
    
    // User who performed the action
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    
    // Action details
    pub action: String,
    pub action_details: Option<serde_json::Value>,
    
    // Timestamp
    pub created_at: DateTime<Utc>,
    
    // Context
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    
    // Metadata
    pub metadata: Option<serde_json::Value>,
}

/// Query parameters for listing notifications
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListNotificationsParams {
    pub is_read: Option<bool>,
    pub notification_type: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Create notification request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateNotificationRequest {
    pub user_id: Option<Uuid>,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub priority: String,
    pub category: Option<String>,
    pub action_url: Option<String>,
    pub action_label: Option<String>,
    pub icon: Option<String>,
    pub image_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl RequestValidation for CreateNotificationRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.title, "Title is required");
        validate_required!(self.message, "Message is required");
        validate_required!(self.notification_type, "Notification type is required");
        validate_required!(self.priority, "Priority is required");
        
        validate_length!(self.title, 1, 200, "Title must be between 1 and 200 characters");
        validate_length!(self.message, 1, 1000, "Message must be between 1 and 1000 characters");
        
        // Validate notification_type is one of valid values
        let valid_types = ["info", "success", "warning", "error", "system"];
        validate_field!(
            self.notification_type,
            valid_types.contains(&self.notification_type.as_str()),
            format!("Notification type must be one of: {}", valid_types.join(", "))
        );
        
        // Validate priority is one of valid values
        let valid_priorities = ["low", "normal", "high", "urgent"];
        validate_field!(
            self.priority,
            valid_priorities.contains(&self.priority.as_str()),
            format!("Priority must be one of: {}", valid_priorities.join(", "))
        );
        
        // Validate action_url format if provided
        if let Some(ref url) = self.action_url {
            validate_field!(
                url,
                url.starts_with("http://") || url.starts_with("https://") || url.starts_with("/"),
                "Action URL must be a valid HTTP/HTTPS URL or relative path"
            );
        }
        
        Ok(())
    }
}

/// Mark notification as read request
#[derive(Debug, Deserialize, ToSchema)]
pub struct MarkReadRequest {
    pub read: bool,
}

/// Bulk mark as read request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkMarkReadRequest {
    pub notification_ids: Vec<Uuid>,
    pub read: bool,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// List notifications for the current user
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::NOTIFICATIONS,
    responses(
        (status = 200, description = "List of notifications", body = Vec<Notification>)
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Number of results to return"),
        ("offset" = Option<i64>, Query, description = "Number of results to skip"),
        ("is_read" = Option<bool>, Query, description = "Filter by read status"),
        ("notification_type" = Option<String>, Query, description = "Filter by type"),
        ("priority" = Option<String>, Query, description = "Filter by priority"),
        ("category" = Option<String>, Query, description = "Filter by category"),
    )
)]
pub async fn list_notifications(
    Query(params): Query<ListNotificationsParams>,
    State(app_state): State<RustCareServer>,
    auth: AuthContext, // Using new AuthContext extractor
) -> Result<Json<ApiResponse<Vec<Notification>>>, ApiError> {
    // Use PaginatedQuery utility
    let mut query = PaginatedQuery::new(
        r#"
        SELECT 
            n.id,
            n.organization_id,
            n.user_id,
            n.title,
            n.message,
            n.notification_type,
            n.priority,
            n.category,
            n.action_url,
            n.action_label,
            n.is_read,
            n.read_at,
            n.icon,
            n.image_url,
            n.expires_at,
            n.created_at,
            n.updated_at
        FROM notifications n
        "#
    );
    
    query
        .add_base_filter("n.user_id", auth.user_id)
        .query_builder()
        .push(" AND (n.expires_at IS NULL OR n.expires_at > NOW())");
    
    query
        .filter_eq("n.is_read", params.is_read)
        .filter_eq("n.notification_type", params.notification_type.as_ref())
        .filter_eq("n.priority", params.priority.as_ref())
        .filter_eq("n.category", params.category.as_ref())
        .order_by("n.created_at", "DESC")
        .paginate(
            params.pagination.page,
            params.pagination.page_size
        );
    
    let notifications: Vec<Notification> = query.build_query_as().fetch_all(&app_state.db_pool).await?;
    
    // Get total count for pagination metadata
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM notifications
        WHERE user_id = $1
            AND (expires_at IS NULL OR expires_at > NOW())
            AND ($2::bool IS NULL OR is_read = $2)
            AND ($3::text IS NULL OR notification_type = $3)
            AND ($4::text IS NULL OR priority = $4)
            AND ($5::text IS NULL OR category = $5)
        "#
    )
    .bind(auth.user_id)
    .bind(params.is_read)
    .bind(params.notification_type.as_deref())
    .bind(params.priority.as_deref())
    .bind(params.category.as_deref())
    .fetch_one(&app_state.db_pool)
    .await?;
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(notifications, metadata)))
}

/// Get notification by ID
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::NOTIFICATION_BY_ID,
    responses(
        (status = 200, description = "Notification details", body = Notification)
    ),
    params(
        ("id" = Uuid, Path, description = "Notification ID")
    )
)]
pub async fn get_notification(
    Path(id): Path<Uuid>,
    State(app_state): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
    let result = sqlx::query_as::<_, Notification>(
        r#"
        SELECT 
            n.id,
            n.organization_id,
            n.user_id,
            n.title,
            n.message,
            n.notification_type,
            n.priority,
            n.category,
            n.action_url,
            n.action_label,
            n.is_read,
            n.read_at,
            n.icon,
            n.image_url,
            n.expires_at,
            n.created_at,
            n.updated_at
        FROM notifications n
        WHERE n.id = $1
          AND n.user_id = $2
        "#
    )
    .bind(id)
    .bind(auth.user_id) // Ensure user can only access their own notifications
    .fetch_optional(&app_state.db_pool)
    .await;
    
    match result {
        Ok(Some(notification)) => Ok(Json(api_success(notification))),
        Ok(None) => Err(ApiError::not_found("Notification not found")),
        Err(e) => Err(ApiError::internal(format!("Failed to fetch notification: {}", e)))
    }
}

/// Create a new notification
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::NOTIFICATIONS,
    request_body = CreateNotificationRequest,
    responses(
        (status = 201, description = "Notification created", body = Notification)
    )
)]
pub async fn create_notification(
    State(app_state): State<RustCareServer>,
    Json(req): Json<CreateNotificationRequest>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
    // Validate request
    req.validate()?;
    
    let result = sqlx::query_as::<_, Notification>(
        r#"
        INSERT INTO notifications (
            organization_id,
            user_id,
            title,
            message,
            notification_type,
            priority,
            category,
            action_url,
            action_label,
            icon,
            image_url,
            expires_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING 
            id,
            organization_id,
            user_id,
            title,
            message,
            notification_type,
            priority,
            category,
            action_url,
            action_label,
            is_read,
            read_at,
            icon,
            image_url,
            expires_at,
            created_at,
            updated_at
        "#
    )
    .bind(auth.organization_id) // Use actual auth context
    .bind(req.user_id.or(Some(auth.user_id))) // Use request user_id or default to auth user_id
    .bind(&req.title)
    .bind(&req.message)
    .bind(&req.notification_type)
    .bind(&req.priority)
    .bind(req.category.as_deref())
    .bind(req.action_url.as_deref())
    .bind(req.action_label.as_deref())
    .bind(req.icon.as_deref())
    .bind(req.image_url.as_deref())
    .bind(req.expires_at)
    .fetch_one(&app_state.db_pool)
    .await;
    
    match result {
        Ok(notification) => {
            // Log the creation using AuditService
            let audit_service = AuditService::new(app_state.db_pool.clone());
            let _ = audit_service.log_notification_action(
                &auth,
                notification.id,
                "created",
                None,
            ).await;
            
            Ok(Json(api_success(notification)))
        },
        Err(e) => Err(ApiError::internal(format!("Failed to create notification: {}", e)))
    }
}

/// Mark notification as read/unread
#[utoipa::path(
    patch,
    path = crate::routes::paths::api_v1::NOTIFICATION_MARK_READ,
    request_body = MarkReadRequest,
    responses(
        (status = 200, description = "Notification updated", body = Notification)
    ),
    params(
        ("id" = Uuid, Path, description = "Notification ID")
    )
)]
pub async fn mark_notification_read(
    Path(id): Path<Uuid>,
    State(app_state): State<RustCareServer>,
    Json(req): Json<MarkReadRequest>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
    let result = sqlx::query_as::<_, Notification>(
        r#"
        UPDATE notifications
        SET 
            is_read = $1,
            read_at = CASE WHEN $1 THEN NOW() ELSE NULL END,
            updated_at = NOW()
        WHERE id = $2 AND user_id = $3
        RETURNING 
            id,
            organization_id,
            user_id,
            title,
            message,
            notification_type,
            priority,
            category,
            action_url,
            action_label,
            is_read,
            read_at,
            icon,
            image_url,
            expires_at,
            created_at,
            updated_at
        "#
    )
    .bind(req.read)
    .bind(id)
    .bind(auth.user_id) // Ensure user can only update their own notifications
    .fetch_optional(&app_state.db_pool)
    .await;
    
    match result {
        Ok(Some(notification)) => {
            // Log the action using AuditService
            let audit_service = AuditService::new(app_state.db_pool.clone());
            let action = if req.read { "read" } else { "unread" };
            let _ = audit_service.log_notification_action(
                &auth,
                notification.id,
                action,
                Some(serde_json::json!({"read": req.read})),
            ).await;
            
            Ok(Json(api_success(notification)))
        },
        Ok(None) => Err(ApiError::not_found("Notification not found")),
        Err(e) => Err(ApiError::internal(format!("Failed to update notification: {}", e)))
    }
}

/// Bulk mark notifications as read/unread
#[utoipa::path(
    patch,
    path = crate::routes::paths::api_v1::NOTIFICATION_BULK_READ,
    request_body = BulkMarkReadRequest,
    responses(
        (status = 200, description = "Notifications updated")
    )
)]
pub async fn bulk_mark_read(
    State(app_state): State<RustCareServer>,
    Json(req): Json<BulkMarkReadRequest>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    if req.notification_ids.is_empty() {
        return Ok(Json(api_success(())));
    }
    
    let result = sqlx::query(
        r#"
        UPDATE notifications
        SET 
            is_read = $1,
            read_at = CASE WHEN $1 THEN NOW() ELSE NULL END,
            updated_at = NOW()
        WHERE id = ANY($2) AND user_id = $3
        "#
    )
    .bind(req.read)
    .bind(&req.notification_ids)
    .bind(auth.user_id) // Ensure user can only update their own notifications
    .execute(&app_state.db_pool)
    .await;
    
    match result {
        Ok(_) => {
            // Log bulk actions using AuditService
            let audit_service = AuditService::new(app_state.db_pool.clone());
            let action = if req.read { "bulk_read" } else { "bulk_unread" };
            for notification_id in req.notification_ids {
                let _ = audit_service.log_notification_action(
                    &auth,
                    notification_id,
                    action,
                    None,
                ).await;
            }
            
            Ok(Json(api_success(())))
        },
        Err(e) => Err(ApiError::internal(format!("Failed to bulk update notifications: {}", e)))
    }
}

/// Get unread notification count
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::NOTIFICATION_UNREAD_COUNT,
    responses(
        (status = 200, description = "Unread count")
    )
)]
pub async fn get_unread_count(
    State(app_state): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<i64>>, ApiError> {
    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM notifications
        WHERE user_id = $1
            AND is_read = false
            AND (expires_at IS NULL OR expires_at > NOW())
        "#
    )
    .bind(auth.user_id) // Use actual auth context
    .fetch_one(&app_state.db_pool)
    .await;
    
    match result {
        Ok(count) => Ok(Json(api_success(count))),
        Err(e) => Err(ApiError::internal(format!("Failed to fetch unread count: {}", e)))
    }
}

/// List notification audit logs
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::NOTIFICATION_AUDIT_LOGS,
    responses(
        (status = 200, description = "Audit logs", body = Vec<NotificationAuditLog>)
    ),
    params(
        ("id" = Uuid, Path, description = "Notification ID"),
        ("limit" = Option<i64>, Query, description = "Number of results to return"),
        ("offset" = Option<i64>, Query, description = "Number of results to skip"),
    )
)]
pub async fn list_audit_logs(
    Path(id): Path<Uuid>,
    Query(params): Query<ListNotificationsParams>,
    State(app_state): State<RustCareServer>,
) -> Result<Json<ApiResponse<Vec<NotificationAuditLog>>>, ApiError> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    
    let result = sqlx::query_as::<_, NotificationAuditLog>(
        r#"
        SELECT 
            nal.id,
            nal.notification_id,
            nal.organization_id,
            nal.user_id,
            nal.user_email,
            nal.action,
            nal.action_details,
            nal.created_at,
            nal.ip_address,
            nal.user_agent,
            nal.metadata
        FROM notification_audit_logs nal
        WHERE nal.notification_id = $1
        ORDER BY nal.created_at DESC
        LIMIT $2
        OFFSET $3
        "#
    )
    .bind(id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&app_state.db_pool)
    .await;
    
    match result {
        Ok(logs) => Ok(Json(api_success(logs))),
        Err(e) => Err(ApiError::internal(format!("Failed to fetch audit logs: {}", e)))
    }
}

// Note: Audit logging is now handled by AuditService
// The old log_notification_action function has been removed
// All audit logging now uses AuditService::log_notification_action()

