use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    /// Enabled authentication providers
    pub providers: Vec<AuthProvider>,
    
    /// JWT token configuration
    pub token: TokenConfig,
    
    /// Session management configuration
    pub session: SessionConfig,
    
    /// Certificate authentication configuration
    pub certificate: Option<CertificateConfig>,
    
    /// OAuth/OIDC configuration
    pub oauth: Option<OAuthConfig>,
    
    /// Security settings
    pub security: SecurityConfig,
}

/// Authentication provider types
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthProvider {
    /// Email/password authentication
    EmailPassword,
    /// OAuth2/OIDC SSO
    OAuth,
    /// Client certificate (mTLS)
    Certificate,
    /// SAML 2.0 SSO
    Saml,
    /// WebAuthn/FIDO2
    WebAuthn,
}

/// JWT token configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenConfig {
    /// Access token lifetime in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_access_token_lifetime")]
    pub access_token_lifetime: u64,
    
    /// Refresh token lifetime in days (default: 30 days)
    #[serde(default = "default_refresh_token_lifetime")]
    pub refresh_token_lifetime: u64,
    
    /// Alias for refresh_token_lifetime (compatibility)
    #[serde(skip)]
    pub refresh_token_ttl_days: u64,
    
    /// Alias for key_grace_period_days (compatibility)
    #[serde(skip)]
    pub key_rotation_grace_period_days: u64,
    
    /// JWT signing algorithm (RS256, RS384, RS512, EdDSA)
    #[serde(default = "default_jwt_algorithm")]
    pub algorithm: String,
    
    /// RSA key size for RS256/RS384/RS512 (2048, 3072, 4096)
    #[serde(default = "default_key_size")]
    pub key_size: u32,
    
    /// JWT issuer claim
    #[serde(default = "default_issuer")]
    pub issuer: String,
    
    /// JWT audience claim
    pub audience: Option<String>,
    
    /// Key rotation interval in days (default: 90 days)
    #[serde(default = "default_key_rotation_days")]
    pub key_rotation_days: u64,
    
    /// Grace period for old keys in days (default: 30 days)
    #[serde(default = "default_key_grace_period")]
    pub key_grace_period_days: u64,
    
    /// Path to store JWT signing keys
    #[serde(default = "default_keys_path")]
    pub keys_path: PathBuf,
}

/// Session management configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionConfig {
    /// Session backend (redis, memory, database)
    #[serde(default = "default_session_backend")]
    pub backend: String,
    
    /// Redis connection URL (if backend=redis)
    pub redis_url: Option<String>,
    
    /// Idle timeout in minutes (default: 15 minutes)
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_minutes: u64,
    
    /// Absolute timeout in hours (default: 8 hours)
    #[serde(default = "default_absolute_timeout")]
    pub absolute_timeout_hours: u64,
    
    /// Maximum concurrent sessions per user (default: 3)
    #[serde(default = "default_max_sessions")]
    pub max_concurrent_sessions: u32,
    
    /// Validate IP address consistency
    #[serde(default = "default_true")]
    pub validate_ip: bool,
    
    /// Validate user agent consistency
    #[serde(default = "default_true")]
    pub validate_user_agent: bool,
    
    /// Validate device fingerprint
    #[serde(default = "default_true")]
    pub validate_device_fingerprint: bool,
}

/// Certificate authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CertificateConfig {
    /// Enable certificate authentication
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Require client certificate for all requests
    #[serde(default = "default_false")]
    pub require_client_cert: bool,
    
    /// Path to CA root certificates
    pub ca_roots_path: PathBuf,
    
    /// Path to CA intermediate certificates
    pub ca_intermediates_path: Option<PathBuf>,
    
    /// Path to Certificate Revocation Lists (CRL)
    pub crl_path: Option<PathBuf>,
    
    /// Verify full certificate chain
    #[serde(default = "default_true")]
    pub verify_chain: bool,
    
    /// Check certificate revocation status
    #[serde(default = "default_true")]
    pub check_revocation: bool,
    
    /// CRL update interval in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_crl_update_interval")]
    pub crl_update_interval: u64,
    
    /// OCSP configuration
    pub ocsp: Option<OcspConfig>,
    
    /// Certificate identity mapping
    pub identity_mapping: CertificateIdentityMapping,
    
    /// Allowed certificate key usages
    #[serde(default = "default_key_usages")]
    pub allowed_key_usages: Vec<String>,
    
    /// Maximum certificate chain depth
    #[serde(default = "default_max_chain_depth")]
    pub max_chain_depth: u8,
}

