pub mod paths;

use axum::{
    routing::{get, post, put, delete, patch},
    Router,
};
use crate::{
    handlers::{health, auth, workflow, sync, permissions, geographic, compliance, organizations, devices, secrets, kms, healthcare, pharmacy, vendors, notifications, onboarding, ui_components, plugins, forms}, // websocket temporarily disabled
    server::RustCareServer,
    openapi,
};

/// Create health check routes
pub fn health_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::health::HEALTH, get(health::health_check))
        .route(paths::health::VERSION, get(health::version_info))
        .route(paths::health::STATUS, get(health::system_status))
}

/// Create authentication routes
pub fn auth_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::auth::LOGIN, post(auth::login))
        .route(paths::auth::LOGOUT, post(auth::logout))
        .route(paths::auth::OAUTH_AUTHORIZE, post(auth::oauth_authorize))
        .route(paths::auth::TOKEN_VALIDATE, post(auth::validate_token))
}

/// Create workflow routes
pub fn workflow_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::workflow::WORKFLOWS, get(workflow::list_workflows))
        .route(paths::workflow::WORKFLOW_BY_ID, get(workflow::get_workflow))
        .route(paths::workflow::EXECUTE, post(workflow::execute_workflow))
        .route(paths::workflow::EXECUTION_STATUS, get(workflow::get_execution_status))
        .route(paths::workflow::EXECUTION_CANCEL, delete(workflow::cancel_execution))
}

/// Create sync routes for offline-first synchronization
pub fn sync_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::sync::PULL, post(sync::pull))
        .route(paths::sync::PUSH, post(sync::push))
}

/// Create permission management routes
pub fn permission_routes() -> Router<RustCareServer> {
    Router::new()
        // Resource management
        .route(paths::permissions::RESOURCES, get(permissions::list_resources))
        .route(paths::permissions::RESOURCES, post(permissions::create_resource))
        // Group management
        .route(paths::permissions::GROUPS, get(permissions::list_groups))
        .route(paths::permissions::GROUPS, post(permissions::create_group))
        // Role management
        .route(paths::permissions::ROLES, get(permissions::list_roles))
        .route(paths::permissions::ROLES, post(permissions::create_role))
        // User permissions
        .route(paths::permissions::USER_PERMISSIONS, get(permissions::get_user_permissions))
        .route(paths::permissions::USER_PERMISSIONS, put(permissions::assign_user_permissions))
        // Permission check endpoint (for Remix loaders)
        .route(paths::auth::CHECK, post(permissions::check_permission))
}

/// Create geographic location management routes
pub fn geographic_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::geographic::REGIONS, get(geographic::list_geographic_regions))
        .route(paths::geographic::REGIONS, post(geographic::create_geographic_region))
        .route(paths::geographic::REGION_BY_ID, get(geographic::get_geographic_region))
        .route(paths::geographic::REGION_BY_ID, put(geographic::update_geographic_region))
        .route(paths::geographic::REGION_BY_ID, delete(geographic::delete_geographic_region))
        .route(paths::geographic::REGION_HIERARCHY, get(geographic::get_geographic_hierarchy))
        .route(paths::geographic::POSTAL_CODE_COMPLIANCE, get(geographic::get_postal_code_compliance))
}

/// Create compliance framework management routes
pub fn compliance_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::compliance::FRAMEWORKS, get(compliance::list_frameworks))
        .route(paths::compliance::FRAMEWORKS, post(compliance::create_framework))
        .route(paths::compliance::FRAMEWORK_BY_ID, get(compliance::get_framework))
        .route(paths::compliance::FRAMEWORK_BY_ID, put(compliance::update_framework))
        .route(paths::compliance::FRAMEWORK_BY_ID, delete(compliance::delete_framework))
        .route(paths::compliance::FRAMEWORK_RULES, get(compliance::list_framework_rules))
        .route(paths::compliance::RULES, get(compliance::list_rules))
        .route(paths::compliance::RULES, post(compliance::create_rule))
        .route(paths::compliance::RULE_BY_ID, get(compliance::get_rule))
        .route(paths::compliance::RULE_BY_ID, put(compliance::update_rule))
        .route(paths::compliance::RULE_BY_ID, delete(compliance::delete_rule))
        .route(paths::compliance::AUTO_ASSIGN, post(compliance::auto_assign_compliance))
        .route(paths::compliance::ENTITY_COMPLIANCE, get(compliance::get_entity_compliance))
}

