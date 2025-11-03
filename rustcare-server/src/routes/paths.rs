//! Centralized API route path constants
//!
//! This module provides constants for all API routes to ensure consistency
//! between runtime route definitions and OpenAPI documentation.
//!
//! **Important**: While utoipa `#[path(...)]` attributes require string literals
//! and cannot use these constants directly, these constants should be used for
//! all runtime route definitions. The paths in utoipa attributes should match
//! these constants exactly.
//!
//! # Usage
//!
//! ```rust
//! use crate::routes::paths;
//!
//! // In routes.rs:
//! Router::new()
//!     .route(paths::API_V1_PHARMACY_PHARMACIES, get(handler))
//!
//! // In handlers (utoipa still needs string literal):
//! #[utoipa::path(
//!     get,
//!     path = "/api/v1/pharmacy/pharmacies",  // Must match paths::API_V1_PHARMACY_PHARMACIES
//!     ...
//! )]
//! ```

/// API base paths
pub const API_V1: &str = "/api/v1";

/// Health check endpoints
pub mod health {
    use super::API_V1;
    pub const HEALTH: &str = "/health";
    pub const VERSION: &str = "/version";
    pub const STATUS: &str = "/status";
}

/// Authentication endpoints
pub mod auth {
    use super::API_V1;
    pub const LOGIN: &str = "/login";
    pub const LOGOUT: &str = "/logout";
    pub const OAUTH_AUTHORIZE: &str = "/oauth/authorize";
    pub const TOKEN_VALIDATE: &str = "/token/validate";
    pub const CHECK: &str = "/auth/check";
}

/// Workflow endpoints
pub mod workflow {
    use super::API_V1;
    pub const WORKFLOWS: &str = "/workflows";
    pub const WORKFLOW_BY_ID: &str = "/workflows/:id";
    pub const EXECUTE: &str = "/workflows/execute";
    pub const EXECUTION_STATUS: &str = "/executions/:id/status";
    pub const EXECUTION_CANCEL: &str = "/executions/:id/cancel";
}

/// Sync endpoints
pub mod sync {
    use super::API_V1;
    pub const PULL: &str = "/sync/pull";
    pub const PUSH: &str = "/sync/push";
}

/// Permission management endpoints
pub mod permissions {
    use super::API_V1;
    pub const RESOURCES: &str = "/permissions/resources";
    pub const GROUPS: &str = "/permissions/groups";
    pub const ROLES: &str = "/permissions/roles";
    pub const USER_PERMISSIONS: &str = "/permissions/users/:user_id";
}

/// Geographic location endpoints
pub mod geographic {
    use super::API_V1;
    pub const REGIONS: &str = "/geographic/regions";
    pub const REGION_BY_ID: &str = "/geographic/regions/:id";
    pub const REGION_HIERARCHY: &str = "/geographic/regions/:id/hierarchy";
    pub const POSTAL_CODE_COMPLIANCE: &str = "/geographic/postal-codes/:postal_code/compliance";
}

/// Compliance framework endpoints
pub mod compliance {
    use super::API_V1;
    pub const FRAMEWORKS: &str = "/compliance/frameworks";
    pub const FRAMEWORK_BY_ID: &str = "/compliance/frameworks/:id";
    pub const FRAMEWORK_RULES: &str = "/compliance/frameworks/:id/rules";
    pub const RULES: &str = "/compliance/rules";
    pub const RULE_BY_ID: &str = "/compliance/rules/:id";
    pub const AUTO_ASSIGN: &str = "/compliance/assignment/auto";
    pub const ENTITY_COMPLIANCE: &str = "/compliance/entity/:entity_id";
}

/// Organization management endpoints
pub mod organizations {
    use super::API_V1;
    pub const ORGANIZATIONS: &str = "/organizations";
    pub const ORGANIZATION_BY_ID: &str = "/organizations/:org_id";
    pub const ROLE_TEMPLATES: &str = "/role-templates";
    pub const ORGANIZATION_ROLES: &str = "/organizations/:org_id/roles";
    pub const ORGANIZATION_EMPLOYEES: &str = "/organizations/:org_id/employees";
    pub const EMPLOYEE_ROLES: &str = "/organizations/:org_id/employees/:employee_id/roles";
    pub const ORGANIZATION_PATIENTS: &str = "/organizations/:org_id/patients";
}