/// OCSP (Online Certificate Status Protocol) configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OcspConfig {
    /// Enable OCSP checking
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// OCSP responder URL (if not in certificate)
    pub responder_url: Option<String>,
    
    /// OCSP request timeout in seconds
    #[serde(default = "default_ocsp_timeout")]
    pub timeout: u64,
    
    /// Cache OCSP responses (TTL in seconds)
    #[serde(default = "default_ocsp_cache_ttl")]
    pub cache_ttl: u64,
    
    /// Fallback to CRL if OCSP unavailable
    #[serde(default = "default_true")]
    pub fallback_to_crl: bool,
}

/// Certificate identity mapping configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CertificateIdentityMapping {
    /// Primary subject field for identity (e.g., "emailAddress", "CN", "UID")
    #[serde(default = "default_subject_field")]
    pub subject_field: String,
    
    /// Fallback fields if primary not found
    #[serde(default = "default_fallback_fields")]
    pub fallback_fields: Vec<String>,
    
    /// Use Subject Alternative Name (SAN) for email
    #[serde(default = "default_true")]
    pub use_san_email: bool,
    
    /// Custom attribute mapping (OID -> claim name)
    #[serde(default)]
    pub custom_attributes: std::collections::HashMap<String, String>,
}

/// OAuth/OIDC configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthConfig {
    /// OAuth providers
    pub providers: Vec<OAuthProviderConfig>,
    
    /// Default redirect URI after successful authentication
    pub default_redirect_uri: String,
    
    /// Enable PKCE for all flows
    #[serde(default = "default_true")]
    pub enable_pkce: bool,
    
    /// State parameter timeout in seconds
    #[serde(default = "default_oauth_state_timeout")]
    pub state_timeout: u64,
}

/// OAuth provider configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthProviderConfig {
    /// Provider name (google, azure, okta, github, etc.)
    pub name: String,
    
    /// Provider type (oidc, oauth2)
    pub provider_type: String,
    
    /// Client ID
    pub client_id: String,
    
    /// Client secret
    pub client_secret: String,
    
    /// Authorization endpoint URL
    pub authorization_url: String,
    
    /// Token endpoint URL
    pub token_url: String,
    
    /// UserInfo endpoint URL (OIDC)
    pub userinfo_url: Option<String>,
    
    /// JWKS endpoint URL (OIDC)
    pub jwks_url: Option<String>,
    
    /// Requested scopes
    #[serde(default = "default_oauth_scopes")]
    pub scopes: Vec<String>,
    
    /// User attribute mapping (provider attr -> internal claim)
    #[serde(default)]
    pub attribute_mapping: std::collections::HashMap<String, String>,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    /// Enable anomaly detection
    #[serde(default = "default_true")]
    pub anomaly_detection: bool,
    
    /// Maximum failed authentication attempts before lockout
    #[serde(default = "default_max_failed_attempts")]
    pub max_failed_attempts: u32,
    
    /// Account lockout duration in minutes
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration_minutes: u64,
    
    /// Enable step-up authentication
    #[serde(default = "default_true")]
    pub step_up_auth_enabled: bool,
    
    /// Step-up token lifetime in seconds (default: 120 = 2 minutes)
    #[serde(default = "default_step_up_lifetime")]
    pub step_up_token_lifetime: u64,
    
    /// Operations requiring step-up authentication
    #[serde(default = "default_step_up_operations")]
    pub step_up_required_for: Vec<String>,
    
    /// Audit all authentication events
    #[serde(default = "default_true")]
    pub audit_all_events: bool,
    
    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub rate_limiting: bool,
    
    /// Rate limit: requests per minute
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

// Default value functions

fn default_access_token_lifetime() -> u64 { 300 } // 5 minutes
fn default_refresh_token_lifetime() -> u64 { 30 } // 30 days
fn default_jwt_algorithm() -> String { "RS256".to_string() }
fn default_key_size() -> u32 { 4096 }
fn default_issuer() -> String { "rustcare-engine".to_string() }
fn default_key_rotation_days() -> u64 { 90 }
fn default_key_grace_period() -> u64 { 30 }
fn default_keys_path() -> PathBuf { PathBuf::from("/etc/rustcare/auth/keys") }

fn default_session_backend() -> String { "redis".to_string() }
fn default_idle_timeout() -> u64 { 15 } // 15 minutes
fn default_absolute_timeout() -> u64 { 8 } // 8 hours
fn default_max_sessions() -> u32 { 3 }

