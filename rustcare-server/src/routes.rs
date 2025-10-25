use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::{
    handlers::{health, auth, workflow, sync, permissions, geographic, compliance, organizations, devices}, // websocket temporarily disabled
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

/// Create permission management routes
pub fn permission_routes() -> Router<RustCareServer> {
    Router::new()
        // Resource management
        .route("/permissions/resources", get(permissions::list_resources))
        .route("/permissions/resources", post(permissions::create_resource))
        // Group management
        .route("/permissions/groups", get(permissions::list_groups))
        .route("/permissions/groups", post(permissions::create_group))
        // Role management
        .route("/permissions/roles", get(permissions::list_roles))
        .route("/permissions/roles", post(permissions::create_role))
        // User permissions
        .route("/permissions/users/:user_id", get(permissions::get_user_permissions))
        .route("/permissions/users/:user_id", put(permissions::assign_user_permissions))
        // Permission check endpoint (for Remix loaders)
        .route("/auth/check", post(permissions::check_permission))
}

/// Create geographic location management routes
pub fn geographic_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/geographic/regions", get(geographic::list_geographic_regions))
        .route("/geographic/regions", post(geographic::create_geographic_region))
        .route("/geographic/regions/:id", get(geographic::get_geographic_region))
        .route("/geographic/regions/:id", put(geographic::update_geographic_region))
        .route("/geographic/regions/:id", delete(geographic::delete_geographic_region))
        .route("/geographic/regions/:id/hierarchy", get(geographic::get_geographic_hierarchy))
        .route("/geographic/postal-codes/:postal_code/compliance", get(geographic::get_postal_code_compliance))
}

/// Create compliance framework management routes
pub fn compliance_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/compliance/frameworks", get(compliance::list_frameworks))
        .route("/compliance/frameworks", post(compliance::create_framework))
        .route("/compliance/frameworks/:id", get(compliance::get_framework))
        .route("/compliance/frameworks/:id", put(compliance::update_framework))
        .route("/compliance/frameworks/:id", delete(compliance::delete_framework))
        .route("/compliance/frameworks/:id/rules", get(compliance::list_framework_rules))
        .route("/compliance/rules", get(compliance::list_rules))
        .route("/compliance/rules", post(compliance::create_rule))
        .route("/compliance/rules/:id", get(compliance::get_rule))
        .route("/compliance/rules/:id", put(compliance::update_rule))
        .route("/compliance/rules/:id", delete(compliance::delete_rule))
        .route("/compliance/assignment/auto", post(compliance::auto_assign_compliance))
        .route("/compliance/entity/:entity_id", get(compliance::get_entity_compliance))
}

/// Create organization management routes
pub fn organization_routes() -> Router<RustCareServer> {
    Router::new()
        .route("/organizations", get(organizations::list_organizations))
        .route("/organizations", post(organizations::create_organization))
        .route("/role-templates", get(organizations::get_role_templates))
        .route("/organizations/:org_id/roles", get(organizations::list_organization_roles))
        .route("/organizations/:org_id/roles", post(organizations::create_organization_role))
        .route("/organizations/:org_id/employees", get(organizations::list_organization_employees))
        .route("/organizations/:org_id/employees/:employee_id/roles", post(organizations::assign_employee_role))
        .route("/organizations/:org_id/patients", get(organizations::list_organization_patients))
        .route("/organizations/:org_id/patients", post(organizations::create_organization_patient))
}

/// Create device management routes
pub fn device_routes() -> Router<RustCareServer> {
    Router::new()
        // Device CRUD
        .route("/devices", get(devices::list_devices))
        .route("/devices", post(devices::register_device))
        .route("/devices/:device_id", get(devices::get_device))
        .route("/devices/:device_id", put(devices::update_device))
        .route("/devices/:device_id", delete(devices::delete_device))
        
        // Connection management
        .route("/devices/:device_id/connect", post(devices::connect_device))
        .route("/devices/:device_id/disconnect", post(devices::disconnect_device))
        
        // Data operations
        .route("/devices/:device_id/data", get(devices::get_device_data_history))
        .route("/devices/:device_id/data/read", post(devices::read_device_data))
        
        // Command execution
        .route("/devices/:device_id/commands", get(devices::get_device_commands))
        .route("/devices/:device_id/commands", post(devices::send_device_command))
        
        // Configuration endpoints
        .route("/devices/types", get(devices::list_device_types))
        .route("/devices/connection-types", get(devices::list_connection_types))
        .route("/devices/formats", get(devices::list_data_formats))
}

/// Create API v1 routes
pub fn api_v1_routes() -> Router<RustCareServer> {
    Router::new()
        .nest("/auth", auth_routes())
        .nest("/workflow", workflow_routes())
        .merge(sync_routes())
        .merge(permission_routes())
        .merge(geographic_routes())
        .merge(compliance_routes())
        .merge(organization_routes())
        .merge(device_routes())
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