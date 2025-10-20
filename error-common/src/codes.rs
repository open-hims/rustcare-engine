// Error codes implementation
// This module contains standardized error codes for the RustCare Engine

pub mod validation {
    pub const INVALID_INPUT: &str = "VALIDATION_1001";
    pub const MISSING_REQUIRED_FIELD: &str = "VALIDATION_1002";
    pub const INVALID_FORMAT: &str = "VALIDATION_1003";
}

pub mod authentication {
    pub const INVALID_CREDENTIALS: &str = "AUTH_2001";
    pub const TOKEN_EXPIRED: &str = "AUTH_2002";
    pub const SESSION_INVALID: &str = "AUTH_2003";
}

pub mod authorization {
    pub const ACCESS_DENIED: &str = "AUTHZ_3001";
    pub const INSUFFICIENT_PERMISSIONS: &str = "AUTHZ_3002";
}

pub mod database {
    pub const CONNECTION_FAILED: &str = "DB_4001";
    pub const QUERY_FAILED: &str = "DB_4002";
    pub const CONSTRAINT_VIOLATION: &str = "DB_4003";
}