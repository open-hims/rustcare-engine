//! Secrets manager that coordinates multiple providers with caching

use async_trait::async_trait;
use crate::{
    SecretProvider, Secret, SecretMetadata, Result, SecretsError, HealthStatus,
    config::{ProviderConfig, CacheConfig, AuditConfig},
    cache::SecretCache,
    audit::{AuditLogger, AuditEvent, AuditEventType},
    providers::{VaultProvider, AwsSecretsManagerProvider},
};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct SecretsManager {
    providers: Vec<Arc<dyn SecretProvider + Send + Sync>>,
    cache: Option<SecretCache>,
    audit: AuditLogger,
}

impl SecretsManager {
    pub async fn new(
        provider_configs: Vec<ProviderConfig>,
        cache_config: Option<CacheConfig>,
        audit_config: AuditConfig,
    ) -> Result<Self> {
        let mut providers: Vec<Arc<dyn SecretProvider + Send + Sync>> = Vec::new();
        
        for config in provider_configs {
            match config {
                ProviderConfig::Vault(vault_config) => {
                    info!("Initializing Vault provider: {}", vault_config.address);
                    let provider = VaultProvider::new(vault_config).await?;
                    providers.push(Arc::new(provider));
                }
                ProviderConfig::AwsSecretsManager(aws_config) => {
                    info!("Initializing AWS Secrets Manager provider");
                    let provider = AwsSecretsManagerProvider::new(aws_config).await?;
                    providers.push(Arc::new(provider));
                }
                ProviderConfig::AzureKeyVault(_azure_config) => {
                    warn!("Azure Key Vault provider not yet implemented");
                    // TODO: Implement Azure provider
                }
                ProviderConfig::GcpSecretManager(_gcp_config) => {
                    warn!("GCP Secret Manager provider not yet implemented");
                    // TODO: Implement GCP provider
                }
                ProviderConfig::Kubernetes(_k8s_config) => {
                    warn!("Kubernetes Secrets provider not yet implemented");
                    // TODO: Implement K8s provider
                }
                ProviderConfig::Environment(_env_config) => {
                    warn!("Environment provider not yet implemented");
                    // TODO: Implement Environment provider
                }
            }
        }
        
        if providers.is_empty() {
            return Err(SecretsError::ConfigurationError(
                "No providers configured".to_string()
            ));
        }
        
        let cache = if let Some(cfg) = cache_config {
            if cfg.enabled {
                Some(SecretCache::new(cfg.ttl_seconds, cfg.max_entries))
            } else {
                None
            }
        } else {
            None
        };
        
        let audit = AuditLogger::new(audit_config.enabled, audit_config.log_all_access);
        
        Ok(Self {
            providers,
            cache,
            audit,
        })
    }
    
    /// Try to get secret from cache first, then from providers
    async fn get_with_cache(&self, key: &str) -> Result<Secret> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(secret) = cache.get(key).await {
                debug!("Secret found in cache: {}", key);
                self.audit.log_access(key, None);
                return Ok(secret);
            }
        }
        
        // Try each provider in order
        let mut last_error = None;
        for provider in &self.providers {
            match provider.get_secret(key).await {
                Ok(secret) => {
                    // Cache the secret if caching is enabled
                    if let Some(ref cache) = self.cache {
                        let _ = cache.set(key.to_string(), secret.clone()).await;
                    }
                    
                    self.audit.log_event(AuditEvent {
                        timestamp: chrono::Utc::now(),
                        event_type: AuditEventType::SecretAccessed,
                        secret_key: key.to_string(),
                        user: None,
                        success: true,
                        error_message: None,
                    });
                    
                    return Ok(secret);
                }
                Err(SecretsError::NotFound(_)) => {
                    // Continue to next provider
                    continue;
                }
                Err(e) => {
                    warn!("Provider error for key '{}': {}", key, e);
                    last_error = Some(e);
                }
            }
        }
        
        let error = last_error.unwrap_or(SecretsError::NotFound(key.to_string()));
        
        self.audit.log_event(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecretAccessed,
            secret_key: key.to_string(),
            user: None,
            success: false,
            error_message: Some(error.to_string()),
        });
        
        Err(error)
    }
}

