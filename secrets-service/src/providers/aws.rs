//! AWS Secrets Manager provider implementation

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::Client;
use crate::{SecretProvider, Secret, SecretMetadata, Result, SecretsError, HealthStatus};
use crate::config::AwsSecretsManagerConfig;
use std::collections::HashMap;
use tracing::{debug, warn};

pub struct AwsSecretsManagerProvider {
    client: Client,
}

impl AwsSecretsManagerProvider {
    pub async fn new(config: AwsSecretsManagerConfig) -> Result<Self> {
        let mut aws_config = aws_config::defaults(BehaviorVersion::latest());
        
        aws_config = aws_config.region(
            aws_sdk_secretsmanager::config::Region::new(config.region.clone())
        );
        
        let sdk_config = aws_config.load().await;
        let client = Client::new(&sdk_config);
        
        Ok(Self { client })
    }
}

#[async_trait]
impl SecretProvider for AwsSecretsManagerProvider {
    fn name(&self) -> &str {
        "aws-secrets-manager"
    }
    
    async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();
        
        // Try to list secrets to check connectivity
        match self.client.list_secrets()
            .max_results(1)
            .send()
            .await
        {
            Ok(_) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                Ok(HealthStatus {
                    healthy: true,
                    message: "AWS Secrets Manager is healthy".to_string(),
                    latency_ms,
                    last_check: chrono::Utc::now(),
                })
            }
            Err(e) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                warn!("AWS Secrets Manager health check failed: {}", e);
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
        debug!("Getting secret from AWS Secrets Manager: {}", key);
        
        let response = self.client.get_secret_value()
            .secret_id(key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ResourceNotFoundException") {
                    SecretsError::NotFound(key.to_string())
                } else {
                    SecretsError::ProviderError(format!("AWS Secrets Manager error: {}", e))
                }
            })?;
        
        let value = response.secret_string()
            .ok_or_else(|| SecretsError::ProviderError("Secret has no string value".to_string()))?
            .to_string();
        
        Ok(Secret {
            metadata: SecretMetadata {
                key: key.to_string(),
                version: response.version_id().map(|s| s.to_string()),
                created_at: response.created_date()
                    .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs_f64() as i64, 0)),
                updated_at: Some(chrono::Utc::now()),
                expires_at: None,
                rotation_enabled: false,
                rotation_interval_days: None,
                tags: HashMap::new(),
            },
            value,
        })
    }
    
    async fn get_secret_version(&self, key: &str, version: &str) -> Result<Secret> {
        debug!("Getting secret version from AWS Secrets Manager: {} v{}", key, version);
        
        let response = self.client.get_secret_value()
            .secret_id(key)
            .version_id(version)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ResourceNotFoundException") {
                    SecretsError::NotFound(format!("{} version {}", key, version))
                } else {
                    SecretsError::ProviderError(format!("AWS Secrets Manager error: {}", e))
                }
            })?;
        
        let value = response.secret_string()
            .ok_or_else(|| SecretsError::ProviderError("Secret has no string value".to_string()))?
            .to_string();
        
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
        debug!("Setting secret in AWS Secrets Manager: {}", key);
        
        // Try to update first, if it doesn't exist, create it
        match self.client.update_secret()
            .secret_id(key)
            .secret_string(value)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("ResourceNotFoundException") => {
                // Secret doesn't exist, create it
                self.client.create_secret()
                    .name(key)
                    .secret_string(value)
                    .send()
                    .await
                    .map_err(|e| SecretsError::ProviderError(format!("AWS create secret error: {}", e)))?;
                
                Ok(())
            }
            Err(e) => Err(SecretsError::ProviderError(format!("AWS update secret error: {}", e)))
        }
    }
    
    async fn delete_secret(&self, key: &str) -> Result<()> {
        debug!("Deleting secret from AWS Secrets Manager: {}", key);
        
        self.client.delete_secret()
            .secret_id(key)
            .force_delete_without_recovery(true)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ResourceNotFoundException") {
                    SecretsError::NotFound(key.to_string())
                } else {
                    SecretsError::ProviderError(format!("AWS delete secret error: {}", e))
                }
            })?;
        
        Ok(())
    }
    
    async fn list_secrets(&self) -> Result<Vec<String>> {
        debug!("Listing secrets from AWS Secrets Manager");
        
        let response = self.client.list_secrets()
            .send()
            .await
            .map_err(|e| SecretsError::ProviderError(format!("AWS list secrets error: {}", e)))?;
        
        let mut secrets = Vec::new();
        for secret in response.secret_list() {
            if let Some(name) = secret.name() {
                secrets.push(name.to_string());
            }
        }
        
        Ok(secrets)
    }
    
    async fn list_versions(&self, key: &str) -> Result<Vec<String>> {
        debug!("Listing versions for secret: {}", key);
        
        let response = self.client.list_secret_version_ids()
            .secret_id(key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ResourceNotFoundException") {
                    SecretsError::NotFound(key.to_string())
                } else {
                    SecretsError::ProviderError(format!("AWS list versions error: {}", e))
                }
            })?;
        
        let mut versions = Vec::new();
        for version_info in response.versions() {
            if let Some(version_id) = version_info.version_id() {
                versions.push(version_id.to_string());
            }
        }
        
        Ok(versions)
    }
    
    async fn rotate_secret(&self, key: &str) -> Result<String> {
        debug!("Rotating secret in AWS Secrets Manager: {}", key);
        
        let response = self.client.rotate_secret()
            .secret_id(key)
            .send()
            .await
            .map_err(|e| SecretsError::ProviderError(format!("AWS rotate secret error: {}", e)))?;
        
        Ok(response.version_id().unwrap_or("").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Note: These tests require AWS credentials and proper permissions
    // They are marked as ignored by default
    
    #[tokio::test]
    #[ignore]
    async fn test_aws_operations() {
        let config = AwsSecretsManagerConfig {
            region: "us-east-1".to_string(),
            role_arn: None,
            external_id: None,
            endpoint_url: None,
        };
        
        let provider = AwsSecretsManagerProvider::new(config).await.unwrap();
        
        // Test health check
        let health = provider.health_check().await.unwrap();
        assert!(health.healthy);
        
        // Test set secret
        let value = serde_json::json!({"password": "secret123"});
        provider.set_secret("test/myapp", &value.to_string(), None).await.unwrap();
        
        // Test get secret
        let secret = provider.get_secret("test/myapp").await.unwrap();
        assert_eq!(secret.metadata.key, "test/myapp");
        
        // Test delete secret
        provider.delete_secret("test/myapp").await.unwrap();
    }
}
