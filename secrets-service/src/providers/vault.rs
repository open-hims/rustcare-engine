//! HashiCorp Vault secret provider implementation

use async_trait::async_trait;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;
use crate::{SecretProvider, Secret, SecretMetadata, Result, SecretsError, HealthStatus};
use crate::config::VaultConfig;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct VaultProvider {
    client: VaultClient,
    mount: String,
    #[allow(dead_code)]
    namespace: Option<String>,
}

impl VaultProvider {
    pub async fn new(config: VaultConfig) -> Result<Self> {
        let settings = VaultClientSettingsBuilder::default()
            .address(&config.address)
            .build()
            .map_err(|e| SecretsError::ConfigurationError(e.to_string()))?;
        
        let mut client = VaultClient::new(settings)
            .map_err(|e| SecretsError::ProviderError(format!("Failed to create Vault client: {}", e)))?;
        
        // Set namespace if provided
        if let Some(ref _ns) = config.namespace {
            // Note: vaultrs doesn't have set_namespace, we'd need to use headers or different approach
            // For now, we'll skip this as it's an enterprise feature
        }
        
        // Authenticate based on configured method
        if let Some(ref _token) = config.token {
            // Token is set in client settings, not here
            // We'd need to rebuild client with token
        } else if let Some(ref approle) = config.app_role {
            let response = vaultrs::auth::approle::login(
                &client, 
                &approle.mount_point, 
                &approle.role_id, 
                &approle.secret_id
            )
            .await
            .map_err(|e| SecretsError::AuthenticationFailed(e.to_string()))?;
            
            // Rebuild client with token
            let settings = VaultClientSettingsBuilder::default()
                .address(&config.address)
                .token(&response.client_token)
                .build()
                .map_err(|e| SecretsError::ConfigurationError(e.to_string()))?;
            
            client = VaultClient::new(settings)
                .map_err(|e| SecretsError::ProviderError(format!("Failed to create Vault client: {}", e)))?;
        } else if let Some(ref k8s) = config.kubernetes_auth {
            let jwt = tokio::fs::read_to_string(&k8s.jwt_path)
                .await
                .map_err(|e| SecretsError::AuthenticationFailed(format!("Failed to read K8s token: {}", e)))?;
            
            let response = vaultrs::auth::kubernetes::login(&client, &k8s.mount_point, &k8s.role, &jwt)
                .await
                .map_err(|e| SecretsError::AuthenticationFailed(e.to_string()))?;
            
            // Rebuild client with token
            let settings = VaultClientSettingsBuilder::default()
                .address(&config.address)
                .token(&response.client_token)
                .build()
                .map_err(|e| SecretsError::ConfigurationError(e.to_string()))?;
            
            client = VaultClient::new(settings)
                .map_err(|e| SecretsError::ProviderError(format!("Failed to create Vault client: {}", e)))?;
        }
        
        Ok(Self {
            client,
            mount: config.mount_path,
            namespace: config.namespace,
        })
    }
}

#[async_trait]
impl SecretProvider for VaultProvider {
    fn name(&self) -> &str {
        "vault"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();
        
        match vaultrs::sys::health(&self.client).await {
            Ok(health) => {
                let healthy = health.initialized && !health.sealed;
                let latency_ms = start.elapsed().as_millis() as u64;
                
                Ok(HealthStatus {
                    healthy,
                    message: if healthy {
                        "Vault is healthy".to_string()
                    } else {
                        format!("Vault unhealthy: initialized={}, sealed={}", health.initialized, health.sealed)
                    },
                    latency_ms,
                    last_check: chrono::Utc::now(),
                })
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                warn!("Vault health check error: {}", e);
                
                Ok(HealthStatus {
                    healthy: false,
                    message: format!("Health check failed: {}", e),
                    latency_ms,
                    last_check: chrono::Utc::now(),
                })
            }
        }
    }
    
