use crate::error::{GovernanceError, GovernanceResult};
use crate::storage::{AccessLog, ObjectMetadata, ObjectVersion, StorageBackend};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use crypto::aes_gcm::{Aes256GcmEncryptor, KeyGenerator};
use crypto::encryption::Encryptor;
use crypto::envelope::{EnvelopeEncryption, EnvelopeMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Minimum size for envelope encryption (5MB - AWS multipart threshold)
const ENVELOPE_THRESHOLD: usize = 5 * 1024 * 1024;

/// Chunk size for envelope encryption (5MB for S3 multipart)
const CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// S3 storage backend with client-side encryption
pub struct S3Backend {
    /// S3 client
    client: S3Client,
    /// Bucket name
    bucket: String,
    /// Optional prefix for all keys
    prefix: Option<String>,
    /// KEK (Key Encryption Key) for envelope encryption
    kek: [u8; 32],
    /// Direct encryptor for small files
    encryptor: Aes256GcmEncryptor,
}

/// Encrypted object metadata stored as S3 object tags
#[derive(Debug, Clone, Serialize, Deserialize)]
struct S3EncryptedMetadata {
    /// Original metadata
    pub metadata: ObjectMetadata,
    /// Encryption algorithm used
    pub encryption_algorithm: String,
    /// Envelope metadata (for large objects)
    pub envelope_metadata: Option<String>, // JSON-serialized EnvelopeMetadata
    /// Whether data is stored in multipart
    pub is_multipart: bool,
}

impl S3Backend {
    /// Create a new S3 backend with client-side encryption
    /// 
    /// # Arguments
    /// * `client` - AWS S3 client
    /// * `bucket` - S3 bucket name
    /// * `kek` - 32-byte Key Encryption Key for envelope encryption
    pub fn new(client: S3Client, bucket: String, kek: [u8; 32]) -> GovernanceResult<Self> {
        let encryptor = Aes256GcmEncryptor::new(kek)
            .map_err(|e| GovernanceError::Storage(format!("Failed to create encryptor: {}", e)))?;

        Ok(Self {
            client,
            bucket,
            prefix: None,
            kek,
            encryptor,
        })
    }

    /// Create a new S3 backend from AWS config
    /// 
    /// # Arguments
    /// * `bucket` - S3 bucket name
    /// * `region` - AWS region (e.g., "us-east-1")
    /// * `kek` - 32-byte Key Encryption Key
    pub async fn from_config(
        bucket: String,
        region: &str,
        kek: [u8; 32],
    ) -> GovernanceResult<Self> {
        let config = aws_config::from_env()
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let client = S3Client::new(&config);

        Self::new(client, bucket, kek)
    }

    /// Set a prefix for all object keys
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Generate a new random KEK
    pub fn generate_kek() -> [u8; 32] {
        KeyGenerator::generate_aes256_key()
    }

    /// Get full S3 key with prefix
    fn get_s3_key(&self, key: &str) -> String {
        if let Some(prefix) = &self.prefix {
            format!("{}/{}", prefix, key)
        } else {
            key.to_string()
        }
    }

    /// Get metadata key for object
    fn get_metadata_key(&self, key: &str, version_id: &Uuid) -> String {
        let s3_key = self.get_s3_key(key);
        format!("{}/.metadata/{}.json", s3_key, version_id)
    }

    /// Get versions list key
    fn get_versions_key(&self, key: &str) -> String {
        let s3_key = self.get_s3_key(key);
        format!("{}/.versions.json", s3_key)
    }

    /// Encrypt and upload object data
    async fn upload_encrypted_data(
        &self,
        key: &str,
        version_id: &Uuid,
        data: &[u8],
    ) -> GovernanceResult<S3EncryptedMetadata> {
        let s3_key = format!("{}/{}", self.get_s3_key(key), version_id);

        if data.len() < ENVELOPE_THRESHOLD {
            // Small file: use direct encryption
            let encrypted = self
                .encryptor
                .encrypt(data)
                .map_err(|e| GovernanceError::Storage(format!("Encryption failed: {}", e)))?;

            // Upload to S3
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(&s3_key)
                .body(ByteStream::from(Bytes::from(encrypted)))
                .content_type("application/octet-stream")
                .metadata("encrypted", "true")
                .metadata("algorithm", "AES-256-GCM")
                .send()
                .await
                .map_err(|e| GovernanceError::Storage(format!("S3 upload failed: {}", e)))?;

            Ok(S3EncryptedMetadata {
                metadata: ObjectMetadata::new(
                    key.to_string(),
                    data.len() as u64,
                    "application/octet-stream".to_string(),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                )
                .with_encryption("AES-256-GCM".to_string()),
                encryption_algorithm: "AES-256-GCM".to_string(),
                envelope_metadata: None,
                is_multipart: false,
            })
        } else {
            // Large file: use envelope encryption with multipart upload
            let envelope = EnvelopeEncryption::new(self.kek)
                .map_err(|e| GovernanceError::Storage(format!("Failed to create envelope: {}", e)))?;

            let (chunks, metadata) = envelope
                .encrypt_chunked(data, CHUNK_SIZE)
                .map_err(|e| GovernanceError::Storage(format!("Envelope encryption failed: {}", e)))?;

            // Create multipart upload
            let multipart = self
                .client
                .create_multipart_upload()
                .bucket(&self.bucket)
                .key(&s3_key)
                .content_type("application/octet-stream")
                .metadata("encrypted", "true")
                .metadata("algorithm", "AES-256-GCM-Envelope")
                .send()
                .await
                .map_err(|e| GovernanceError::Storage(format!("Failed to create multipart upload: {}", e)))?;

            let upload_id = multipart
                .upload_id()
                .ok_or_else(|| GovernanceError::Storage("No upload ID returned".to_string()))?;

            // Upload parts
            let mut completed_parts = Vec::new();
            for (i, chunk) in chunks.iter().enumerate() {
                let part_number = (i + 1) as i32;
                let upload_part = self
                    .client
                    .upload_part()
                    .bucket(&self.bucket)
                    .key(&s3_key)
                    .upload_id(upload_id)
                    .part_number(part_number)
                    .body(ByteStream::from(Bytes::from(chunk.clone())))
                    .send()
                    .await
                    .map_err(|e| {
                        GovernanceError::Storage(format!("Failed to upload part {}: {}", i, e))
                    })?;

                completed_parts.push(
                    aws_sdk_s3::types::CompletedPart::builder()
                        .part_number(part_number)
                        .e_tag(upload_part.e_tag().unwrap_or_default())
                        .build(),
                );
            }

            // Complete multipart upload
            self.client
                .complete_multipart_upload()
                .bucket(&self.bucket)
                .key(&s3_key)
                .upload_id(upload_id)
                .multipart_upload(
                    aws_sdk_s3::types::CompletedMultipartUpload::builder()
                        .set_parts(Some(completed_parts))
                        .build(),
                )
                .send()
                .await
                .map_err(|e| GovernanceError::Storage(format!("Failed to complete multipart: {}", e)))?;

            // Serialize envelope metadata
            let envelope_json = serde_json::to_string(&metadata)
                .map_err(|e| GovernanceError::Storage(format!("Failed to serialize envelope metadata: {}", e)))?;

            Ok(S3EncryptedMetadata {
                metadata: ObjectMetadata::new(
                    key.to_string(),
                    data.len() as u64,
                    "application/octet-stream".to_string(),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                )
                .with_encryption("AES-256-GCM-Envelope".to_string()),
                encryption_algorithm: "AES-256-GCM-Envelope".to_string(),
                envelope_metadata: Some(envelope_json),
                is_multipart: true,
            })
        }
    }

    /// Download and decrypt object data
    async fn download_decrypted_data(
        &self,
        key: &str,
        version_id: &Uuid,
        encrypted_meta: &S3EncryptedMetadata,
    ) -> GovernanceResult<Vec<u8>> {
        let s3_key = format!("{}/{}", self.get_s3_key(key), version_id);

        // Download from S3
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| GovernanceError::Storage(format!("S3 download failed: {}", e)))?;

        let encrypted_data = response
            .body
            .collect()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to read S3 body: {}", e)))?
            .into_bytes()
            .to_vec();

        if encrypted_meta.is_multipart {
            // Large file: decrypt with envelope
            let envelope_json = encrypted_meta
                .envelope_metadata
                .as_ref()
                .ok_or_else(|| GovernanceError::Storage("Missing envelope metadata".to_string()))?;

            let envelope_metadata: EnvelopeMetadata = serde_json::from_str(envelope_json)
                .map_err(|e| GovernanceError::Storage(format!("Failed to parse envelope metadata: {}", e)))?;

            // Split encrypted data back into chunks
            let chunk_size = envelope_metadata.chunk_size.unwrap_or(CHUNK_SIZE);
            let chunks: Vec<Vec<u8>> = encrypted_data
                .chunks(chunk_size)
                .map(|chunk| chunk.to_vec())
                .collect();

            let envelope = EnvelopeEncryption::new(self.kek)
                .map_err(|e| GovernanceError::Storage(format!("Failed to create envelope: {}", e)))?;

            envelope
                .decrypt_chunked(&chunks, &envelope_metadata)
                .map_err(|e| GovernanceError::Storage(format!("Envelope decryption failed: {}", e)))
        } else {
            // Small file: direct decryption
            self.encryptor
                .decrypt(&encrypted_data)
                .map_err(|e| GovernanceError::Storage(format!("Decryption failed: {}", e)))
        }
    }

    /// Load version list from S3
    async fn load_versions(&self, key: &str) -> GovernanceResult<Vec<ObjectVersion>> {
        let versions_key = self.get_versions_key(key);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&versions_key)
            .send()
            .await
        {
            Ok(response) => {
                let body = response
                    .body
                    .collect()
                    .await
                    .map_err(|e| GovernanceError::Storage(format!("Failed to read versions: {}", e)))?
                    .into_bytes();

                serde_json::from_slice(&body)
                    .map_err(|e| GovernanceError::Storage(format!("Failed to parse versions: {}", e)))
            }
            Err(_) => Ok(Vec::new()), // No versions yet
        }
    }

    /// Save version list to S3
    async fn save_versions(&self, key: &str, versions: &[ObjectVersion]) -> GovernanceResult<()> {
        let versions_key = self.get_versions_key(key);
        let content = serde_json::to_vec_pretty(versions)
            .map_err(|e| GovernanceError::Storage(format!("Failed to serialize versions: {}", e)))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&versions_key)
            .body(ByteStream::from(Bytes::from(content)))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to save versions: {}", e)))?;

        Ok(())
    }

    /// Save metadata to S3
    async fn save_metadata(&self, key: &str, metadata: &S3EncryptedMetadata) -> GovernanceResult<()> {
        let metadata_key = self.get_metadata_key(key, &metadata.metadata.version_id);
        let content = serde_json::to_vec_pretty(metadata)
            .map_err(|e| GovernanceError::Storage(format!("Failed to serialize metadata: {}", e)))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&metadata_key)
            .body(ByteStream::from(Bytes::from(content)))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to save metadata: {}", e)))?;

        Ok(())
    }

    /// Load metadata from S3
    async fn load_metadata(&self, key: &str, version_id: &Uuid) -> GovernanceResult<S3EncryptedMetadata> {
        let metadata_key = self.get_metadata_key(key, version_id);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&metadata_key)
            .send()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to load metadata: {}", e)))?;

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to read metadata: {}", e)))?
            .into_bytes();

        serde_json::from_slice(&body)
            .map_err(|e| GovernanceError::Storage(format!("Failed to parse metadata: {}", e)))
    }
}