/// Device management endpoints
pub mod devices {
    use super::API_V1;
    pub const DEVICES: &str = "/devices";
    pub const DEVICE_BY_ID: &str = "/devices/:device_id";
    pub const CONNECT: &str = "/devices/:device_id/connect";
    pub const DISCONNECT: &str = "/devices/:device_id/disconnect";
    pub const DATA: &str = "/devices/:device_id/data";
    pub const DATA_READ: &str = "/devices/:device_id/data/read";
    pub const COMMANDS: &str = "/devices/:device_id/commands";
    pub const TYPES: &str = "/devices/types";
    pub const CONNECTION_TYPES: &str = "/devices/connection-types";
    pub const FORMATS: &str = "/devices/formats";
}

/// Secrets management endpoints
pub mod secrets {
    use super::API_V1;
    pub const HEALTH: &str = "/secrets/health";
    pub const SECRETS: &str = "/secrets";
    pub const SECRET_BY_KEY: &str = "/secrets/:key";
    pub const SECRET_VERSIONS: &str = "/secrets/:key/versions";
    pub const SECRET_VERSION: &str = "/secrets/:key/versions/:version";
    pub const ROTATE: &str = "/secrets/:key/rotate";
}

/// KMS (Key Management Service) endpoints
pub mod kms {
    use super::API_V1;
    pub const TEST: &str = "/kms/test";
    pub const DATAKEY_GENERATE: &str = "/kms/datakey/generate";
    pub const DATAKEY_DECRYPT: &str = "/kms/datakey/decrypt";
    pub const ENCRYPT: &str = "/kms/encrypt";
    pub const DECRYPT: &str = "/kms/decrypt";
    pub const RE_ENCRYPT: &str = "/kms/re-encrypt";
    pub const KEYS: &str = "/kms/keys";
    pub const KEY_BY_ID: &str = "/kms/keys/:key_id";
    pub const KEY_ENABLE: &str = "/kms/keys/:key_id/enable";
    pub const KEY_DISABLE: &str = "/kms/keys/:key_id/disable";
    pub const KEY_ROTATION_ENABLE: &str = "/kms/keys/:key_id/rotation/enable";
    pub const KEY_ROTATION_DISABLE: &str = "/kms/keys/:key_id/rotation/disable";
    pub const KEY_ROTATION_STATUS: &str = "/kms/keys/:key_id/rotation/status";
    pub const KEY_ROTATE: &str = "/kms/keys/:key_id/rotate";
    pub const KEY_SCHEDULE_DELETION: &str = "/kms/keys/:key_id/schedule-deletion";
    pub const KEY_CANCEL_DELETION: &str = "/kms/keys/:key_id/cancel-deletion";
}

/// Healthcare endpoints
pub mod healthcare {
    use super::API_V1;
    pub const MEDICAL_RECORDS: &str = "/healthcare/medical-records";
    pub const MEDICAL_RECORD_BY_ID: &str = "/healthcare/medical-records/:record_id";
    pub const MEDICAL_RECORD_AUDIT: &str = "/healthcare/medical-records/:record_id/audit";
    pub const PROVIDERS: &str = "/healthcare/providers";
    pub const SERVICE_TYPES: &str = "/healthcare/service-types";
    pub const SERVICE_TYPE_BY_ID: &str = "/healthcare/service-types/:service_type_id";
    pub const APPOINTMENTS: &str = "/healthcare/appointments";
    pub const APPOINTMENT_STATUS: &str = "/healthcare/appointments/:appointment_id/status";
}

/// Pharmacy endpoints
pub mod pharmacy {
    use super::API_V1;
    pub const PHARMACIES: &str = "/pharmacy/pharmacies";
    pub const PHARMACY_BY_ID: &str = "/pharmacy/pharmacies/:pharmacy_id";
    pub const INVENTORY: &str = "/pharmacy/inventory";
    pub const PRESCRIPTIONS: &str = "/pharmacy/prescriptions";
}

/// Vendor endpoints
pub mod vendors {
    use super::API_V1;
    pub const TYPES: &str = "/vendors/types";
    pub const VENDORS: &str = "/vendors";
    pub const VENDOR_BY_ID: &str = "/vendors/:vendor_id";
    pub const VENDOR_INVENTORY: &str = "/vendors/:vendor_id/inventory";
    pub const VENDOR_SERVICES: &str = "/vendors/:vendor_id/services";
}

