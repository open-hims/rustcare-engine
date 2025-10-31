use axum::{
    routing::{get, post, put, delete, patch},
    Router,
};
use crate::{
    handlers::{health, auth, workflow, sync, permissions, geographic, compliance, organizations, devices, secrets, kms, healthcare, pharmacy, vendors, notifications}, // websocket temporarily disabled
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

/// Create secrets management routes
pub fn secrets_routes() -> Router<RustCareServer> {
    Router::new()
        // Health check
        .route("/secrets/health", get(secrets::secrets_health_check))
        // Secret CRUD
        .route("/secrets", get(secrets::list_secrets))
        .route("/secrets", post(secrets::create_secret))
        .route("/secrets/:key", get(secrets::get_secret))
        .route("/secrets/:key", put(secrets::update_secret))
        .route("/secrets/:key", delete(secrets::delete_secret))
        // Version management
        .route("/secrets/:key/versions", get(secrets::list_secret_versions))
        .route("/secrets/:key/versions/:version", get(secrets::get_secret_version))
        // Rotation
        .route("/secrets/:key/rotate", post(secrets::rotate_secret))
}

/// Create KMS (Key Management Service) routes
/// 
/// Provides cryptographic operations following enterprise KMS patterns:
/// - Envelope encryption (generate/decrypt data keys)
/// - Direct encryption/decryption (small data)
/// - Key rotation and re-encryption
/// - Key lifecycle management
pub fn kms_routes() -> Router<RustCareServer> {
    Router::new()
        // Testing endpoint (all operations on backend)
        .route("/kms/test", post(kms::test_kms_integration))
        
        // Data key operations (envelope encryption pattern)
        .route("/kms/datakey/generate", post(kms::generate_data_key))
        .route("/kms/datakey/decrypt", post(kms::decrypt_data_key))
        
        // Direct encryption operations (small data < 4KB)
        .route("/kms/encrypt", post(kms::encrypt))
        .route("/kms/decrypt", post(kms::decrypt))
        .route("/kms/re-encrypt", post(kms::re_encrypt))
        
        // Key management
        .route("/kms/keys", get(kms::list_keys))
        .route("/kms/keys", post(kms::create_key))
        .route("/kms/keys/:key_id", get(kms::describe_key))
        .route("/kms/keys/:key_id/enable", post(kms::enable_key))
        .route("/kms/keys/:key_id/disable", post(kms::disable_key))
        
        // Key rotation
        .route("/kms/keys/:key_id/rotation/enable", post(kms::enable_key_rotation))
        .route("/kms/keys/:key_id/rotation/disable", post(kms::disable_key_rotation))
        .route("/kms/keys/:key_id/rotation/status", get(kms::get_key_rotation_status))
        .route("/kms/keys/:key_id/rotate", post(kms::rotate_key))
        
        // Key lifecycle
        .route("/kms/keys/:key_id/schedule-deletion", post(kms::schedule_key_deletion))
        .route("/kms/keys/:key_id/cancel-deletion", post(kms::cancel_key_deletion))
}

/// Create healthcare routes
/// 
/// HIPAA-compliant medical records management:
/// - Medical records CRUD
/// - Provider management
/// - Service types (dynamic catalog)
/// - Appointments & Visits
/// - Clinical Orders
/// - Audit logging
/// - Access control
pub fn healthcare_routes() -> Router<RustCareServer> {
    Router::new()
        // Medical Records
        .route("/healthcare/medical-records", get(healthcare::list_medical_records))
        .route("/healthcare/medical-records", post(healthcare::create_medical_record))
        .route("/healthcare/medical-records/:record_id", get(healthcare::get_medical_record))
        .route("/healthcare/medical-records/:record_id", put(healthcare::update_medical_record))
        .route("/healthcare/medical-records/:record_id", delete(healthcare::delete_medical_record))
        .route("/healthcare/medical-records/:record_id/audit", get(healthcare::get_medical_record_audit))
        
        // Providers
        .route("/healthcare/providers", get(healthcare::list_providers))
        
        // Service Types (Dynamic Catalog)
        .route("/healthcare/service-types", get(healthcare::list_service_types))
        .route("/healthcare/service-types", post(healthcare::create_service_type))
        .route("/healthcare/service-types/:service_type_id", get(healthcare::get_service_type))
        .route("/healthcare/service-types/:service_type_id", put(healthcare::update_service_type))
        .route("/healthcare/service-types/:service_type_id", delete(healthcare::delete_service_type))
        
        // Appointments
        .route("/healthcare/appointments", get(healthcare::list_appointments))
        .route("/healthcare/appointments", post(healthcare::create_appointment))
        .route("/healthcare/appointments/:appointment_id/status", put(healthcare::update_appointment_status))
}

/// Create pharmacy routes
pub fn pharmacy_routes() -> Router<RustCareServer> {
    Router::new()
        // Pharmacies
        .route("/pharmacy/pharmacies", get(pharmacy::list_pharmacies))
        
        // Inventory
        .route("/pharmacy/inventory", get(pharmacy::list_inventory))
        
        // Prescriptions
        .route("/pharmacy/prescriptions", get(pharmacy::list_prescriptions))
}

/// Create vendor routes
pub fn vendor_routes() -> Router<RustCareServer> {
    Router::new()
        // Vendor Types
        .route("/vendors/types", get(vendors::list_vendor_types))
        
        // Vendors
        .route("/vendors", get(vendors::list_vendors))
        
        // Vendor Catalog
        .route("/vendors/:vendor_id/inventory", get(vendors::get_vendor_inventory))
        .route("/vendors/:vendor_id/services", get(vendors::get_vendor_services))
}

/// Create notification routes
/// 
/// Real-time notifications with audit logging:
/// - List and filter notifications
/// - Mark as read/unread (individual & bulk)
/// - Unread count
/// - Complete audit trail
pub fn notification_routes() -> Router<RustCareServer> {
    Router::new()
        // Notifications
        .route("/notifications", get(notifications::list_notifications))
        .route("/notifications", post(notifications::create_notification))
        .route("/notifications/:id", get(notifications::get_notification))
        .route("/notifications/:id/read", patch(notifications::mark_notification_read))
        .route("/notifications/bulk-read", patch(notifications::bulk_mark_read))
        .route("/notifications/unread/count", get(notifications::get_unread_count))
        
        // Audit Logs
        .route("/notifications/:id/audit-logs", get(notifications::list_audit_logs))
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
        .merge(secrets_routes())
        .merge(kms_routes())
        .merge(healthcare_routes())
        .merge(pharmacy_routes())
        .merge(vendor_routes())
        .merge(notification_routes())
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