use super::traits::{
    AuditLogEntry, KeyManagementService, KeyMetadata, KeyOrigin, KeyRotationPolicy, KeyState,
    KeyUsage, KmsResult, OperationStatus,
};
use crate::error::CryptoError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroizing;

/// HashiCorp Vault KMS provider
/// 
/// Integrates with Vault's Transit secrets engine for encryption as a service
pub struct VaultKmsProvider {
    client: reqwest::Client,
    vault_addr: String,
    vault_token: String,
    mount_path: String,
}

/// Vault encryption response
#[derive(Debug, Deserialize)]
struct VaultEncryptResponse {
    data: VaultEncryptData,
}

#[derive(Debug, Deserialize)]
struct VaultEncryptData {
    ciphertext: String,
}

/// Vault decryption response
#[derive(Debug, Deserialize)]
struct VaultDecryptResponse {
    data: VaultDecryptData,
}

#[derive(Debug, Deserialize)]
struct VaultDecryptData {
    plaintext: String,
}

/// Vault data key response
#[derive(Debug, Deserialize)]
struct VaultDataKeyResponse {
    data: VaultDataKeyData,
}

#[derive(Debug, Deserialize)]
struct VaultDataKeyData {
    plaintext: String,
    ciphertext: String,
}

/// Vault key info response
#[derive(Debug, Deserialize)]
struct VaultKeyInfoResponse {
    data: VaultKeyInfo,
}

#[derive(Debug, Deserialize)]
struct VaultKeyInfo {
    #[serde(rename = "type")]
    key_type: String,
    deletion_allowed: bool,
    exportable: bool,
    #[serde(default)]
    keys: HashMap<String, i64>,
    latest_version: i32,
    min_decryption_version: i32,
    min_encryption_version: i32,
    supports_decryption: bool,
    supports_encryption: bool,
    supports_signing: bool,
}

impl VaultKmsProvider {
    /// Create a new Vault KMS provider
    /// 
    /// # Arguments
    /// * `vault_addr` - Vault server address (e.g., "https://vault.example.com:8200")
    /// * `vault_token` - Vault authentication token
    /// * `mount_path` - Transit engine mount path (default: "transit")
    pub fn new(vault_addr: String, vault_token: String, mount_path: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            vault_addr: vault_addr.trim_end_matches('/').to_string(),
            vault_token,
            mount_path: mount_path.unwrap_or_else(|| "transit".to_string()),
        }
    }

    /// Get full URL for an endpoint
    fn url(&self, path: &str) -> String {
        format!("{}/v1/{}/{}", self.vault_addr, self.mount_path, path.trim_start_matches('/'))
    }

    /// Decode base64 data
    fn decode_base64(&self, data: &str) -> Result<Vec<u8>, CryptoError> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD
            .decode(data)
            .map_err(|e| CryptoError::DecryptionFailed(format!("Base64 decode error: {}", e)))
    }

    /// Encode data to base64
    fn encode_base64(&self, data: &[u8]) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD.encode(data)
    }
}

#[async_trait]
impl KeyManagementService for VaultKmsProvider {
    async fn generate_data_key(
        &self,
        kek_id: &str,
        key_spec: &str,
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<(Zeroizing<Vec<u8>>, Vec<u8>)> {
        let bits = match key_spec {
            "AES_256" => 256,
            "AES_128" => 128,
            _ => return Err(CryptoError::InvalidKey(format!("Unsupported key spec: {}", key_spec))),
        };

        let url = self.url(&format!("datakey/plaintext/{}", kek_id));
        
        let mut payload = serde_json::json!({
            "bits": bits,
        });

        if let Some(ctx) = context {
            payload["context"] = serde_json::to_value(ctx)
                .map_err(|e| CryptoError::InvalidKey(format!("Context serialization error: {}", e)))?;
        }

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CryptoError::KeyDerivationFailed(format!(
                "Vault error {}: {}",
                status, error_text
            )));
        }

