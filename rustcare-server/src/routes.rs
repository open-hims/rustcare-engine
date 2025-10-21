use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::{
    handlers::{health, auth, workflow, sync}, // websocket temporarily disabled
    server::RustCareServer,
    openapi,
};

/// Create health check routes
pub fn health_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/version", get(health::version_info))
        .route("/status", get(health::system_status))
}

/// Create authentication routes
pub fn auth_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/oauth/authorize", post(auth::oauth_authorize))
        .route("/token/validate", post(auth::validate_token))
}

/// Create workflow routes
pub fn workflow_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/workflows", get(workflow::list_workflows))
        .route("/workflows/:id", get(workflow::get_workflow))
        .route("/workflows/execute", post(workflow::execute_workflow))
        .route("/executions/:id/status", get(workflow::get_execution_status))
        .route("/executions/:id/cancel", delete(workflow::cancel_execution))
}

/// Create sync routes for offline-first synchronization
pub fn sync_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/sync/pull", post(sync::pull))
        .route("/sync/push", post(sync::push))
}

/// Create API v1 routes
pub fn api_v1_routes() -> Router<RustCareServer> {
    Router::new()
        .nest("/auth", auth_routes())
        .nest("/workflow", workflow_routes())
        .merge(sync_routes())
        // TODO: Add more API routes here:
        // .nest("/plugins", plugin_routes())
        // .nest("/audit", audit_routes())
        // .nest("/patients", patient_routes())
        // .nest("/staff", staff_routes())
        // .nest("/analytics", analytics_routes())
}

/// Create WebSocket routes (temporarily disabled)
pub fn websocket_routes() -> Router<RustCareServer> {
    Router::new()
        // TODO: Re-enable WebSocket routes after fixing compilation issues
        // .route("/ws", get(websocket::websocket_handler))
        // .route("/ws/health", get(websocket::websocket_handler))
}

/// Postman collection handler
pub async fn postman_collection() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(openapi::generate_postman_collection())
}

/// Create all application routes
pub fn create_routes() -> Router<RustCareServer> {
    Router::new()
        // Health check routes (no authentication required)
        .merge(health_routes())
        // API documentation routes
        .merge(openapi::create_docs_routes())
        // Postman collection endpoint
        .route("/postman-collection.json", get(postman_collection))
        // API v1 routes (authentication required)
        .nest("/api/v1", api_v1_routes())
        // TODO: Add API versioning:
        // .nest("/api/v2", api_v2_routes())
}