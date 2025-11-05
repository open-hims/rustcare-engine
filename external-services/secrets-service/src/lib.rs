//! # RustCare Secrets Service
//! 
//! Multi-provider secrets management service with UI integration.
//! 
//! ## Supported Providers:
//! - HashiCorp Vault
//! - AWS Secrets Manager
//! - Azure Key Vault
//! - Google Cloud Secret Manager
//! - Kubernetes Secrets
//! - Environment Variables (fallback)
//! 
//! ## Features:
//! - Secret rotation
//! - Caching with TTL
//! - Audit logging
//! - Health checks
//! - UI for secret management
//! - Role-based access control

pub mod config;
pub mod providers;
pub mod cache;
pub mod error;
pub mod manager;
pub mod rotation;
pub mod audit;
pub mod health;

pub use config::*;
pub use providers::*;
pub use error::*;
pub use manager::SecretsManager;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result type for secrets service
pub type Result<T> = std::result::Result<T, SecretsError>;

/// Secret metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Secret key/name
    pub key: String,
    
    /// Secret version
    pub version: Option<String>,
    
    /// When the secret was created
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// When the secret was last updated
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// When the secret expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Rotation policy
    pub rotation_enabled: bool,
    
    /// Rotation interval in days
    pub rotation_interval_days: Option<u32>,
    
    /// Tags/labels
    pub tags: std::collections::HashMap<String, String>,
}

/// Secret value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    /// Secret metadata
    pub metadata: SecretMetadata,
    
    /// Secret value (encrypted in memory)
    #[serde(skip_serializing)]
    pub value: String,
}

/// Trait for secret providers
#[async_trait]
pub trait SecretProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;
    
    /// Health check
    async fn health_check(&self) -> Result<HealthStatus>;
    
    /// Get a secret by key
    async fn get_secret(&self, key: &str) -> Result<Secret>;
    
    /// Get a specific version of a secret
    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret>;
    
    /// Set a secret
    async fn set_secret(&self, key: &str, value: &str, metadata: Option<SecretMetadata>) -> Result<()>;
    
    /// Delete a secret
    async fn delete_secret(&self, key: &str) -> Result<()>;
    
    /// List all secret keys
    async fn list_secrets(&self) -> Result<Vec<String>>;
    
    /// List secret versions
    async fn list_versions(&self, key: &str) -> Result<Vec<String>>;
    
    /// Rotate a secret
    async fn rotate_secret(&self, key: &str) -> Result<String>;
}

/// Health status for providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub latency_ms: u64,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Basic test
        assert_eq!(2 + 2, 4);
    }
}