/// Notification endpoints
pub mod notifications {
    use super::API_V1;
    pub const NOTIFICATIONS: &str = "/notifications";
    pub const NOTIFICATION_BY_ID: &str = "/notifications/:id";
    pub const MARK_READ: &str = "/notifications/:id/read";
    pub const BULK_READ: &str = "/notifications/bulk-read";
    pub const UNREAD_COUNT: &str = "/notifications/unread/count";
    pub const AUDIT_LOGS: &str = "/notifications/:id/audit-logs";
}

/// Onboarding endpoints
pub mod onboarding {
    use super::API_V1;
    pub const ORGANIZATION_USERS: &str = "/organizations/:org_id/users";
    pub const USER_BY_ID: &str = "/users/:user_id";
    pub const RESEND_CREDENTIALS: &str = "/users/:user_id/resend-credentials";
    pub const EMAIL_VERIFY: &str = "/email/verify";
}

/// UI components endpoints
pub mod ui_components {
    use super::API_V1;
    pub const REGISTER_COMPONENT: &str = "/ui/components/register";
    pub const REGISTER_ACTION: &str = "/ui/components/actions/register";
    pub const COMPONENTS: &str = "/ui/components";
}

/// Full API paths (for reference and utoipa)
///
/// These are the full paths including the `/api/v1` prefix.
/// Use these constants when referencing full paths in documentation or tests.
pub mod api_v1 {
    use super::API_V1;
    
    // Health
    pub const HEALTH: &str = "/health";
    pub const VERSION: &str = "/version";
    pub const STATUS: &str = "/status";
    
    // Auth
    pub const AUTH_LOGIN: &str = "/api/v1/login";
    pub const AUTH_LOGOUT: &str = "/api/v1/logout";
    pub const AUTH_OAUTH_AUTHORIZE: &str = "/api/v1/oauth/authorize";
    pub const AUTH_TOKEN_VALIDATE: &str = "/api/v1/token/validate";
    pub const AUTH_CHECK: &str = "/api/v1/auth/check";
    
    // Pharmacy
    pub const PHARMACY_PHARMACIES: &str = "/api/v1/pharmacy/pharmacies";
    pub const PHARMACY_PHARMACY_BY_ID: &str = "/api/v1/pharmacy/pharmacies/{pharmacy_id}";
    pub const PHARMACY_INVENTORY: &str = "/api/v1/pharmacy/inventory";
    pub const PHARMACY_PRESCRIPTIONS: &str = "/api/v1/pharmacy/prescriptions";
    
    // Geographic
    pub const GEOGRAPHIC_REGIONS: &str = "/api/v1/geographic/regions";
    pub const GEOGRAPHIC_REGION_BY_ID: &str = "/api/v1/geographic/regions/{id}";
    pub const GEOGRAPHIC_REGION_HIERARCHY: &str = "/api/v1/geographic/regions/{id}/hierarchy";
    pub const GEOGRAPHIC_POSTAL_CODE_COMPLIANCE: &str = "/api/v1/geographic/postal-codes/{postal_code}/compliance";
    
    // Compliance
    pub const COMPLIANCE_FRAMEWORKS: &str = "/api/v1/compliance/frameworks";
    pub const COMPLIANCE_FRAMEWORK_BY_ID: &str = "/api/v1/compliance/frameworks/{framework_id}";
    pub const COMPLIANCE_FRAMEWORK_RULES: &str = "/api/v1/compliance/frameworks/{framework_id}/rules";
    pub const COMPLIANCE_RULES: &str = "/api/v1/compliance/rules";
    pub const COMPLIANCE_RULE_BY_ID: &str = "/api/v1/compliance/rules/{id}";
    pub const COMPLIANCE_AUTO_ASSIGN: &str = "/api/v1/compliance/assignment/auto";
    pub const COMPLIANCE_ENTITY_COMPLIANCE: &str = "/api/v1/compliance/entity/{entity_type}/{entity_id}";
    pub const COMPLIANCE_ENTITY_ASSESS: &str = "/api/v1/compliance/entities/{entity_type}/{entity_id}/assess";
    
