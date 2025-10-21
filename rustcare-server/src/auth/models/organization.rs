/// Organization model for multi-tenant architecture
///
/// Represents a tenant/organization in the system with subscription tiers,
/// limits, and organization-specific settings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Subscription tier for organizations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Starter,
    Professional,
    Enterprise,
    Custom,
}

impl std::fmt::Display for SubscriptionTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionTier::Free => write!(f, "free"),
            SubscriptionTier::Starter => write!(f, "starter"),
            SubscriptionTier::Professional => write!(f, "professional"),
            SubscriptionTier::Enterprise => write!(f, "enterprise"),
            SubscriptionTier::Custom => write!(f, "custom"),
        }
    }
}

/// Organization entity representing a tenant in the multi-tenant system
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: Uuid,
    
    // Organization identification
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    
    // Organization details
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub website_url: Option<String>,
    
    // Subscription and limits
    pub subscription_tier: String,
    pub max_users: i32,
    pub max_storage_gb: i32,
    
    // Organization status
    pub is_active: bool,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    
    // Settings (JSONB)
    pub settings: serde_json::Value,
    
    // Contact information
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub billing_email: Option<String>,
    
    // Address
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    
    // Audit timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Organization creation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrganization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub description: Option<String>,
    pub subscription_tier: SubscriptionTier,
    pub max_users: Option<i32>,
    pub max_storage_gb: Option<i32>,
    pub contact_email: Option<String>,
    pub settings: Option<serde_json::Value>,
    pub is_active: Option<bool>,
    pub contact_phone: Option<String>,
    pub billing_email: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub is_verified: Option<bool>,
    pub state: Option<String>,
    pub tax_id: Option<String>,
}

/// Organization update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOrganization {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub website_url: Option<String>,
    pub subscription_tier: Option<SubscriptionTier>,
    pub max_users: Option<i32>,
    pub max_storage_gb: Option<i32>,
    pub is_active: Option<bool>,
    pub settings: Option<serde_json::Value>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub billing_email: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state_province: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub is_verified: Option<bool>,
    pub state: Option<String>,
    pub tax_id: Option<String>,
}

/// Organization settings structure (example)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branding: Option<BrandingSettings>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySettings>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<FeatureSettings>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance: Option<ComplianceSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingSettings {
    pub primary_color: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub require_mfa: bool,
    pub session_timeout_minutes: i32,
    pub password_min_length: i32,
    pub password_require_special: bool,
    pub max_login_attempts: i32,
    pub lockout_duration_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSettings {
    pub enable_audit_logs: bool,
    pub enable_api_access: bool,
    pub enable_webhooks: bool,
    pub enable_sso: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSettings {
    pub hipaa_enabled: bool,
    pub data_retention_days: i32,
    pub enable_encryption_at_rest: bool,
    pub enable_encryption_in_transit: bool,
}

impl Default for OrganizationSettings {
    fn default() -> Self {
        Self {
            branding: None,
            security: Some(SecuritySettings {
                require_mfa: false,
                session_timeout_minutes: 30,
                password_min_length: 12,
                password_require_special: true,
                max_login_attempts: 5,
                lockout_duration_minutes: 15,
            }),
            features: Some(FeatureSettings {
                enable_audit_logs: true,
                enable_api_access: false,
                enable_webhooks: false,
                enable_sso: false,
            }),
            compliance: Some(ComplianceSettings {
                hipaa_enabled: false,
                data_retention_days: 2555, // 7 years
                enable_encryption_at_rest: true,
                enable_encryption_in_transit: true,
            }),
        }
    }
}

/// System organization ID constant
pub const SYSTEM_ORGANIZATION_ID: Uuid = Uuid::from_bytes([0; 16]);