#[async_trait]
impl StorageBackend for S3Backend {
    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        mut metadata: ObjectMetadata,
    ) -> GovernanceResult<ObjectMetadata> {
        // Encrypt and upload data
        let mut encrypted_meta = self.upload_encrypted_data(key, &metadata.version_id, &data).await?;

        // Update metadata
        metadata.size = data.len() as u64;
        metadata.modified_at = chrono::Utc::now();
        metadata.encrypted = true;
        metadata.encryption_algorithm = Some(encrypted_meta.encryption_algorithm.clone());
        metadata.etag = format!("{:x}-{}", metadata.size, metadata.modified_at.timestamp());

        encrypted_meta.metadata = metadata.clone();

        // Save metadata
        self.save_metadata(key, &encrypted_meta).await?;

        // Update version list
        let mut versions = self.load_versions(key).await?;

        // Mark all existing versions as not latest
        for v in versions.iter_mut() {
            v.is_latest = false;
        }

        // Add new version
        versions.push(ObjectVersion {
            version_id: metadata.version_id,
            metadata: metadata.clone(),
            is_latest: true,
            is_delete_marker: false,
        });

        self.save_versions(key, &versions).await?;

        Ok(metadata)
    }

    async fn get_object(
        &self,
        key: &str,
        version_id: Option<Uuid>,
    ) -> GovernanceResult<(Vec<u8>, ObjectMetadata)> {
        // Load versions
        let versions = self.load_versions(key).await?;
        if versions.is_empty() {
            return Err(GovernanceError::ObjectNotFound(key.to_string()));
        }

        // Find requested version
        let version = if let Some(vid) = version_id {
            versions
                .iter()
                .find(|v| v.version_id == vid)
                .ok_or_else(|| GovernanceError::VersionNotFound(vid.to_string()))?
        } else {
            versions
                .iter()
                .find(|v| v.is_latest)
                .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?
        };

        if version.is_delete_marker {
            return Err(GovernanceError::ObjectNotFound(key.to_string()));
        }

        // Load encrypted metadata
        let encrypted_meta = self.load_metadata(key, &version.version_id).await?;

        // Download and decrypt data
        let data = self
            .download_decrypted_data(key, &version.version_id, &encrypted_meta)
            .await?;

        Ok((data, version.metadata.clone()))
    }

    async fn delete_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<()> {
        let mut versions = self.load_versions(key).await?;
        if versions.is_empty() {
            return Err(GovernanceError::ObjectNotFound(key.to_string()));
        }

        if let Some(vid) = version_id {
            // Delete specific version
            let version = versions
                .iter()
                .find(|v| v.version_id == vid)
                .ok_or_else(|| GovernanceError::VersionNotFound(vid.to_string()))?;

            // Check if can delete
            if !version.metadata.can_delete() {
                return Err(GovernanceError::Storage(
                    "Object is under legal hold or retention".to_string(),
                ));
            }

            // Delete S3 objects
            let s3_key = format!("{}/{}", self.get_s3_key(key), vid);
            let metadata_key = self.get_metadata_key(key, &vid);

            let delete_objects = vec![
                ObjectIdentifier::builder().key(s3_key).build()
                    .map_err(|e| GovernanceError::Storage(format!("Failed to build object identifier: {}", e)))?,
                ObjectIdentifier::builder().key(metadata_key).build()
                    .map_err(|e| GovernanceError::Storage(format!("Failed to build metadata identifier: {}", e)))?,
            ];

            self.client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(Delete::builder().set_objects(Some(delete_objects)).build()
                    .map_err(|e| GovernanceError::Storage(format!("Failed to build delete request: {}", e)))?)
                .send()
                .await
                .map_err(|e| GovernanceError::Storage(format!("Failed to delete objects: {}", e)))?;

            versions.retain(|v| v.version_id != vid);
            if versions.is_empty() {
                let versions_key = self.get_versions_key(key);
                self.client
                    .delete_object()
                    .bucket(&self.bucket)
                    .key(&versions_key)
                    .send()
                    .await
                    .map_err(|e| GovernanceError::Storage(format!("Failed to delete versions: {}", e)))?;
            } else {
                self.save_versions(key, &versions).await?;
            }
        } else {
            // Create delete marker
            let latest = versions
                .iter()
                .find(|v| v.is_latest)
                .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?;

            if !latest.metadata.can_delete() {
                return Err(GovernanceError::Storage(
                    "Object is under legal hold or retention".to_string(),
                ));
            }

            let delete_marker = ObjectVersion {
                version_id: Uuid::new_v4(),
                metadata: latest.metadata.clone(),
                is_latest: true,
                is_delete_marker: true,
            };

            for v in versions.iter_mut() {
                v.is_latest = false;
            }

            versions.push(delete_marker);
            self.save_versions(key, &versions).await?;
        }

        Ok(())
    }

    async fn list_objects(&self, prefix: &str, max_keys: usize) -> GovernanceResult<Vec<ObjectMetadata>> {
        let search_prefix = if let Some(base_prefix) = &self.prefix {
            format!("{}/{}", base_prefix, prefix)
        } else {
            prefix.to_string()
        };

        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&search_prefix)
            .max_keys(max_keys as i32)
            .send()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to list objects: {}", e)))?;

        let mut results = Vec::new();
        if let Some(contents) = response.contents {
            for object in contents {
                if let Some(key) = object.key() {
                    // Skip metadata and version files
                    if key.contains("/.metadata/") || key.ends_with("/.versions.json") {
                        continue;
                    }

                    // Extract original key (remove prefix and version ID)
                    let original_key = if let Some(base_prefix) = &self.prefix {
                        key.strip_prefix(&format!("{}/", base_prefix))
                            .unwrap_or(key)
                    } else {
                        key
                    };

                    // Try to load versions to get latest metadata
                    if let Ok(versions) = self.load_versions(original_key).await {
                        if let Some(latest) = versions.iter().find(|v| v.is_latest && !v.is_delete_marker) {
                            results.push(latest.metadata.clone());
                        }
                    }
                }

                if results.len() >= max_keys {
                    break;
                }
            }
        }

        results.sort_by(|a, b| a.key.cmp(&b.key));
        results.truncate(max_keys);

        Ok(results)
    }

    async fn head_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<ObjectMetadata> {
        let versions = self.load_versions(key).await?;
        if versions.is_empty() {
            return Err(GovernanceError::ObjectNotFound(key.to_string()));
        }

        let version = if let Some(vid) = version_id {
            versions
                .iter()
                .find(|v| v.version_id == vid)
                .ok_or_else(|| GovernanceError::VersionNotFound(vid.to_string()))?
        } else {
            versions
                .iter()
                .find(|v| v.is_latest)
                .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?
        };

        if version.is_delete_marker {
            return Err(GovernanceError::ObjectNotFound(key.to_string()));
        }

        Ok(version.metadata.clone())
    }

    async fn list_versions(&self, key: &str) -> GovernanceResult<Vec<ObjectVersion>> {
        self.load_versions(key).await
    }

    async fn copy_object(&self, source_key: &str, dest_key: &str) -> GovernanceResult<ObjectMetadata> {
        // Get source object
        let (data, source_metadata) = self.get_object(source_key, None).await?;

        // Create new metadata for destination
        let mut new_metadata = source_metadata.clone();
        new_metadata.key = dest_key.to_string();
        new_metadata.version_id = Uuid::new_v4();
        new_metadata.created_at = chrono::Utc::now();
        new_metadata.modified_at = chrono::Utc::now();

        // Put to new location (will re-encrypt with new version ID)
        self.put_object(dest_key, data, new_metadata).await
    }

    async fn log_access(&self, log: AccessLog) -> GovernanceResult<()> {
        let log_key = format!("{}/.logs/access-{}.json", 
            self.prefix.as_deref().unwrap_or(""),
            chrono::Utc::now().format("%Y%m%d"));

        let log_json = serde_json::to_vec(&log)
            .map_err(|e| GovernanceError::Storage(format!("Failed to serialize log: {}", e)))?;

        // Append to log file (in production, use a proper log aggregation service)
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&log_key)
            .body(ByteStream::from(Bytes::from(log_json)))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to write log: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require actual AWS credentials and S3 bucket
    // They are marked as ignored by default
    // Run with: cargo test --features integration-tests -- --ignored

    #[tokio::test]
    #[ignore]
    async fn test_s3_encrypt_decrypt_small_file() {
        let kek = S3Backend::generate_kek();
        let backend = S3Backend::from_config(
            "test-bucket".to_string(),
            "us-east-1",
            kek,
        )
        .await
        .unwrap();

        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let metadata = ObjectMetadata::new(
            "test-small.txt".to_string(),
            13,
            "text/plain".to_string(),
            user_id,
            org_id,
        );

        let data = b"Hello, World!".to_vec();
        let stored = backend.put_object("test-small.txt", data.clone(), metadata).await;
        assert!(stored.is_ok());

        let (retrieved_data, _) = backend.get_object("test-small.txt", None).await.unwrap();
        assert_eq!(retrieved_data, data);
    }

    #[tokio::test]
    #[ignore]
    async fn test_s3_encrypt_decrypt_large_file() {
        let kek = S3Backend::generate_kek();
        let backend = S3Backend::from_config(
            "test-bucket".to_string(),
            "us-east-1",
            kek,
        )
        .await
        .unwrap();

        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        // Create 10MB file
        let data = vec![0x42u8; 10 * 1024 * 1024];

        let metadata = ObjectMetadata::new(
            "test-large.bin".to_string(),
            data.len() as u64,
            "application/octet-stream".to_string(),
            user_id,
            org_id,
        );

        let stored = backend.put_object("test-large.bin", data.clone(), metadata).await;
        assert!(stored.is_ok());

        let stored_meta = stored.unwrap();
        assert!(stored_meta.encrypted);
        assert_eq!(stored_meta.encryption_algorithm, Some("AES-256-GCM-Envelope".to_string()));

        let (retrieved_data, _) = backend.get_object("test-large.bin", None).await.unwrap();
        assert_eq!(retrieved_data.len(), data.len());
        assert_eq!(retrieved_data, data);
    }
}
