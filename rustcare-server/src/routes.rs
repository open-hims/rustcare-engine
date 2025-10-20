use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::{
    handlers::{health, auth, workflow, websocket},
    server::RustCareServer,
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

/// Create API v1 routes
pub fn api_v1_routes() -> Router<RustCareServer> {
    Router::new()
        .nest("/auth", auth_routes())
        .nest("/workflow", workflow_routes())
        // TODO: Add more API routes here:
        // .nest("/plugins", plugin_routes())
        // .nest("/audit", audit_routes())
        // .nest("/patients", patient_routes())
        // .nest("/staff", staff_routes())
        // .nest("/analytics", analytics_routes())
}

/// Create WebSocket routes
pub fn websocket_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/ws", get(websocket::websocket_handler))
        .route("/ws/health", get(websocket::websocket_handler))
}

/// Create all application routes
pub fn create_routes() -> Router<RustCareServer> {
    Router::new()
        // Health check routes (no authentication required)
        .merge(health_routes())
        // WebSocket routes
        .merge(websocket_routes())
        // API v1 routes (authentication required)
        .nest("/api/v1", api_v1_routes())
        // TODO: Add API versioning:
        // .nest("/api/v2", api_v2_routes())
}