/// Create organization management routes
pub fn organization_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::organizations::ORGANIZATIONS, get(organizations::list_organizations))
        .route(paths::organizations::ORGANIZATIONS, post(organizations::create_organization))
        .route(paths::organizations::ROLE_TEMPLATES, get(organizations::get_role_templates))
        .route(paths::organizations::ORGANIZATION_ROLES, get(organizations::list_organization_roles))
        .route(paths::organizations::ORGANIZATION_ROLES, post(organizations::create_organization_role))
        .route(paths::organizations::ORGANIZATION_EMPLOYEES, get(organizations::list_organization_employees))
        .route(paths::organizations::EMPLOYEE_ROLES, post(organizations::assign_employee_role))
        .route(paths::organizations::ORGANIZATION_PATIENTS, get(organizations::list_organization_patients))
        .route(paths::organizations::ORGANIZATION_PATIENTS, post(organizations::create_organization_patient))
}

/// Create device management routes
pub fn device_routes() -> Router<RustCareServer> {
    Router::new()
        // Device CRUD
        .route(paths::devices::DEVICES, get(devices::list_devices))
        .route(paths::devices::DEVICES, post(devices::register_device))
        .route(paths::devices::DEVICE_BY_ID, get(devices::get_device))
        .route(paths::devices::DEVICE_BY_ID, put(devices::update_device))
        .route(paths::devices::DEVICE_BY_ID, delete(devices::delete_device))
        
        // Connection management
        .route(paths::devices::CONNECT, post(devices::connect_device))
        .route(paths::devices::DISCONNECT, post(devices::disconnect_device))
        
        // Data operations
        .route(paths::devices::DATA, get(devices::get_device_data_history))
        .route(paths::devices::DATA_READ, post(devices::read_device_data))
        
        // Command execution
        .route(paths::devices::COMMANDS, get(devices::get_device_commands))
        .route(paths::devices::COMMANDS, post(devices::send_device_command))
        
        // Configuration endpoints
        .route(paths::devices::TYPES, get(devices::list_device_types))
        .route(paths::devices::CONNECTION_TYPES, get(devices::list_connection_types))
        .route(paths::devices::FORMATS, get(devices::list_data_formats))
}