        let vault_response: VaultDataKeyResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("Response parse error: {}", e)))?;

        let plaintext = self.decode_base64(&vault_response.data.plaintext)?;
        let ciphertext = vault_response.data.ciphertext.into_bytes();

        Ok((Zeroizing::new(plaintext), ciphertext))
    }

    async fn decrypt_data_key(
        &self,
        encrypted_dek: &[u8],
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Zeroizing<Vec<u8>>> {
        let ciphertext_str = String::from_utf8(encrypted_dek.to_vec())
            .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid ciphertext encoding: {}", e)))?;

        // Extract key name from ciphertext (format: "vault:v1:key_name:...")
        let parts: Vec<&str> = ciphertext_str.split(':').collect();
        if parts.len() < 3 || parts[0] != "vault" {
            return Err(CryptoError::DecryptionFailed("Invalid Vault ciphertext format".to_string()));
        }

        let url = self.url("decrypt/data");
        
        let mut payload = serde_json::json!({
            "ciphertext": ciphertext_str,
        });

        if let Some(ctx) = context {
            payload["context"] = serde_json::to_value(ctx)
                .map_err(|e| CryptoError::InvalidKey(format!("Context serialization error: {}", e)))?;
        }

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::DecryptionFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CryptoError::DecryptionFailed(format!(
                "Vault error {}: {}",
                status, error_text
            )));
        }

        let vault_response: VaultDecryptResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::DecryptionFailed(format!("Response parse error: {}", e)))?;

        let plaintext = self.decode_base64(&vault_response.data.plaintext)?;

        Ok(Zeroizing::new(plaintext))
    }

    async fn encrypt(
        &self,
        key_id: &str,
        plaintext: &[u8],
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Vec<u8>> {
        let url = self.url(&format!("encrypt/{}", key_id));
        
        let plaintext_b64 = self.encode_base64(plaintext);
        
        let mut payload = serde_json::json!({
            "plaintext": plaintext_b64,
        });

        if let Some(ctx) = context {
            let context_b64 = self.encode_base64(serde_json::to_string(ctx)
                .map_err(|e| CryptoError::InvalidKey(format!("Context serialization error: {}", e)))?
                .as_bytes());
            payload["context"] = serde_json::Value::String(context_b64);
        }

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::EncryptionFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CryptoError::EncryptionFailed(format!(
                "Vault error {}: {}",
                status, error_text
            )));
        }

        let vault_response: VaultEncryptResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::EncryptionFailed(format!("Response parse error: {}", e)))?;

        Ok(vault_response.data.ciphertext.into_bytes())
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Zeroizing<Vec<u8>>> {
        let ciphertext_str = String::from_utf8(ciphertext.to_vec())
            .map_err(|e| CryptoError::DecryptionFailed(format!("Invalid ciphertext encoding: {}", e)))?;

        // Extract key name from ciphertext
        let parts: Vec<&str> = ciphertext_str.split(':').collect();
        if parts.len() < 3 || parts[0] != "vault" {
            return Err(CryptoError::DecryptionFailed("Invalid Vault ciphertext format".to_string()));
        }
        
        let key_name = parts[2];
        let url = self.url(&format!("decrypt/{}", key_name));
        
        let mut payload = serde_json::json!({
            "ciphertext": ciphertext_str,
        });

        if let Some(ctx) = context {
            let context_b64 = self.encode_base64(serde_json::to_string(ctx)
                .map_err(|e| CryptoError::InvalidKey(format!("Context serialization error: {}", e)))?
                .as_bytes());
            payload["context"] = serde_json::Value::String(context_b64);
        }

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::DecryptionFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CryptoError::DecryptionFailed(format!(
                "Vault error {}: {}",
                status, error_text
            )));
        }

        let vault_response: VaultDecryptResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::DecryptionFailed(format!("Response parse error: {}", e)))?;

        let plaintext = self.decode_base64(&vault_response.data.plaintext)?;

        Ok(Zeroizing::new(plaintext))
    }

    async fn re_encrypt(
        &self,
        ciphertext: &[u8],
        new_key_id: &str,
        _source_context: Option<&HashMap<String, String>>,
        dest_context: Option<&HashMap<String, String>>,
    ) -> KmsResult<Vec<u8>> {
        let ciphertext_str = String::from_utf8(ciphertext.to_vec())
            .map_err(|e| CryptoError::EncryptionFailed(format!("Invalid ciphertext encoding: {}", e)))?;

        let url = self.url(&format!("rewrap/{}", new_key_id));
        
        let mut payload = serde_json::json!({
            "ciphertext": ciphertext_str,
        });

        if let Some(ctx) = dest_context {
            let context_b64 = self.encode_base64(serde_json::to_string(ctx)
                .map_err(|e| CryptoError::InvalidKey(format!("Context serialization error: {}", e)))?
                .as_bytes());
            payload["context"] = serde_json::Value::String(context_b64);
        }

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::EncryptionFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(CryptoError::EncryptionFailed(format!(
                "Vault error {}: {}",
                status, error_text
            )));
        }

        let vault_response: VaultEncryptResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::EncryptionFailed(format!("Response parse error: {}", e)))?;

        Ok(vault_response.data.ciphertext.into_bytes())
    }

    async fn create_key(
        &self,
        _description: &str,
        key_spec: &str,
        _key_usage: KeyUsage,
        _tags: Option<HashMap<String, String>>,
    ) -> KmsResult<KeyMetadata> {
        // Vault Transit keys are created when first used
        // This is a no-op that returns metadata for a new key
        Ok(KeyMetadata {
            key_id: key_spec.to_string(),
            alias: Some(key_spec.to_string()),
            created_at: Utc::now(),
            state: KeyState::Enabled,
            usage: KeyUsage::EncryptDecrypt,
            algorithm: "AES256-GCM96".to_string(),
            origin: KeyOrigin::Kms,
            last_rotated: None,
            next_rotation: None,
            description: Some("Vault Transit key".to_string()),
            tags: HashMap::new(),
        })
    }

    async fn describe_key(&self, key_id: &str) -> KmsResult<KeyMetadata> {
        let url = self.url(&format!("keys/{}", key_id));

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", &self.vault_token)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CryptoError::InvalidKey(format!("Vault error: {}", status)));
        }

        let vault_response: VaultKeyInfoResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("Response parse error: {}", e)))?;

        let created_at = vault_response
            .data
            .keys
            .values()
            .min()
            .map(|&timestamp| DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now))
            .unwrap_or_else(Utc::now);

        Ok(KeyMetadata {
            key_id: key_id.to_string(),
            alias: Some(key_id.to_string()),
            created_at,
            state: if vault_response.data.deletion_allowed {
                KeyState::Enabled
            } else {
                KeyState::Disabled
            },
            usage: if vault_response.data.supports_encryption {
                KeyUsage::EncryptDecrypt
            } else {
                KeyUsage::SignVerify
            },
            algorithm: vault_response.data.key_type,
            origin: KeyOrigin::Kms,
            last_rotated: None,
            next_rotation: None,
            description: None,
            tags: HashMap::new(),
        })
    }

    async fn list_keys(&self, _max_results: Option<u32>) -> KmsResult<Vec<KeyMetadata>> {
        let url = self.url("keys?list=true");

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", &self.vault_token)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        #[derive(Deserialize)]
        struct ListResponse {
            data: ListData,
        }

        #[derive(Deserialize)]
        struct ListData {
            keys: Vec<String>,
        }

        let list_response: ListResponse = response
            .json()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("Response parse error: {}", e)))?;

        let mut keys = Vec::new();
        for key_name in list_response.data.keys {
            if let Ok(metadata) = self.describe_key(&key_name).await {
                keys.push(metadata);
            }
        }

        Ok(keys)
    }

    async fn enable_key_rotation(&self, key_id: &str, _rotation_period_days: Option<u32>) -> KmsResult<()> {
        let url = self.url(&format!("keys/{}/config", key_id));

        let payload = serde_json::json!({
            "auto_rotate_period": "8760h", // 365 days
        });

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CryptoError::KeyDerivationFailed(format!("Vault error: {}", status)));
        }

        Ok(())
    }

    async fn disable_key_rotation(&self, key_id: &str) -> KmsResult<()> {
        let url = self.url(&format!("keys/{}/config", key_id));

        let payload = serde_json::json!({
            "auto_rotate_period": "0",
        });

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CryptoError::KeyDerivationFailed(format!("Vault error: {}", status)));
        }

        Ok(())
    }

    async fn get_key_rotation_status(&self, _key_id: &str) -> KmsResult<KeyRotationPolicy> {
        // Vault doesn't expose rotation status easily via API
        Ok(KeyRotationPolicy {
            enabled: false,
            rotation_period_days: Some(365),
            last_rotated: None,
            next_rotation: None,
        })
    }

    async fn rotate_key(&self, key_id: &str) -> KmsResult<KeyMetadata> {
        let url = self.url(&format!("keys/{}/rotate", key_id));

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.vault_token)
            .send()
            .await
            .map_err(|e| CryptoError::KeyDerivationFailed(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CryptoError::KeyDerivationFailed(format!("Vault error: {}", status)));
        }

        self.describe_key(key_id).await
    }

    async fn enable_key(&self, _key_id: &str) -> KmsResult<()> {
        // Vault keys are always enabled unless deleted
        Ok(())
    }

    async fn disable_key(&self, _key_id: &str) -> KmsResult<()> {
        // Vault doesn't support disabling keys
        Err(CryptoError::InvalidKey("Vault doesn't support disabling keys".to_string()))
    }

    async fn schedule_key_deletion(&self, key_id: &str, _pending_window_days: u32) -> KmsResult<DateTime<Utc>> {
        let url = self.url(&format!("keys/{}", key_id));

        let response = self
            .client
            .delete(&url)
            .header("X-Vault-Token", &self.vault_token)
            .send()
            .await
            .map_err(|e| CryptoError::InvalidKey(format!("Vault request error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CryptoError::InvalidKey(format!("Vault error: {}", status)));
        }

        Ok(Utc::now())
    }

    async fn cancel_key_deletion(&self, _key_id: &str) -> KmsResult<()> {
        // Vault deletes keys immediately
        Err(CryptoError::InvalidKey("Vault deletes keys immediately, cannot cancel".to_string()))
    }

    async fn create_alias(&self, _alias_name: &str, _key_id: &str) -> KmsResult<()> {
        // Vault Transit doesn't support aliases
        Ok(())
    }

    async fn update_alias(&self, _alias_name: &str, _key_id: &str) -> KmsResult<()> {
        Ok(())
    }

    async fn delete_alias(&self, _alias_name: &str) -> KmsResult<()> {
        Ok(())
    }

    async fn list_aliases(&self, _key_id: &str) -> KmsResult<Vec<String>> {
        Ok(Vec::new())
    }

    async fn get_key_audit_logs(
        &self,
        _key_id: &str,
        _start_time: DateTime<Utc>,
        _end_time: DateTime<Utc>,
    ) -> KmsResult<Vec<AuditLogEntry>> {
        // Vault audit logs would require separate audit device configuration
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_provider_creation() {
        let provider = VaultKmsProvider::new(
            "https://vault.example.com:8200".to_string(),
            "test-token".to_string(),
            Some("transit".to_string()),
        );

        assert_eq!(provider.vault_addr, "https://vault.example.com:8200");
        assert_eq!(provider.vault_token, "test-token");
        assert_eq!(provider.mount_path, "transit");
    }

    #[test]
    fn test_url_generation() {
        let provider = VaultKmsProvider::new(
            "https://vault.example.com:8200".to_string(),
            "token".to_string(),
            None,
        );

        let url = provider.url("encrypt/my-key");
        assert_eq!(url, "https://vault.example.com:8200/v1/transit/encrypt/my-key");
    }
}
