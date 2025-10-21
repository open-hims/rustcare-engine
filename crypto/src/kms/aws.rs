use super::traits::{
    AuditLogEntry, KeyManagementService, KeyMetadata, KeyOrigin, KeyRotationPolicy, KeyState,
    KeyUsage, KmsResult, OperationStatus,
};
use crate::error::CryptoError;
use async_trait::async_trait;
use aws_sdk_kms::types::{DataKeySpec, KeySpec, KeyUsageType};
use aws_sdk_kms::Client as KmsClient;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use zeroize::Zeroizing;

/// AWS KMS provider implementation
pub struct AwsKmsProvider {
    client: KmsClient,
    region: String,
}

impl AwsKmsProvider {
    /// Create a new AWS KMS provider
    /// 
    /// # Arguments
    /// * `client` - AWS KMS client
    /// * `region` - AWS region (e.g., "us-east-1")
    pub fn new(client: KmsClient, region: String) -> Self {
        Self { client, region }
    }

    /// Create from AWS config
    /// 
    /// # Arguments
    /// * `region` - AWS region
    pub async fn from_config(region: &str) -> KmsResult<Self> {
        let config = aws_config::from_env()
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = KmsClient::new(&config);

        Ok(Self::new(client, region.to_string()))
    }

    /// Convert AWS key state to our KeyState
    fn convert_key_state(state: &aws_sdk_kms::types::KeyState) -> KeyState {
        match state {
            aws_sdk_kms::types::KeyState::Enabled => KeyState::Enabled,
            aws_sdk_kms::types::KeyState::Disabled => KeyState::Disabled,
            aws_sdk_kms::types::KeyState::PendingDeletion => KeyState::PendingDeletion,
            aws_sdk_kms::types::KeyState::PendingImport => KeyState::PendingImport,
            aws_sdk_kms::types::KeyState::Unavailable => KeyState::Unavailable,
            _ => KeyState::Unavailable,
        }
    }

    /// Convert AWS key usage to our KeyUsage
    fn convert_key_usage(usage: &KeyUsageType) -> KeyUsage {
        match usage {
            KeyUsageType::EncryptDecrypt => KeyUsage::EncryptDecrypt,
            KeyUsageType::SignVerify => KeyUsage::SignVerify,
            _ => KeyUsage::EncryptDecrypt,
        }
    }

    /// Convert encryption context to AWS format
    fn to_aws_context(
        context: Option<&HashMap<String, String>>,
    ) -> Option<HashMap<String, String>> {
        context.map(|c| c.clone())
    }
}

#[async_trait]
impl KeyManagementService for AwsKmsProvider {
    async fn generate_data_key(
        &self,
        kek_id: &str,
        key_spec: &str,
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<(Zeroizing<Vec<u8>>, Vec<u8>)> {
        let spec = match key_spec {
            "AES_256" => DataKeySpec::Aes256,
            "AES_128" => DataKeySpec::Aes128,
            _ => {
                return Err(CryptoError::InvalidKey(format!(
                    "Unsupported key spec: {}",
                    key_spec
                )))
            }
        };

        let mut request = self.client.generate_data_key().key_id(kek_id).key_spec(spec);

        if let Some(ctx) = context {
            request = request.set_encryption_context(Some(ctx.clone()));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("AWS KMS error: {}", e)))?;

        let plaintext = response
            .plaintext()
            .ok_or_else(|| CryptoError::KeyDerivationFailed("No plaintext in response".to_string()))?
            .as_ref()
            .to_vec();

        let ciphertext = response
            .ciphertext_blob()
            .ok_or_else(|| CryptoError::KeyDerivationFailed("No ciphertext in response".to_string()))?
            .as_ref()
            .to_vec();

        Ok((Zeroizing::new(plaintext), ciphertext))
    }

