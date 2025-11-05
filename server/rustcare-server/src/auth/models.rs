/// Database models for authentication system
/// 
/// These models correspond to the database schema in migrations/001_create_auth_tables.sql
/// Uses SQLx for compile-time verified queries

use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use sqlx::types::Uuid;

// =============================================================================
// ORGANIZATION MODEL
// =============================================================================

mod organization;
pub use organization::*;

// =============================================================================
// USER MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub full_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub last_login_ip: Option<IpNetwork>,
    pub last_login_method: Option<String>,
    pub failed_login_attempts: i32,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    Locked,
    PendingVerification,
}

impl User {
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active && self.deleted_at.is_none()
    }
    
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            locked_until > Utc::now()
        } else {
            false
        }
    }
}

// =============================================================================
// USER CREDENTIALS MODEL (Email/Password)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserCredential {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub password_hash: String,
    pub password_algorithm: String,
    pub password_changed_at: DateTime<Utc>,
    pub password_expires_at: Option<DateTime<Utc>>,
    pub mfa_enabled: bool,
    pub mfa_secret: Option<String>,
    pub mfa_backup_codes: Option<Vec<String>>,
    pub mfa_enabled_at: Option<DateTime<Utc>>,
    pub security_questions: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserCredential {
    pub fn is_password_expired(&self) -> bool {
        if let Some(expires_at) = self.password_expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }
}

// =============================================================================
// OAUTH ACCOUNT MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthAccount {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub provider_email: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub provider_data: Option<serde_json::Value>,
    pub scopes: Option<Vec<String>>,
    pub first_login_at: DateTime<Utc>,
    pub last_login_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// CLIENT CERTIFICATE MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClientCertificate {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub serial_number: String,
    pub fingerprint_sha256: String,
    pub subject_dn: String,
    pub issuer_dn: String,
    pub common_name: Option<String>,
    pub email_address: Option<String>,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub status: CertificateStatus,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
    pub certificate_pem: String,
    pub public_key_pem: Option<String>,
    pub first_login_at: DateTime<Utc>,
    pub last_login_at: DateTime<Utc>,
    pub login_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "text")]
pub enum CertificateStatus {
    Active,
    Revoked,
    Expired,
    Suspended,
}

impl ClientCertificate {
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        self.status == CertificateStatus::Active
            && self.not_before <= now
            && self.not_after > now
    }
}

// =============================================================================
// REFRESH TOKEN MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub token_family: Uuid,
    pub device_name: Option<String>,
    pub device_fingerprint: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<IpNetwork>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_reason: Option<String>,
    pub auth_method: Option<String>,
    pub cert_serial: Option<String>,
    pub parent_token_id: Option<Uuid>,
    pub replaced_by_token_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl RefreshToken {
    pub fn is_valid(&self) -> bool {
        !self.revoked && self.expires_at > Utc::now()
    }
}

// =============================================================================
// SESSION MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub session_token: String,
    pub device_fingerprint: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<IpNetwork>,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub auth_method: String,
    pub cert_serial: Option<String>,
    pub oauth_provider: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub active: bool,
    pub terminated_at: Option<DateTime<Utc>>,
    pub termination_reason: Option<String>,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        self.active && self.expires_at > Utc::now()
    }
    
    pub fn is_idle(&self, idle_timeout_seconds: i64) -> bool {
        let now = Utc::now();
        let idle_duration = now - self.last_activity_at;
        idle_duration.num_seconds() > idle_timeout_seconds
    }
}

// =============================================================================
// JWT SIGNING KEY MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JwtSigningKey {
    pub id: Uuid,
    pub organization_id: Option<Uuid>, // Global keys allowed (NULL for system-wide keys)
    pub kid: String,
    pub algorithm: String,
    pub private_key_pem: String,
    pub public_key_pem: String,
    pub status: KeyStatus,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub rotated_at: Option<DateTime<Utc>>,
    pub retired_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tokens_signed: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub key_size: Option<i32>,
    pub rotation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "text")]
pub enum KeyStatus {
    Active,
    Rotating,
    Retired,
}

impl JwtSigningKey {
    pub fn is_usable(&self) -> bool {
        self.status == KeyStatus::Active || self.status == KeyStatus::Rotating
    }
}

// =============================================================================
// AUTH AUDIT LOG MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuthAuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub email: Option<String>,
    pub event_type: String,
    pub event_status: String,
    pub auth_method: Option<String>,
    pub oauth_provider: Option<String>,
    pub cert_serial: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<IpNetwork>,
    pub user_agent: Option<String>,
    pub device_fingerprint: Option<String>,
    pub geolocation: Option<serde_json::Value>,
    pub session_id: Option<Uuid>,
    pub request_id: Option<String>,
    pub endpoint: Option<String>,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub anomaly_detected: Option<bool>,
    pub risk_score: Option<i32>,
    pub blocked_reason: Option<String>,
    pub organization_id: Option<Uuid>,
}

// =============================================================================
// USER PERMISSION MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPermission {
    pub id: Uuid,
    pub organization_id: Option<Uuid>, // NULL for global permissions
    pub user_id: Uuid,
    pub permission: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl UserPermission {
    pub fn is_valid(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at > Utc::now()
        } else {
            true
        }
    }
}

// =============================================================================
// RATE LIMIT MODEL
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RateLimit {
    pub id: Uuid,
    pub organization_id: Option<Uuid>, // NULL for global rate limits
    pub key_type: String,
    pub key_value: String,
    pub endpoint: Option<String>,
    pub request_count: i32,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RateLimit {
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            locked_until > Utc::now()
        } else {
            false
        }
    }
    
    pub fn is_window_active(&self) -> bool {
        let now = Utc::now();
        now >= self.window_start && now < self.window_end
    }
}

// =============================================================================
// HELPER STRUCTS FOR QUERIES
// =============================================================================

/// User with authentication methods view
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserWithAuthMethods {
    pub user_id: Option<Uuid>,
    pub email: Option<String>,
    pub has_password: Option<bool>,
    pub oauth_providers: Option<Vec<String>>,
    pub active_certificates: Option<i64>,
}

/// Active session with user info
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActiveSessionWithUser {
    pub id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub user_status: Option<String>,
    pub session_token: Option<String>,
    pub device_name: Option<String>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub auth_method: Option<String>,
}

// =============================================================================
// CONVERSION IMPLEMENTATIONS
// =============================================================================

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserStatus::Active => write!(f, "active"),
            UserStatus::Inactive => write!(f, "inactive"),
            UserStatus::Suspended => write!(f, "suspended"),
            UserStatus::Locked => write!(f, "locked"),
            UserStatus::PendingVerification => write!(f, "pending_verification"),
        }
    }
}

impl std::fmt::Display for CertificateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateStatus::Active => write!(f, "active"),
            CertificateStatus::Revoked => write!(f, "revoked"),
            CertificateStatus::Expired => write!(f, "expired"),
            CertificateStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::fmt::Display for KeyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyStatus::Active => write!(f, "active"),
            KeyStatus::Rotating => write!(f, "rotating"),
            KeyStatus::Retired => write!(f, "retired"),
        }
    }
}

impl PartialEq for UserStatus {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (UserStatus::Active, UserStatus::Active)
                | (UserStatus::Inactive, UserStatus::Inactive)
                | (UserStatus::Suspended, UserStatus::Suspended)
                | (UserStatus::Locked, UserStatus::Locked)
                | (UserStatus::PendingVerification, UserStatus::PendingVerification)
        )
    }
}

impl Eq for UserStatus {}