/// Create secrets management routes
pub fn secrets_routes() -> Router<RustCareServer> {
    Router::new()
        // Health check
        .route(paths::secrets::HEALTH, get(secrets::secrets_health_check))
        // Secret CRUD
        .route(paths::secrets::SECRETS, get(secrets::list_secrets))
        .route(paths::secrets::SECRETS, post(secrets::create_secret))
        .route(paths::secrets::SECRET_BY_KEY, get(secrets::get_secret))
        .route(paths::secrets::SECRET_BY_KEY, put(secrets::update_secret))
        .route(paths::secrets::SECRET_BY_KEY, delete(secrets::delete_secret))
        // Version management
        .route(paths::secrets::SECRET_VERSIONS, get(secrets::list_secret_versions))
        .route(paths::secrets::SECRET_VERSION, get(secrets::get_secret_version))
        // Rotation
        .route(paths::secrets::ROTATE, post(secrets::rotate_secret))
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
        .route(paths::kms::TEST, post(kms::test_kms_integration))
        
        // Data key operations (envelope encryption pattern)
        .route(paths::kms::DATAKEY_GENERATE, post(kms::generate_data_key))
        .route(paths::kms::DATAKEY_DECRYPT, post(kms::decrypt_data_key))
        
        // Direct encryption operations (small data < 4KB)
        .route(paths::kms::ENCRYPT, post(kms::encrypt))
        .route(paths::kms::DECRYPT, post(kms::decrypt))
        .route(paths::kms::RE_ENCRYPT, post(kms::re_encrypt))
        
        // Key management
        .route(paths::kms::KEYS, get(kms::list_keys))
        .route(paths::kms::KEYS, post(kms::create_key))
        .route(paths::kms::KEY_BY_ID, get(kms::describe_key))
        .route(paths::kms::KEY_ENABLE, post(kms::enable_key))
        .route(paths::kms::KEY_DISABLE, post(kms::disable_key))
        
        // Key rotation
        .route(paths::kms::KEY_ROTATION_ENABLE, post(kms::enable_key_rotation))
        .route(paths::kms::KEY_ROTATION_DISABLE, post(kms::disable_key_rotation))
        .route(paths::kms::KEY_ROTATION_STATUS, get(kms::get_key_rotation_status))
        .route(paths::kms::KEY_ROTATE, post(kms::rotate_key))
        
        // Key lifecycle
        .route(paths::kms::KEY_SCHEDULE_DELETION, post(kms::schedule_key_deletion))
        .route(paths::kms::KEY_CANCEL_DELETION, post(kms::cancel_key_deletion))
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
        .route(paths::healthcare::MEDICAL_RECORDS, get(healthcare::list_medical_records))
        .route(paths::healthcare::MEDICAL_RECORDS, post(healthcare::create_medical_record))
        .route(paths::healthcare::MEDICAL_RECORD_BY_ID, get(healthcare::get_medical_record))
        .route(paths::healthcare::MEDICAL_RECORD_BY_ID, put(healthcare::update_medical_record))
        .route(paths::healthcare::MEDICAL_RECORD_BY_ID, delete(healthcare::delete_medical_record))
        .route(paths::healthcare::MEDICAL_RECORD_AUDIT, get(healthcare::get_medical_record_audit))
        
        // Providers
        .route(paths::healthcare::PROVIDERS, get(healthcare::list_providers))
        
        // Service Types (Dynamic Catalog)
        .route(paths::healthcare::SERVICE_TYPES, get(healthcare::list_service_types))
        .route(paths::healthcare::SERVICE_TYPES, post(healthcare::create_service_type))
        .route(paths::healthcare::SERVICE_TYPE_BY_ID, get(healthcare::get_service_type))
        .route(paths::healthcare::SERVICE_TYPE_BY_ID, put(healthcare::update_service_type))
        .route(paths::healthcare::SERVICE_TYPE_BY_ID, delete(healthcare::delete_service_type))
        
        // Appointments
        .route(paths::healthcare::APPOINTMENTS, get(healthcare::list_appointments))
        .route(paths::healthcare::APPOINTMENTS, post(healthcare::create_appointment))
        .route(paths::healthcare::APPOINTMENT_STATUS, put(healthcare::update_appointment_status))
}

/// Create pharmacy routes
pub fn pharmacy_routes() -> Router<RustCareServer> {
    Router::new()
        // Pharmacies CRUD
        .route(paths::pharmacy::PHARMACIES, get(pharmacy::list_pharmacies))
        .route(paths::pharmacy::PHARMACIES, post(pharmacy::create_pharmacy))
        .route(paths::pharmacy::PHARMACY_BY_ID, get(pharmacy::get_pharmacy))
        .route(paths::pharmacy::PHARMACY_BY_ID, put(pharmacy::update_pharmacy))
        .route(paths::pharmacy::PHARMACY_BY_ID, delete(pharmacy::delete_pharmacy))
        
        // Inventory
        .route(paths::pharmacy::INVENTORY, get(pharmacy::list_inventory))
        
        // Prescriptions
        .route(paths::pharmacy::PRESCRIPTIONS, get(pharmacy::list_prescriptions))
}