    // Healthcare
    pub const HEALTHCARE_MEDICAL_RECORDS: &str = "/api/v1/healthcare/medical-records";
    pub const HEALTHCARE_MEDICAL_RECORD_BY_ID: &str = "/api/v1/healthcare/medical-records/{record_id}";
    pub const HEALTHCARE_MEDICAL_RECORD_AUDIT: &str = "/api/v1/healthcare/medical-records/{record_id}/audit";
    pub const HEALTHCARE_PROVIDERS: &str = "/api/v1/healthcare/providers";
    pub const HEALTHCARE_SERVICE_TYPES: &str = "/api/v1/healthcare/service-types";
    pub const HEALTHCARE_SERVICE_TYPE_BY_ID: &str = "/api/v1/healthcare/service-types/{service_type_id}";
    pub const HEALTHCARE_APPOINTMENTS: &str = "/api/v1/healthcare/appointments";
    pub const HEALTHCARE_APPOINTMENT_STATUS: &str = "/api/v1/healthcare/appointments/{appointment_id}/status";
    
    // Notifications
    pub const NOTIFICATIONS: &str = "/api/v1/notifications";
    pub const NOTIFICATION_BY_ID: &str = "/api/v1/notifications/{id}";
    pub const NOTIFICATION_MARK_READ: &str = "/api/v1/notifications/{id}/read";
    pub const NOTIFICATION_BULK_READ: &str = "/api/v1/notifications/bulk-read";
    pub const NOTIFICATION_UNREAD_COUNT: &str = "/api/v1/notifications/unread/count";
    pub const NOTIFICATION_AUDIT_LOGS: &str = "/api/v1/notifications/{id}/audit-logs";
    
    // UI Components
    pub const UI_COMPONENTS_REGISTER: &str = "/api/v1/ui/components/register";
    pub const UI_COMPONENTS_REGISTER_ACTION: &str = "/api/v1/ui/components/actions/register";
    pub const UI_COMPONENTS: &str = "/api/v1/ui/components";
    
    // Organizations
    pub const ORGANIZATIONS: &str = "/api/v1/organizations";
    pub const ORGANIZATION_BY_ID: &str = "/api/v1/organizations/{org_id}";
    pub const ORGANIZATION_ROLES: &str = "/api/v1/organizations/{org_id}/roles";
    pub const ORGANIZATION_EMPLOYEES: &str = "/api/v1/organizations/{org_id}/employees";
    pub const ORGANIZATION_EMPLOYEE_ROLES: &str = "/api/v1/organizations/{org_id}/employees/{employee_id}/roles";
    pub const ORGANIZATION_PATIENTS: &str = "/api/v1/organizations/{org_id}/patients";
    pub const ORGANIZATION_USERS: &str = "/api/v1/organizations/{org_id}/users";
    pub const RESEND_CREDENTIALS: &str = "/api/v1/users/{user_id}/resend-credentials";
    pub const EMAIL_VERIFY: &str = "/api/v1/email/verify";
    pub const ROLE_TEMPLATES: &str = "/api/v1/role-templates";
    
    // Devices
    pub const DEVICES: &str = "/api/v1/devices";
    pub const DEVICE_BY_ID: &str = "/api/v1/devices/{device_id}";
    pub const DEVICE_CONNECT: &str = "/api/v1/devices/{device_id}/connect";
    pub const DEVICE_DISCONNECT: &str = "/api/v1/devices/{device_id}/disconnect";
    pub const DEVICE_DATA: &str = "/api/v1/devices/{device_id}/data";
    pub const DEVICE_DATA_READ: &str = "/api/v1/devices/{device_id}/data/read";
    pub const DEVICE_COMMANDS: &str = "/api/v1/devices/{device_id}/commands";
    pub const DEVICE_TYPES: &str = "/api/v1/devices/types";
    pub const DEVICE_CONNECTION_TYPES: &str = "/api/v1/devices/connection-types";
    pub const DEVICE_DATA_FORMATS: &str = "/api/v1/devices/data-formats";
    
    // Secrets
    pub const SECRETS_HEALTH: &str = "/api/v1/secrets/health";
    pub const SECRETS: &str = "/api/v1/secrets";
    pub const SECRET_BY_KEY: &str = "/api/v1/secrets/{key}";
    pub const SECRET_VERSIONS: &str = "/api/v1/secrets/{key}/versions";
    pub const SECRET_VERSION: &str = "/api/v1/secrets/{key}/versions/{version}";
    pub const SECRET_ROTATE: &str = "/api/v1/secrets/{key}/rotate";
    