#[async_trait]
impl SecretProvider for SecretsManager {
    fn name(&self) -> &str {
        "secrets-manager"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let mut all_healthy = true;
        let mut messages = Vec::new();
        let start = std::time::Instant::now();
        
        for (i, provider) in self.providers.iter().enumerate() {
            match provider.health_check().await {
                Ok(status) => {
                    if !status.healthy {
                        warn!("Provider {} ({}) is unhealthy: {}", i, provider.name(), status.message);
                        all_healthy = false;
                        messages.push(format!("{}: {}", provider.name(), status.message));
                    }
                }
                Err(e) => {
                    warn!("Health check failed for provider {} ({}): {}", i, provider.name(), e);
                    all_healthy = false;
                    messages.push(format!("{}: {}", provider.name(), e));
                }
            }
        }
        
        let latency_ms = start.elapsed().as_millis() as u64;
        
        Ok(HealthStatus {
            healthy: all_healthy,
            message: if all_healthy {
                "All providers healthy".to_string()
            } else {
                format!("Some providers unhealthy: {}", messages.join("; "))
            },
            latency_ms,
            last_check: chrono::Utc::now(),
        })
    }
    
    async fn get_secret(&self, key: &str) -> Result<Secret> {
        self.get_with_cache(key).await
    }
    
    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        debug!("Getting secret version: {} v{}", key, version);
        
        // Try each provider
        for provider in &self.providers {
            match provider.get_secret_version(key, version).await {
                Ok(secret) => return Ok(secret),
                Err(SecretsError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        
        Err(SecretsError::NotFound(format!("{} version {}", key, version)))
    }
    
    async fn set_secret(&self, key: &str, value: &str, metadata: Option<SecretMetadata>) -> Result<()> {
        debug!("Setting secret: {}", key);
        
        // Try first provider (primary)
        self.providers[0].set_secret(key, value, metadata).await?;
        
        // Invalidate cache
        if let Some(ref cache) = self.cache {
            let _ = cache.invalidate(key).await;
        }
        
        self.audit.log_event(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecretCreated,
            secret_key: key.to_string(),
            user: None,
            success: true,
            error_message: None,
        });
        
        Ok(())
    }
    
    async fn delete_secret(&self, key: &str) -> Result<()> {
        debug!("Deleting secret: {}", key);
        
        // Try first provider (primary)
        self.providers[0].delete_secret(key).await?;
        
        // Invalidate cache
        if let Some(ref cache) = self.cache {
            let _ = cache.invalidate(key).await;
        }
        
        self.audit.log_event(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecretDeleted,
            secret_key: key.to_string(),
            user: None,
            success: true,
            error_message: None,
        });
        
        Ok(())
    }
    
    async fn list_secrets(&self) -> Result<Vec<String>> {
        debug!("Listing secrets");
        
        // Use first provider for listing
        self.providers[0].list_secrets().await
    }
    
    async fn list_versions(&self, key: &str) -> Result<Vec<String>> {
        debug!("Listing versions for: {}", key);
        
        // Try each provider
        for provider in &self.providers {
            match provider.list_versions(key).await {
                Ok(versions) => return Ok(versions),
                Err(SecretsError::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        
        Err(SecretsError::NotFound(key.to_string()))
    }
    
    async fn rotate_secret(&self, key: &str) -> Result<String> {
        debug!("Rotating secret: {}", key);
        
        // Try first provider (primary)
        let result = self.providers[0].rotate_secret(key).await?;
        
        // Invalidate cache
        if let Some(ref cache) = self.cache {
            let _ = cache.invalidate(key).await;
        }
        
        self.audit.log_event(AuditEvent {
            timestamp: chrono::Utc::now(),
            event_type: AuditEventType::SecretRotated,
            secret_key: key.to_string(),
            user: None,
            success: true,
            error_message: None,
        });
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VaultConfig;
    
    #[tokio::test]
    #[ignore] // Requires running Vault
    async fn test_secrets_manager() {
        let vault_config = VaultConfig {
            address: "http://localhost:8200".to_string(),
            token: Some("root".to_string()),
            app_role: None,
            kubernetes_auth: None,
            mount_path: "secret".to_string(),
            namespace: None,
            tls_ca_cert: None,
            tls_client_cert: None,
            tls_client_key: None,
            timeout_seconds: 30,
        };
        
        let cache_config = CacheConfig {
            enabled: true,
            ttl_seconds: 300,
            max_entries: 1000,
        };
        
        let audit_config = AuditConfig {
            enabled: true,
            log_all_access: true,
            log_rotation_events: true,
        };
        
        let manager = SecretsManager::new(
            vec![ProviderConfig::Vault(vault_config)],
            Some(cache_config),
            audit_config,
        ).await.unwrap();
        
        // Test operations
        let value = serde_json::json!({"password": "test123"});
        manager.set_secret("test/app", &value.to_string(), None).await.unwrap();
        
        let secret = manager.get_secret("test/app").await.unwrap();
        assert_eq!(secret.metadata.key, "test/app");
        
        // Second get should come from cache
        let secret2 = manager.get_secret("test/app").await.unwrap();
        assert_eq!(secret2.metadata.key, "test/app");
        
        manager.delete_secret("test/app").await.unwrap();
    }
}
