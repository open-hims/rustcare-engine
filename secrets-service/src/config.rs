//! Configuration for secrets service

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsServiceConfig {
    /// Active provider
    pub provider: ProviderConfig,
    
    /// Cache configuration
    pub cache: CacheConfig,
    
    /// Rotation configuration
    pub rotation: RotationConfig,
    
    /// Audit configuration
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProviderConfig {
    Vault(VaultConfig),
    AwsSecretsManager(AwsSecretsManagerConfig),
    AzureKeyVault(AzureKeyVaultConfig),
    GcpSecretManager(GcpSecretManagerConfig),
    Kubernetes(KubernetesConfig),
    Environment(EnvironmentConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub address: String,
    pub token: Option<String>,
    pub app_role: Option<AppRoleAuth>,
    pub kubernetes_auth: Option<K8sAuth>,
    pub mount_path: String,
    pub namespace: Option<String>,
    pub tls_ca_cert: Option<String>,
    pub tls_client_cert: Option<String>,
    pub tls_client_key: Option<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRoleAuth {
    pub role_id: String,
    pub secret_id: String,
    pub mount_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sAuth {
    pub role: String,
    pub jwt_path: String,
    pub mount_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsSecretsManagerConfig {
    pub region: String,
    pub role_arn: Option<String>,
    pub external_id: Option<String>,
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureKeyVaultConfig {
    pub vault_name: String,
    pub tenant_id: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub use_managed_identity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpSecretManagerConfig {
    pub project_id: String,
    pub credentials_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub namespace: String,
    pub api_server: Option<String>,
    pub token_path: String,
    pub ca_cert_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    pub enabled: bool,
    pub check_interval_seconds: u64,
    pub default_rotation_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_all_access: bool,
    pub log_rotation_events: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_seconds: 300,
            max_entries: 1000,
        }
    }
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            check_interval_seconds: 3600,
            default_rotation_days: 90,
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_all_access: true,
            log_rotation_events: true,
        }
    }
}
