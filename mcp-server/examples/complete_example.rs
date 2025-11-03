//! Complete example showing MCP tool with response types and render types

use axum::{extract::State, Json, Query};
use rustcare_server::server::RustCareServer;
use rustcare_server::middleware::AuthContext;
use rustcare_server::error::{ApiResponse, ApiError, api_success};
use rustcare_server::types::pagination::PaginationParams;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Pharmacy struct
#[derive(Debug, Serialize, Deserialize)]
pub struct Pharmacy {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub city: String,
}

/// List pharmacies query params
#[derive(Debug, Deserialize)]
pub struct ListPharmaciesParams {
    pub city: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Example: List pharmacies with table render type
#[mcp_macros::mcp_tool(
    name = "list_pharmacies",
    description = "List all pharmacies for the organization",
    category = "pharmacy",
    requires_permission = "pharmacy:read",
    sensitive = false,
    response_type = "Vec<Pharmacy>",
    render_type = "table"  // Render as markdown table for easy reading
)]
#[utoipa::path(
    get,
    path = "/api/v1/pharmacy/pharmacies",
    responses(
        (status = 200, description = "Pharmacies retrieved", body = Vec<Pharmacy>)
    ),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn list_pharmacies(
    State(_server): State<RustCareServer>,
    _auth: AuthContext,
    Query(_params): Query<ListPharmaciesParams>,
) -> Result<Json<ApiResponse<Vec<Pharmacy>>>, ApiError> {
    // Handler implementation
    // MCP tool will automatically render response as table
    Ok(Json(api_success(vec![])))
}

/// Example: Get patient with markdown render type
#[mcp_macros::mcp_tool(
    name = "get_patient",
    description = "Retrieve patient information by ID",
    category = "healthcare",
    requires_permission = "patient:read",
    sensitive = false,
    response_type = "Patient",
    render_type = "markdown"  // Render as formatted markdown
)]
pub async fn get_patient(
    _patient_id: Uuid,
    _auth: &AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Handler implementation
    // MCP tool will automatically render response as markdown
    Ok(Json(api_success(serde_json::json!({}))))
}

/// Example: Simple notification with text render type
#[mcp_macros::mcp_tool(
    name = "send_notification",
    description = "Send a notification",
    category = "notifications",
    requires_permission = "notification:write",
    sensitive = false,
    response_type = "NotificationStatus",
    render_type = "text"  // Simple text response
)]
pub async fn send_notification(
    _message: String,
    _auth: &AuthContext,
) -> Result<Json<ApiResponse<String>>, ApiError> {
    // Handler implementation
    // MCP tool will automatically render response as plain text
    Ok(Json(api_success("Notification sent".to_string())))
}

/// Example: Sensitive endpoint (excluded from public tool lists)
#[mcp_macros::mcp_tool(
    name = "rotate_secret",
    description = "Rotate a secret key",
    category = "secrets",
    requires_permission = "secrets:rotate",
    sensitive = true,  // Excluded from public tool discovery
    response_type = "SecretRotationStatus",
    render_type = "json"
)]
pub async fn rotate_secret(
    _secret_id: Uuid,
    _auth: &AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // This tool won't appear in public tool lists
    Ok(Json(api_success(serde_json::json!({"status": "rotated"}))))
}