/// Create vendor routes
pub fn vendor_routes() -> Router<RustCareServer> {
    Router::new()
        // Vendor Types
        .route(paths::vendors::TYPES, get(vendors::list_vendor_types))
        
        // Vendors
        .route(paths::vendors::VENDORS, get(vendors::list_vendors))
        
        // Vendor Catalog
        .route(paths::vendors::VENDOR_INVENTORY, get(vendors::get_vendor_inventory))
        .route(paths::vendors::VENDOR_SERVICES, get(vendors::get_vendor_services))
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
        .route(paths::notifications::NOTIFICATIONS, get(notifications::list_notifications))
        .route(paths::notifications::NOTIFICATIONS, post(notifications::create_notification))
        .route(paths::notifications::NOTIFICATION_BY_ID, get(notifications::get_notification))
        .route(paths::notifications::MARK_READ, patch(notifications::mark_notification_read))
        .route(paths::notifications::BULK_READ, patch(notifications::bulk_mark_read))
        .route(paths::notifications::UNREAD_COUNT, get(notifications::get_unread_count))
        
        // Audit Logs
        .route(paths::notifications::AUDIT_LOGS, get(notifications::list_audit_logs))
}

/// Create onboarding routes
/// 
/// Hospital onboarding and user creation:
/// - Create organization users with credentials
/// - Send welcome emails with temporary passwords
/// - Resend credentials
pub fn onboarding_routes() -> Router<RustCareServer> {
    Router::new()
        // User management
        .route(paths::onboarding::ORGANIZATION_USERS, get(onboarding::list_organization_users))
        .route(paths::onboarding::ORGANIZATION_USERS, post(onboarding::create_organization_user))
        .route(paths::onboarding::RESEND_CREDENTIALS, post(onboarding::resend_user_credentials))
        // Email verification
        .route(paths::onboarding::EMAIL_VERIFY, post(onboarding::verify_email_config))
}

/// Create UI components routes
/// 
/// Auto-registration of UI components and actions discovered from decorators:
/// - Register components (pages, forms, modals, etc.)
/// - Register actions (buttons, links, etc.)
/// - List registered components
pub fn ui_components_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::ui_components::REGISTER_COMPONENT, post(ui_components::register_component))
        .route(paths::ui_components::REGISTER_ACTION, post(ui_components::register_component_action))
        .route(paths::ui_components::COMPONENTS, get(ui_components::list_components))
}

/// Create plugin management routes
/// 
/// Plugin lifecycle and execution:
/// - List installed plugins
/// - Install new plugins
/// - Load/unload plugins
/// - Execute plugin functions
/// - Plugin health checks
pub fn plugin_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::api_v1::PLUGINS, get(plugins::list_plugins))
        .route(paths::api_v1::PLUGINS, post(plugins::install_plugin))
        .route(paths::api_v1::PLUGIN_BY_ID, get(plugins::get_plugin))
        .route(paths::api_v1::PLUGIN_BY_ID, delete(plugins::uninstall_plugin))
        .route(paths::api_v1::PLUGIN_LOAD, post(plugins::load_plugin))
        .route(paths::api_v1::PLUGIN_EXECUTE, post(plugins::execute_plugin))
        .route(paths::api_v1::PLUGIN_STOP, post(plugins::stop_plugin))
        .route(paths::api_v1::PLUGIN_HEALTH, get(plugins::plugin_health))
}

/// Create form builder routes
/// 
/// Dynamic form definitions and submissions:
/// - Create/update/delete form definitions
/// - List forms by module/entity type
/// - Submit form data
/// - Manage form submissions
pub fn form_routes() -> Router<RustCareServer> {
    Router::new()
        .route(paths::api_v1::FORMS, get(forms::list_form_definitions))
        .route(paths::api_v1::FORMS, post(forms::create_form_definition))
        .route(paths::api_v1::FORM_BY_ID, get(forms::get_form_definition))
        .route(paths::api_v1::FORM_BY_ID, put(forms::update_form_definition))
        .route(paths::api_v1::FORM_BY_ID, delete(forms::delete_form_definition))
        .route(paths::api_v1::FORM_BY_SLUG, get(forms::get_form_definition_by_slug))
        .route(paths::api_v1::FORM_SUBMIT, post(forms::submit_form))
        .route(paths::api_v1::FORM_SUBMISSIONS, get(forms::list_form_submissions))
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
        .merge(onboarding_routes())
        .merge(ui_components_routes())
        .merge(plugin_routes())
        .merge(form_routes())
        // TODO: Add more API routes here:
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
        .nest(paths::API_V1, api_v1_routes())
        // TODO: Add API versioning:
        // .nest("/api/v2", api_v2_routes())
}