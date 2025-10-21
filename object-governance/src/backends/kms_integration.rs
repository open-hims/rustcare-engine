//! KMS-integrated storage backends
//!
//! Enhances storage backends with Key Management Service integration:
//! - Generate DEKs (Data Encryption Keys) via KMS
//! - Store encrypted DEK alongside encrypted data
//! - Support key rotation via KMS re-encryption
//! - Cache DEKs for performance

use crate::error::{GovernanceError, GovernanceResult};
use async_trait::async_trait;
use crypto::kms::KeyManagementService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

/// Metadata for KMS-encrypted objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmsEncryptedMetadata {
    /// KMS provider type (aws_kms, vault, local)
    pub kms_provider: String,
    
    /// KMS key ID or ARN used to encrypt the DEK
    pub kms_key_id: String,
    
    /// Encrypted Data Encryption Key (encrypted by KMS)
    pub encrypted_dek: Vec<u8>,
    
    /// DEK encryption context (for AWS KMS)
    pub encryption_context: Option<HashMap<String, String>>,
    
    /// Algorithm used for data encryption
    pub data_encryption_algorithm: String,
    
    /// Key version (for rotation tracking)
    pub key_version: u32,
    
    /// Timestamp when DEK was generated
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Cached DEK entry
struct CachedDek {
    /// Plaintext DEK
    dek: Vec<u8>,
    
    /// When this entry was cached
    cached_at: Instant,
    
    /// TTL for this entry
    ttl: Duration,
}

