use axum::{
    extract::{Path, Query, State},
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
#[derive(Debug, Deserialize)]
pub struct ListNotificationsParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub is_read: Option<bool>,
    pub notification_type: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
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
    path = "/api/v1/notifications",
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
) -> Result<Json<ApiResponse<Vec<Notification>>>, ApiError> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    
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
        WHERE n.user_id = $1
            AND (n.expires_at IS NULL OR n.expires_at > NOW())
            AND ($2::bool IS NULL OR n.is_read = $2)
            AND ($3::text IS NULL OR n.notification_type = $3)
            AND ($4::text IS NULL OR n.priority = $4)
            AND ($5::text IS NULL OR n.category = $5)
        ORDER BY n.created_at DESC
        LIMIT $6
        OFFSET $7
        "#
    )
    .bind(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap_or_else(|_| Uuid::nil())) // TODO: Get from auth context
    .bind(params.is_read)
    .bind(params.notification_type.as_deref())
    .bind(params.priority.as_deref())
    .bind(params.category.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&app_state.db_pool)
    .await;
    
    match result {
        Ok(notifications) => Ok(Json(api_success(notifications))),
        Err(e) => Err(ApiError::internal(format!("Failed to fetch notifications: {}", e)))
    }
}

/// Get notification by ID
#[utoipa::path(
    get,
    path = "/api/v1/notifications/{id}",
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
        "#
    )
    .bind(id)
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
    path = "/api/v1/notifications",
    request_body = CreateNotificationRequest,
    responses(
        (status = 201, description = "Notification created", body = Notification)
    )
)]
pub async fn create_notification(
    State(app_state): State<RustCareServer>,
    Json(req): Json<CreateNotificationRequest>,
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
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
    .bind(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap_or_else(|_| Uuid::nil())) // TODO: Get from auth context
    .bind(req.user_id)
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
            // Log the creation in audit logs
            let _ = log_notification_action(
                &app_state,
                notification.id,
                "created".to_string(),
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
    path = "/api/v1/notifications/{id}/read",
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
) -> Result<Json<ApiResponse<Notification>>, ApiError> {
    let result = sqlx::query_as::<_, Notification>(
        r#"
        UPDATE notifications
        SET 
            is_read = $1,
            read_at = CASE WHEN $1 THEN NOW() ELSE NULL END,
            updated_at = NOW()
        WHERE id = $2
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
    .fetch_optional(&app_state.db_pool)
    .await;
    
    match result {
        Ok(Some(notification)) => {
            // Log the action in audit logs
            let _ = log_notification_action(
                &app_state,
                notification.id,
                if req.read { "read".to_string() } else { "unread".to_string() },
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
    path = "/api/v1/notifications/bulk-read",
    request_body = BulkMarkReadRequest,
    responses(
        (status = 200, description = "Notifications updated")
    )
)]
pub async fn bulk_mark_read(
    State(app_state): State<RustCareServer>,
    Json(req): Json<BulkMarkReadRequest>,
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
        WHERE id = ANY($2)
        "#
    )
    .bind(req.read)
    .bind(&req.notification_ids)
    .execute(&app_state.db_pool)
    .await;
    
    match result {
        Ok(_) => {
            // Log bulk actions
            for notification_id in req.notification_ids {
                let _ = log_notification_action(
                    &app_state,
                    notification_id,
                    if req.read { "bulk_read".to_string() } else { "bulk_unread".to_string() },
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
    path = "/api/v1/notifications/unread/count",
    responses(
        (status = 200, description = "Unread count")
    )
)]
pub async fn get_unread_count(
    State(app_state): State<RustCareServer>,
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
    .bind(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap_or_else(|_| Uuid::nil())) // TODO: Get from auth context
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
    path = "/api/v1/notifications/{id}/audit-logs",
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

/// Helper function to log notification actions to audit trail
async fn log_notification_action(
    app_state: &RustCareServer,
    notification_id: Uuid,
    action: String,
    action_details: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO notification_audit_logs (
            notification_id,
            organization_id,
            user_id,
            action,
            action_details
        ) VALUES ($1, $2, $3, $4, $5)
        "#
    )
    .bind(notification_id)
    .bind(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap_or_else(|_| Uuid::nil())) // TODO: Get from auth context
    .bind(Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap_or_else(|_| Uuid::nil())) // TODO: Get from auth context
    .bind(action)
    .bind(action_details)
    .execute(&app_state.db_pool)
    .await?;
    
    Ok(())
}