    async fn decrypt_data_key(
        &self,
        encrypted_dek: &[u8],
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Zeroizing<Vec<u8>>> {
        let mut request = self
            .client
            .decrypt()
            .ciphertext_blob(aws_sdk_kms::primitives::Blob::new(encrypted_dek));

        if let Some(ctx) = context {
            request = request.set_encryption_context(Some(ctx.clone()));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptoError::DecryptionFailed(format!("AWS KMS decrypt error: {}", e)))?;

        let plaintext = response
            .plaintext()
            .ok_or_else(|| CryptoError::DecryptionFailed("No plaintext in response".to_string()))?
            .as_ref()
            .to_vec();

        Ok(Zeroizing::new(plaintext))
    }

    async fn encrypt(
        &self,
        key_id: &str,
        plaintext: &[u8],
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Vec<u8>> {
        if plaintext.len() > 4096 {
            return Err(CryptoError::EncryptionFailed(
                "AWS KMS encrypt supports max 4KB of data".to_string(),
            ));
        }

        let mut request = self
            .client
            .encrypt()
            .key_id(key_id)
            .plaintext(aws_sdk_kms::primitives::Blob::new(plaintext));

        if let Some(ctx) = context {
            request = request.set_encryption_context(Some(ctx.clone()));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptoError::EncryptionFailed(format!("AWS KMS error: {}", e)))?;

        let ciphertext = response
            .ciphertext_blob()
            .ok_or_else(|| CryptoError::EncryptionFailed("No ciphertext in response".to_string()))?
            .as_ref()
            .to_vec();

        Ok(ciphertext)
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Zeroizing<Vec<u8>>> {
        self.decrypt_data_key(ciphertext, context).await
    }

    async fn re_encrypt(
        &self,
        ciphertext: &[u8],
        new_key_id: &str,
        source_context: Option<&HashMap<String, String>>,
        dest_context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Vec<u8>> {
        let mut request = self
            .client
            .re_encrypt()
            .ciphertext_blob(aws_sdk_kms::primitives::Blob::new(ciphertext))
            .destination_key_id(new_key_id);

        if let Some(src_ctx) = source_context {
            request = request.set_source_encryption_context(Some(src_ctx.clone()));
        }

        if let Some(dest_ctx) = dest_context {
            request = request.set_destination_encryption_context(Some(dest_ctx.clone()));
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptoError::EncryptionFailed(format!("AWS KMS re-encrypt error: {}", e)))?;

        let new_ciphertext = response
            .ciphertext_blob()
            .ok_or_else(|| CryptoError::EncryptionFailed("No ciphertext in response".to_string()))?
            .as_ref()
            .to_vec();

        Ok(new_ciphertext)
    }

    async fn create_key(
        &self,
        description: &str,
        key_spec: &str,
        key_usage: KeyUsage,
        tags: Option<HashMap<String, String>>,
    ) -> KmsResult<KeyMetadata> {
        let spec = match key_spec {
            "SYMMETRIC_DEFAULT" => KeySpec::SymmetricDefault,
            "RSA_2048" => KeySpec::Rsa2048,
            "RSA_4096" => KeySpec::Rsa4096,
            _ => KeySpec::SymmetricDefault,
        };

        let usage = match key_usage {
            KeyUsage::EncryptDecrypt => KeyUsageType::EncryptDecrypt,
            KeyUsage::SignVerify => KeyUsageType::SignVerify,
        };

        let mut request = self
            .client
            .create_key()
            .description(description)
            .key_spec(spec)
            .key_usage(usage);

        if let Some(tag_map) = tags {
            for (key, value) in tag_map.iter() {
                request = request.tags(
                    aws_sdk_kms::types::Tag::builder()
                        .tag_key(key)
                        .tag_value(value)
                        .build()
                        .map_err(|e| CryptoError::InvalidKey(format!("Tag build error: {}", e)))?,
                );
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS create key error: {}", e)))?;

        let key_metadata = response
            .key_metadata()
            .ok_or_else(|| CryptoError::InvalidKey("No metadata in response".to_string()))?;

        Ok(KeyMetadata {
            key_id: key_metadata.key_id().unwrap_or_default().to_string(),
            alias: None,
            created_at: key_metadata
                .creation_date()
                .map(|d| DateTime::from_timestamp(d.secs(), 0).unwrap_or_else(Utc::now))
                .unwrap_or_else(Utc::now),
            state: Self::convert_key_state(key_metadata.key_state()),
            usage: Self::convert_key_usage(key_metadata.key_usage()),
            algorithm: format!("{:?}", key_metadata.key_spec()),
            origin: KeyOrigin::Kms,
            last_rotated: None,
            next_rotation: None,
            description: key_metadata.description().map(|s| s.to_string()),
            tags: HashMap::new(),
        })
    }

    async fn describe_key(&self, key_id: &str) -> KmsResult<KeyMetadata> {
        let response = self
            .client
            .describe_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS describe key error: {}", e)))?;

        let key_metadata = response
            .key_metadata()
            .ok_or_else(|| CryptoError::InvalidKey("No metadata in response".to_string()))?;

        Ok(KeyMetadata {
            key_id: key_metadata.key_id().unwrap_or_default().to_string(),
            alias: None,
            created_at: key_metadata
                .creation_date()
                .map(|d| DateTime::from_timestamp(d.secs(), 0).unwrap_or_else(Utc::now))
                .unwrap_or_else(Utc::now),
            state: Self::convert_key_state(key_metadata.key_state()),
            usage: Self::convert_key_usage(key_metadata.key_usage()),
            algorithm: format!("{:?}", key_metadata.key_spec()),
            origin: KeyOrigin::Kms,
            last_rotated: None,
            next_rotation: None,
            description: key_metadata.description().map(|s| s.to_string()),
            tags: HashMap::new(),
        })
    }

    async fn list_keys(&self, max_results: Option<u32>) -> KmsResult<Vec<KeyMetadata>> {
        let mut request = self.client.list_keys();

        if let Some(limit) = max_results {
            request = request.limit(limit as i32);
        }

        let response = request
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS list keys error: {}", e)))?;

        let mut keys = Vec::new();

        if let Some(key_list) = response.keys() {
            for key_entry in key_list {
                if let Some(key_id) = key_entry.key_id() {
                    match self.describe_key(key_id).await {
                        Ok(metadata) => keys.push(metadata),
                        Err(_) => continue, // Skip keys we can't access
                    }
                }
            }
        }

        Ok(keys)
    }

    async fn enable_key_rotation(
        &self,
        key_id: &str,
        _rotation_period_days: Option<u32>,
    ) -> KmsResult<()> {
        self.client
            .enable_key_rotation()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| {
                CryptoError::KeyDerivationFailed(format!("AWS KMS enable rotation error: {}", e))
            })?;

        Ok(())
    }

    async fn disable_key_rotation(&self, key_id: &str) -> KmsResult<()> {
        self.client
            .disable_key_rotation()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| {
                CryptoError::KeyDerivationFailed(format!("AWS KMS disable rotation error: {}", e))
            })?;

        Ok(())
    }

    async fn get_key_rotation_status(&self, key_id: &str) -> KmsResult<KeyRotationPolicy> {
        let response = self
            .client
            .get_key_rotation_status()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| {
                CryptoError::KeyDerivationFailed(format!("AWS KMS rotation status error: {}", e))
            })?;

        Ok(KeyRotationPolicy {
            enabled: response.key_rotation_enabled(),
            rotation_period_days: Some(365), // AWS KMS defaults to 365 days
            last_rotated: None,
            next_rotation: None,
        })
    }

    async fn rotate_key(&self, key_id: &str) -> KmsResult<KeyMetadata> {
        // AWS KMS doesn't support on-demand rotation via API
        // We simulate by enabling automatic rotation
        self.enable_key_rotation(key_id, Some(365)).await?;
        self.describe_key(key_id).await
    }

    async fn enable_key(&self, key_id: &str) -> KmsResult<()> {
        self.client
            .enable_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS enable key error: {}", e)))?;

        Ok(())
    }

    async fn disable_key(&self, key_id: &str) -> KmsResult<()> {
        self.client
            .disable_key()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS disable key error: {}", e)))?;

        Ok(())
    }

    async fn schedule_key_deletion(
        &self,
        key_id: &str,
        pending_window_days: u32,
    ) -> KmsResult<DateTime<Utc>> {
        let response = self
            .client
            .schedule_key_deletion()
            .key_id(key_id)
            .pending_window_in_days(pending_window_days as i32)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS schedule deletion error: {}", e)))?;

        let deletion_date = response
            .deletion_date()
            .map(|d| DateTime::from_timestamp(d.secs(), 0).unwrap_or_else(Utc::now))
            .unwrap_or_else(|| Utc::now() + chrono::Duration::days(pending_window_days as i64));

        Ok(deletion_date)
    }

    async fn cancel_key_deletion(&self, key_id: &str) -> KmsResult<()> {
        self.client
            .cancel_key_deletion()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS cancel deletion error: {}", e)))?;

        Ok(())
    }

    async fn create_alias(&self, alias_name: &str, key_id: &str) -> KmsResult<()> {
        let alias = if alias_name.starts_with("alias/") {
            alias_name.to_string()
        } else {
            format!("alias/{}", alias_name)
        };

        self.client
            .create_alias()
            .alias_name(alias)
            .target_key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS create alias error: {}", e)))?;

        Ok(())
    }

    async fn update_alias(&self, alias_name: &str, key_id: &str) -> KmsResult<()> {
        let alias = if alias_name.starts_with("alias/") {
            alias_name.to_string()
        } else {
            format!("alias/{}", alias_name)
        };

        self.client
            .update_alias()
            .alias_name(alias)
            .target_key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS update alias error: {}", e)))?;

        Ok(())
    }

    async fn delete_alias(&self, alias_name: &str) -> KmsResult<()> {
        let alias = if alias_name.starts_with("alias/") {
            alias_name.to_string()
        } else {
            format!("alias/{}", alias_name)
        };

        self.client
            .delete_alias()
            .alias_name(alias)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS delete alias error: {}", e)))?;

        Ok(())
    }

    async fn list_aliases(&self, key_id: &str) -> KmsResult<Vec<String>> {
        let response = self
            .client
            .list_aliases()
            .key_id(key_id)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("AWS KMS list aliases error: {}", e)))?;

        let mut aliases = Vec::new();

        if let Some(alias_list) = response.aliases() {
            for alias in alias_list {
                if let Some(name) = alias.alias_name() {
                    aliases.push(name.to_string());
                }
            }
        }

        Ok(aliases)
    }

    async fn get_key_audit_logs(
        &self,
        _key_id: &str,
        _start_time: DateTime<Utc>,
        _end_time: DateTime<Utc>,
    ) -> KmsResult<Vec<AuditLogEntry>> {
        // AWS KMS audit logs are available through CloudTrail, not KMS API
        // This would require integration with CloudTrail API
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_state_conversion() {
        let aws_state = aws_sdk_kms::types::KeyState::Enabled;
        let converted = AwsKmsProvider::convert_key_state(&aws_state);
        assert_eq!(converted, KeyState::Enabled);
    }

    #[test]
    fn test_key_usage_conversion() {
        let aws_usage = KeyUsageType::EncryptDecrypt;
        let converted = AwsKmsProvider::convert_key_usage(&aws_usage);
        assert_eq!(converted, KeyUsage::EncryptDecrypt);
    }
}