impl CachedDek {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// DEK cache for performance optimization
pub struct DekCache {
    cache: Arc<RwLock<HashMap<String, CachedDek>>>,
    max_size: usize,
    default_ttl: Duration,
}

impl DekCache {
    /// Create a new DEK cache
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            default_ttl,
        }
    }

    /// Get a DEK from cache
    pub async fn get(&self, cache_key: &str) -> Option<Vec<u8>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(cache_key) {
            if !entry.is_expired() {
                return Some(entry.dek.clone());
            }
        }
        None
    }

    /// Put a DEK in cache
    pub async fn put(&self, cache_key: String, dek: Vec<u8>) {
        let mut cache = self.cache.write().await;
        
        // Evict old entries if cache is full
        if cache.len() >= self.max_size {
            // Simple LRU: remove oldest expired entry, or any random entry
            let to_remove: Vec<String> = cache
                .iter()
                .filter(|(_, v)| v.is_expired())
                .map(|(k, _)| k.clone())
                .take(1)
                .collect();
            
            for key in to_remove {
                cache.remove(&key);
            }
            
            // If still full, remove any entry
            if cache.len() >= self.max_size {
                if let Some(key) = cache.keys().next().cloned() {
                    cache.remove(&key);
                }
            }
        }
        
        cache.insert(
            cache_key,
            CachedDek {
                dek,
                cached_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Clear all cached DEKs
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let expired = cache.values().filter(|v| v.is_expired()).count();
        (total, expired)
    }
}

/// KMS-aware storage operations
#[async_trait]
pub trait KmsStorageBackend {
    /// Generate a new DEK using KMS
    /// 
    /// Returns (plaintext_dek, encrypted_dek, kms_metadata)
    async fn generate_dek(
        &self,
        object_key: &str,
    ) -> GovernanceResult<(Vec<u8>, Vec<u8>, KmsEncryptedMetadata)>;
    
    /// Decrypt a DEK using KMS
    /// 
    /// Returns plaintext DEK
    async fn decrypt_dek(
        &self,
        encrypted_dek: &[u8],
        kms_metadata: &KmsEncryptedMetadata,
    ) -> GovernanceResult<Vec<u8>>;
    
    /// Re-encrypt a DEK with a new KMS key (for key rotation)
    async fn rotate_dek(
        &self,
        encrypted_dek: &[u8],
        old_key_id: &str,
        new_key_id: &str,
    ) -> GovernanceResult<Vec<u8>>;
}

/// KMS integration for storage backends
pub struct KmsIntegration {
    /// KMS provider
    kms_provider: Arc<dyn KeyManagementService + Send + Sync>,
    
    /// KMS key ID (or ARN for AWS)
    kms_key_id: String,
    
    /// DEK cache (optional)
    dek_cache: Option<Arc<DekCache>>,
    
    /// Current key version
    key_version: u32,
}

impl KmsIntegration {
    /// Create a new KMS integration
    pub fn new(
        kms_provider: Arc<dyn KeyManagementService + Send + Sync>,
        kms_key_id: String,
        key_version: u32,
    ) -> Self {
        Self {
            kms_provider,
            kms_key_id,
            dek_cache: None,
            key_version,
        }
    }

    /// Enable DEK caching
    pub fn with_cache(mut self, max_size: usize, ttl: Duration) -> Self {
        self.dek_cache = Some(Arc::new(DekCache::new(max_size, ttl)));
        self
    }

    /// Get KMS provider type
    fn get_provider_type(&self) -> &'static str {
        // TODO: Get this from KMS provider trait
        "unknown"
    }

    /// Generate cache key for a DEK
    fn cache_key(&self, object_key: &str, encrypted_dek: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(object_key.as_bytes());
        hasher.update(encrypted_dek);
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl KmsStorageBackend for KmsIntegration {
    async fn generate_dek(
        &self,
        object_key: &str,
    ) -> GovernanceResult<(Vec<u8>, Vec<u8>, KmsEncryptedMetadata)> {
        // Build encryption context for AWS KMS
        let mut encryption_context = HashMap::new();
        encryption_context.insert("object_key".to_string(), object_key.to_string());
        encryption_context.insert("key_version".to_string(), self.key_version.to_string());

        // Generate DEK via KMS
        let (plaintext_dek, encrypted_dek) = self
            .kms_provider
            .generate_data_key(&self.kms_key_id, "AES_256", Some(&encryption_context))
            .await
            .map_err(|e| GovernanceError::Storage(format!("KMS generate_data_key failed: {}", e)))?;

        let metadata = KmsEncryptedMetadata {
            kms_provider: self.get_provider_type().to_string(),
            kms_key_id: self.kms_key_id.clone(),
            encrypted_dek: encrypted_dek.clone(),
            encryption_context: Some(encryption_context),
            data_encryption_algorithm: "AES-256-GCM".to_string(),
            key_version: self.key_version,
            created_at: chrono::Utc::now(),
        };

        // Cache the plaintext DEK
        if let Some(cache) = &self.dek_cache {
            let cache_key = self.cache_key(object_key, &encrypted_dek);
            cache.put(cache_key, plaintext_dek.to_vec()).await;
        }

        Ok((plaintext_dek.to_vec(), encrypted_dek, metadata))
    }

    async fn decrypt_dek(
        &self,
        encrypted_dek: &[u8],
        kms_metadata: &KmsEncryptedMetadata,
    ) -> GovernanceResult<Vec<u8>> {
        // Check cache first
        if let Some(cache) = &self.dek_cache {
            let cache_key = self.cache_key(
                kms_metadata.encryption_context
                    .as_ref()
                    .and_then(|ctx| ctx.get("object_key"))
                    .map(|s| s.as_str())
                    .unwrap_or(""),
                encrypted_dek,
            );
            
            if let Some(cached_dek) = cache.get(&cache_key).await {
                return Ok(cached_dek);
            }
        }

        // Decrypt via KMS
        let plaintext_dek = self
            .kms_provider
            .decrypt_data_key(
                encrypted_dek,
                kms_metadata.encryption_context.as_ref(),
            )
            .await
            .map_err(|e| GovernanceError::Storage(format!("KMS decrypt failed: {}", e)))?;

        // Cache the result
        if let Some(cache) = &self.dek_cache {
            let cache_key = self.cache_key(
                kms_metadata.encryption_context
                    .as_ref()
                    .and_then(|ctx| ctx.get("object_key"))
                    .map(|s| s.as_str())
                    .unwrap_or(""),
                encrypted_dek,
            );
            cache.put(cache_key, plaintext_dek.to_vec()).await;
        }

        Ok(plaintext_dek.to_vec())
    }

    async fn rotate_dek(
        &self,
        encrypted_dek: &[u8],
        old_key_id: &str,
        new_key_id: &str,
    ) -> GovernanceResult<Vec<u8>> {
        // Re-encrypt DEK with new key
        let new_encrypted_dek = self
            .kms_provider
            .re_encrypt(encrypted_dek, new_key_id, None, None)
            .await
            .map_err(|e| GovernanceError::Storage(format!("KMS re_encrypt failed: {}", e)))?;

        // Invalidate cache (key has changed)
        if let Some(cache) = &self.dek_cache {
            cache.clear().await;
        }

        Ok(new_encrypted_dek)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_dek_cache() {
        let cache = DekCache::new(2, Duration::from_millis(100));
        
        // Put entries
        cache.put("key1".to_string(), vec![1, 2, 3]).await;
        cache.put("key2".to_string(), vec![4, 5, 6]).await;
        
        // Retrieve
        assert_eq!(cache.get("key1").await, Some(vec![1, 2, 3]));
        assert_eq!(cache.get("key2").await, Some(vec![4, 5, 6]));
        
        // Test expiration
        sleep(Duration::from_millis(150)).await;
        assert_eq!(cache.get("key1").await, None);
        
        // Test eviction (cache size = 2)
        cache.put("key3".to_string(), vec![7, 8, 9]).await;
        cache.put("key4".to_string(), vec![10, 11, 12]).await;
        cache.put("key5".to_string(), vec![13, 14, 15]).await;
        
        let (total, _) = cache.stats().await;
        assert_eq!(total, 2); // Cache should only have 2 entries
    }

    #[test]
    fn test_cached_dek_expiration() {
        let entry = CachedDek {
            dek: vec![1, 2, 3],
            cached_at: Instant::now() - Duration::from_secs(10),
            ttl: Duration::from_secs(5),
        };
        assert!(entry.is_expired());

        let fresh_entry = CachedDek {
            dek: vec![1, 2, 3],
            cached_at: Instant::now(),
            ttl: Duration::from_secs(5),
        };
        assert!(!fresh_entry.is_expired());
    }

    #[test]
    fn test_kms_metadata_serialization() {
        let metadata = KmsEncryptedMetadata {
            kms_provider: "aws_kms".to_string(),
            kms_key_id: "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012".to_string(),
            encrypted_dek: vec![1, 2, 3, 4],
            encryption_context: Some([
                ("object_key".to_string(), "test/object".to_string()),
                ("key_version".to_string(), "1".to_string()),
            ].iter().cloned().collect()),
            data_encryption_algorithm: "AES-256-GCM".to_string(),
            key_version: 1,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: KmsEncryptedMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(metadata.kms_provider, deserialized.kms_provider);
        assert_eq!(metadata.kms_key_id, deserialized.kms_key_id);
        assert_eq!(metadata.encrypted_dek, deserialized.encrypted_dek);
    }
}