fn default_crl_update_interval() -> u64 { 3600 } // 1 hour
fn default_ocsp_timeout() -> u64 { 5 } // 5 seconds
fn default_ocsp_cache_ttl() -> u64 { 300 } // 5 minutes
fn default_max_chain_depth() -> u8 { 5 }

fn default_subject_field() -> String { "emailAddress".to_string() }
fn default_fallback_fields() -> Vec<String> { 
    vec!["CN".to_string(), "UID".to_string()] 
}

fn default_key_usages() -> Vec<String> {
    vec!["digitalSignature".to_string(), "keyEncipherment".to_string()]
}

fn default_oauth_scopes() -> Vec<String> {
    vec!["openid".to_string(), "profile".to_string(), "email".to_string()]
}

fn default_oauth_state_timeout() -> u64 { 600 } // 10 minutes

fn default_max_failed_attempts() -> u32 { 5 }
fn default_lockout_duration() -> u64 { 30 } // 30 minutes
fn default_step_up_lifetime() -> u64 { 120 } // 2 minutes
fn default_rate_limit() -> u32 { 60 } // 60 requests per minute

fn default_step_up_operations() -> Vec<String> {
    vec![
        "patient_data_export".to_string(),
        "admin_config_change".to_string(),
        "user_permission_modify".to_string(),
        "billing_operations".to_string(),
    ]
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            providers: vec![AuthProvider::EmailPassword],
            token: TokenConfig::default(),
            session: SessionConfig::default(),
            certificate: None,
            oauth: None,
            security: SecurityConfig::default(),
        }
    }
}

impl Default for TokenConfig {
    fn default() -> Self {
        let refresh_token_lifetime = default_refresh_token_lifetime();
        let key_grace_period_days = default_key_grace_period();
        Self {
            access_token_lifetime: default_access_token_lifetime(),
            refresh_token_lifetime,
            refresh_token_ttl_days: refresh_token_lifetime,
            key_rotation_grace_period_days: key_grace_period_days,
            algorithm: default_jwt_algorithm(),
            key_size: default_key_size(),
            issuer: default_issuer(),
            audience: None,
            key_rotation_days: default_key_rotation_days(),
            key_grace_period_days,
            keys_path: default_keys_path(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            backend: default_session_backend(),
            redis_url: Some("redis://localhost:6379".to_string()),
            idle_timeout_minutes: default_idle_timeout(),
            absolute_timeout_hours: default_absolute_timeout(),
            max_concurrent_sessions: default_max_sessions(),
            validate_ip: true,
            validate_user_agent: true,
            validate_device_fingerprint: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            anomaly_detection: true,
            max_failed_attempts: default_max_failed_attempts(),
            lockout_duration_minutes: default_lockout_duration(),
            step_up_auth_enabled: true,
            step_up_token_lifetime: default_step_up_lifetime(),
            step_up_required_for: default_step_up_operations(),
            audit_all_events: true,
            rate_limiting: true,
            rate_limit_per_minute: default_rate_limit(),
        }
    }
}

impl AuthConfig {
    /// Load configuration from TOML file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: AuthConfig = toml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        // TODO: Implement environment variable parsing
        Ok(Self::default())
    }
    
    /// Check if a specific provider is enabled
    pub fn has_provider(&self, provider: &AuthProvider) -> bool {
        self.providers.contains(provider)
    }
    
    /// Get access token duration
    pub fn access_token_duration(&self) -> Duration {
        Duration::from_secs(self.token.access_token_lifetime)
    }
    
    /// Get refresh token duration
    pub fn refresh_token_duration(&self) -> Duration {
        Duration::from_secs(self.token.refresh_token_lifetime * 24 * 3600)
    }
    
    /// Get idle timeout duration
    pub fn idle_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.session.idle_timeout_minutes * 60)
    }
    
    /// Get absolute timeout duration
    pub fn absolute_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.session.absolute_timeout_hours * 3600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.token.access_token_lifetime, 300); // 5 minutes
        assert_eq!(config.token.refresh_token_lifetime, 30); // 30 days
        assert!(config.has_provider(&AuthProvider::EmailPassword));
    }
    
    #[test]
    fn test_durations() {
        let config = AuthConfig::default();
        assert_eq!(config.access_token_duration().as_secs(), 300);
        assert_eq!(config.refresh_token_duration().as_secs(), 30 * 24 * 3600);
    }
}