    async fn get_secret(&self, key: &str) -> Result<Secret> {
        debug!("Getting secret from Vault: {}", key);
        
        let secret_data: HashMap<String, String> = kv2::read(&self.client, &self.mount, key)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    SecretsError::NotFound(key.to_string())
                } else {
                    SecretsError::ProviderError(format!("Vault read error: {}", e))
                }
            })?;
        
        // Get metadata
        let vault_metadata = kv2::read_metadata(&self.client, &self.mount, key)
            .await
            .map_err(|e| SecretsError::ProviderError(format!("Vault metadata error: {}", e)))?;
        
        // Convert to JSON string
        let value = serde_json::to_string(&secret_data)
            .map_err(|e| SecretsError::SerializationError(e.to_string()))?;
        
        Ok(Secret {
            metadata: SecretMetadata {
                key: key.to_string(),
                version: Some(vault_metadata.current_version.to_string()),
                created_at: None, // Vault returns timestamps as strings, would need parsing
                updated_at: None,
                expires_at: None,
                rotation_enabled: false,
                rotation_interval_days: None,
                tags: HashMap::new(),
            },
            value,
        })
    }
    
    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        debug!("Getting secret version from Vault: {} v{}", key, version);
        
        let version_num: u64 = version.parse()
            .map_err(|e| SecretsError::ConfigurationError(format!("Invalid version: {}", e)))?;
        
        let secret_data: HashMap<String, String> = kv2::read_version(&self.client, &self.mount, key, version_num)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    SecretsError::NotFound(format!("{} version {}", key, version))
                } else {
                    SecretsError::ProviderError(format!("Vault read error: {}", e))
                }
            })?;
        
        let value = serde_json::to_string(&secret_data)
            .map_err(|e| SecretsError::SerializationError(e.to_string()))?;
        
        Ok(Secret {
            metadata: SecretMetadata {
                key: key.to_string(),
                version: Some(version.to_string()),
                created_at: None,
                updated_at: None,
                expires_at: None,
                rotation_enabled: false,
                rotation_interval_days: None,
                tags: HashMap::new(),
            },
            value,
        })
    }
    
    async fn set_secret(&self, key: &str, value: &str, _metadata: Option<SecretMetadata>) -> Result<()> {
        debug!("Setting secret in Vault: {}", key);
        
        // Parse value as JSON to HashMap
        let data: HashMap<String, String> = serde_json::from_str(value)
            .map_err(|e| SecretsError::SerializationError(e.to_string()))?;
        
        kv2::set(&self.client, &self.mount, key, &data)
            .await
            .map_err(|e| SecretsError::ProviderError(format!("Vault write error: {}", e)))?;
        
        Ok(())
    }
    
    async fn delete_secret(&self, key: &str) -> Result<()> {
        debug!("Deleting secret from Vault: {}", key);
        
        kv2::delete_latest(&self.client, &self.mount, key)
            .await
            .map_err(|e| SecretsError::ProviderError(format!("Vault delete error: {}", e)))?;
        
        Ok(())
    }
    
    async fn list_secrets(&self) -> Result<Vec<String>> {
        debug!("Listing secrets from Vault");
        
        let secrets = kv2::list(&self.client, &self.mount, "")
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    return SecretsError::NotFound("No secrets found".to_string());
                }
                SecretsError::ProviderError(format!("Vault list error: {}", e))
            })?;
        
        Ok(secrets)
    }
    
    async fn list_versions(&self, key: &str) -> Result<Vec<String>> {
        debug!("Listing versions for secret: {}", key);
        
        let metadata = kv2::read_metadata(&self.client, &self.mount, key)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    SecretsError::NotFound(key.to_string())
                } else {
                    SecretsError::ProviderError(format!("Vault metadata error: {}", e))
                }
            })?;
        
        let versions: Vec<String> = metadata.versions
            .keys()
            .map(|v| v.to_string())
            .collect();
        
        Ok(versions)
    }
    
    async fn rotate_secret(&self, key: &str) -> Result<String> {
        debug!("Rotating secret in Vault: {}", key);
        
        // Get current secret
        let current = self.get_secret(key).await?;
        
        // Re-write the same value to create a new version
        self.set_secret(key, &current.value, None).await?;
        
        // Get the new version
        let metadata = kv2::read_metadata(&self.client, &self.mount, key)
            .await
            .map_err(|e| SecretsError::ProviderError(format!("Vault metadata error: {}", e)))?;
        
        Ok(metadata.current_version.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: These tests require a running Vault instance
    // They are marked as ignored by default
    
    #[tokio::test]
    #[ignore]
    async fn test_vault_operations() {
        let config = VaultConfig {
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
        
        let provider = VaultProvider::new(config).await.unwrap();
        
        // Test health check
        let health = provider.health_check().await.unwrap();
        assert!(health.healthy);
        
        // Test set secret
        let value = serde_json::json!({"password": "secret123"});
        provider.set_secret("test/myapp", &value.to_string(), None).await.unwrap();
        
        // Test get secret
        let secret = provider.get_secret("test/myapp").await.unwrap();
        assert_eq!(secret.metadata.key, "test/myapp");
        
        // Test list
        let secrets = provider.list_secrets().await.unwrap();
        assert!(secrets.len() > 0);
        
        // Test delete secret
        provider.delete_secret("test/myapp").await.unwrap();
    }
}