    // KMS
    pub const KMS_TEST: &str = "/api/v1/kms/test";
    pub const KMS_DATAKEY_GENERATE: &str = "/api/v1/kms/datakey/generate";
    pub const KMS_DATAKEY_DECRYPT: &str = "/api/v1/kms/datakey/decrypt";
    pub const KMS_ENCRYPT: &str = "/api/v1/kms/encrypt";
    pub const KMS_DECRYPT: &str = "/api/v1/kms/decrypt";
    pub const KMS_RE_ENCRYPT: &str = "/api/v1/kms/re-encrypt";
    pub const KMS_KEYS: &str = "/api/v1/kms/keys";
    pub const KMS_KEY_BY_ID: &str = "/api/v1/kms/keys/{key_id}";
    pub const KMS_KEY_ROTATE: &str = "/api/v1/kms/keys/{key_id}/rotate";
    
    // Vendors
    pub const VENDORS_TYPES: &str = "/api/v1/vendors/types";
    pub const VENDORS: &str = "/api/v1/vendors";
    pub const VENDOR_BY_ID: &str = "/api/v1/vendors/{vendor_id}";
    pub const VENDOR_INVENTORY: &str = "/api/v1/vendors/{vendor_id}/inventory";
    pub const VENDOR_SERVICES: &str = "/api/v1/vendors/{vendor_id}/services";
    
    // Workflow
    pub const WORKFLOWS: &str = "/api/v1/workflows";
    pub const WORKFLOW_BY_ID: &str = "/api/v1/workflows/{workflow_id}";
    pub const WORKFLOW_EXECUTE: &str = "/api/v1/workflows/execute";
    pub const WORKFLOW_EXECUTION_BY_ID: &str = "/api/v1/workflows/executions/{execution_id}";
    
    // Sync
    pub const SYNC_PULL: &str = "/api/v1/sync/pull";
    pub const SYNC_PUSH: &str = "/api/v1/sync/push";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to convert route path to utoipa path format
    /// Converts `:param` to `{param}` and adds `/api/v1` prefix if missing
    fn route_to_utoipa_path(route: &str) -> String {
        let mut path = route.to_string();
        
        // Replace :param with {param}
        path = path.replace(":pharmacy_id", "{pharmacy_id}");
        path = path.replace(":id", "{id}");
        path = path.replace(":org_id", "{org_id}");
        path = path.replace(":device_id", "{device_id}");
        path = path.replace(":key", "{key}");
        path = path.replace(":key_id", "{key_id}");
        path = path.replace(":record_id", "{record_id}");
        path = path.replace(":service_type_id", "{service_type_id}");
        path = path.replace(":appointment_id", "{appointment_id}");
        path = path.replace(":postal_code", "{postal_code}");
        path = path.replace(":framework_id", "{framework_id}");
        path = path.replace(":entity_type", "{entity_type}");
        path = path.replace(":entity_id", "{entity_id}");
        path = path.replace(":user_id", "{user_id}");
        path = path.replace(":employee_id", "{employee_id}");
        path = path.replace(":version", "{version}");
        path = path.replace(":vendor_id", "{vendor_id}");
        
        // Add /api/v1 prefix if not present
        if !path.starts_with("/api/v1") && !path.starts_with("/health") && !path.starts_with("/version") && !path.starts_with("/status") {
            format!("/api/v1{}", path)
        } else {
            path
        }
    }

    #[test]
    fn test_route_paths_exist() {
        // Verify that key paths are defined
        assert!(!pharmacy::PHARMACIES.is_empty());
        assert!(!geographic::REGIONS.is_empty());
        assert!(!compliance::FRAMEWORKS.is_empty());
    }

    #[test]
    fn test_api_v1_paths_match_routes() {
        // Test that route paths match expected API v1 paths
        let pharmacy_path = route_to_utoipa_path(pharmacy::PHARMACIES);
        assert_eq!(pharmacy_path, api_v1::PHARMACY_PHARMACIES);
        
        let geographic_path = route_to_utoipa_path(geographic::REGIONS);
        assert_eq!(geographic_path, api_v1::GEOGRAPHIC_REGIONS);
    }
}

