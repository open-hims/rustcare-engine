use crate::error::{GovernanceError, GovernanceResult};
use crate::storage::{AccessLog, ObjectMetadata, ObjectVersion, StorageBackend};
use async_trait::async_trait;
use crypto::aes_gcm::{Aes256GcmEncryptor, KeyGenerator};
use crypto::encryption::Encryptor;
use crypto::envelope::{EnvelopeEncryption, EnvelopeMetadata};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// Minimum size for envelope encryption (1MB)
const ENVELOPE_THRESHOLD: usize = 1024 * 1024;

/// Chunk size for envelope encryption (1MB)
const CHUNK_SIZE: usize = 1024 * 1024;

/// File system storage backend with encryption
pub struct FileSystemBackend {
    /// Base directory for storage
    base_path: PathBuf,
    /// KEK (Key Encryption Key) for envelope encryption
    kek: [u8; 32],
    /// Direct encryptor for small files
    encryptor: Aes256GcmEncryptor,
}

/// Encrypted object wrapper stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedObject {
    /// Object metadata (unencrypted for querying)
    pub metadata: ObjectMetadata,
    /// Encrypted data (for small objects)
    pub encrypted_data: Option<Vec<u8>>,
    /// Envelope metadata (for large objects)
    pub envelope_metadata: Option<EnvelopeMetadata>,
    /// Whether data is stored in chunks
    pub is_chunked: bool,
}

impl FileSystemBackend {
    /// Create a new filesystem backend with encryption
    /// 
    /// # Arguments
    /// * `base_path` - Base directory for storage
    /// * `kek` - 32-byte Key Encryption Key for envelope encryption
    pub fn new(base_path: impl AsRef<Path>, kek: [u8; 32]) -> GovernanceResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Create encryptor for small files
        let encryptor = Aes256GcmEncryptor::new(kek)
            .map_err(|e| GovernanceError::Storage(format!("Failed to create encryptor: {}", e)))?;

        Ok(Self {
            base_path,
            kek,
            encryptor,
        })
    }

    /// Generate a new random KEK
    pub fn generate_kek() -> [u8; 32] {
        KeyGenerator::generate_aes256_key()
    }

    /// Initialize storage directory structure
    pub async fn initialize(&self) -> GovernanceResult<()> {
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to create base directory: {}", e)))?;

        // Create subdirectories
        for subdir in &["objects", "versions", "logs"] {
            let path = self.base_path.join(subdir);
            fs::create_dir_all(&path)
                .await
                .map_err(|e| GovernanceError::Storage(format!("Failed to create {} directory: {}", subdir, e)))?;
        }

        Ok(())
    }

    /// Get path for object data
    fn get_object_path(&self, key: &str, version_id: &Uuid) -> PathBuf {
        self.base_path
            .join("objects")
            .join(format!("{}-{}.dat", key.replace('/', "_"), version_id))
    }

    /// Get path for object metadata
    fn get_metadata_path(&self, key: &str, version_id: &Uuid) -> PathBuf {
        self.base_path
            .join("objects")
            .join(format!("{}-{}.meta.json", key.replace('/', "_"), version_id))
    }

    /// Get path for version list
    fn get_versions_path(&self, key: &str) -> PathBuf {
        self.base_path
            .join("versions")
            .join(format!("{}.json", key.replace('/', "_")))
    }

    /// Get path for chunk file
    fn get_chunk_path(&self, key: &str, version_id: &Uuid, chunk_index: usize) -> PathBuf {
        self.base_path
            .join("objects")
            .join(format!("{}-{}.chunk{}.dat", key.replace('/', "_"), version_id, chunk_index))
    }

    /// Encrypt and store object data
    async fn store_encrypted_data(
        &self,
        key: &str,
        version_id: &Uuid,
        data: &[u8],
    ) -> GovernanceResult<EncryptedObject> {
        if data.len() < ENVELOPE_THRESHOLD {
            // Small file: use direct encryption
            let encrypted = self
                .encryptor
                .encrypt(data)
                .map_err(|e| GovernanceError::Storage(format!("Encryption failed: {}", e)))?;

            Ok(EncryptedObject {
                metadata: ObjectMetadata::new(
                    key.to_string(),
                    data.len() as u64,
                    "application/octet-stream".to_string(),
                    Uuid::new_v4(), // Will be replaced by caller
                    Uuid::new_v4(), // Will be replaced by caller
                )
                .with_encryption("AES-256-GCM".to_string()),
                encrypted_data: Some(encrypted),
                envelope_metadata: None,
                is_chunked: false,
            })
        } else {
            // Large file: use envelope encryption with chunking
            let envelope = EnvelopeEncryption::new(self.kek)
                .map_err(|e| GovernanceError::Storage(format!("Failed to create envelope encryption: {}", e)))?;
            let (chunks, metadata) = envelope
                .encrypt_chunked(data, CHUNK_SIZE)
                .map_err(|e| GovernanceError::Storage(format!("Envelope encryption failed: {}", e)))?;

            // Write chunks to disk
            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_path = self.get_chunk_path(key, version_id, i);
                fs::write(&chunk_path, chunk)
                    .await
                    .map_err(|e| GovernanceError::Storage(format!("Failed to write chunk {}: {}", i, e)))?;
            }

            Ok(EncryptedObject {
                metadata: ObjectMetadata::new(
                    key.to_string(),
                    data.len() as u64,
                    "application/octet-stream".to_string(),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                )
                .with_encryption("AES-256-GCM-Envelope".to_string()),
                encrypted_data: None,
                envelope_metadata: Some(metadata),
                is_chunked: true,
            })
        }
    }

    /// Retrieve and decrypt object data
    async fn retrieve_decrypted_data(
        &self,
        key: &str,
        version_id: &Uuid,
        encrypted_obj: &EncryptedObject,
    ) -> GovernanceResult<Vec<u8>> {
        if encrypted_obj.is_chunked {
            // Large file: decrypt chunks
            let envelope_metadata = encrypted_obj
                .envelope_metadata
                .as_ref()
                .ok_or_else(|| GovernanceError::Storage("Missing envelope metadata".to_string()))?;

            let chunk_count = envelope_metadata.chunk_count.unwrap_or(1);
            let mut chunks = Vec::with_capacity(chunk_count);

            for i in 0..chunk_count {
                let chunk_path = self.get_chunk_path(key, version_id, i);
                let chunk = fs::read(&chunk_path)
                    .await
                    .map_err(|e| GovernanceError::Storage(format!("Failed to read chunk {}: {}", i, e)))?;
                chunks.push(chunk);
            }

            let envelope = EnvelopeEncryption::new(self.kek)
                .map_err(|e| GovernanceError::Storage(format!("Failed to create envelope encryption: {}", e)))?;
            envelope
                .decrypt_chunked(&chunks, envelope_metadata)
                .map_err(|e| GovernanceError::Storage(format!("Envelope decryption failed: {}", e)))
        } else {
            // Small file: direct decryption
            let encrypted_data = encrypted_obj
                .encrypted_data
                .as_ref()
                .ok_or_else(|| GovernanceError::Storage("Missing encrypted data".to_string()))?;

            self.encryptor
                .decrypt(encrypted_data)
                .map_err(|e| GovernanceError::Storage(format!("Decryption failed: {}", e)))
        }
    }

    /// Load version list from disk
    async fn load_versions(&self, key: &str) -> GovernanceResult<Vec<ObjectVersion>> {
        let versions_path = self.get_versions_path(key);
        
        if !versions_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&versions_path)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to read versions: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| GovernanceError::Storage(format!("Failed to parse versions: {}", e)))
    }

    /// Save version list to disk
    async fn save_versions(&self, key: &str, versions: &[ObjectVersion]) -> GovernanceResult<()> {
        let versions_path = self.get_versions_path(key);
        let content = serde_json::to_string_pretty(versions)
            .map_err(|e| GovernanceError::Storage(format!("Failed to serialize versions: {}", e)))?;

        fs::write(&versions_path, content)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to write versions: {}", e)))?;

        Ok(())
    }

    /// Delete all chunks for an object
    async fn delete_chunks(&self, key: &str, version_id: &Uuid, chunk_count: usize) -> GovernanceResult<()> {
        for i in 0..chunk_count {
            let chunk_path = self.get_chunk_path(key, version_id, i);
            if chunk_path.exists() {
                fs::remove_file(&chunk_path)
                    .await
                    .map_err(|e| GovernanceError::Storage(format!("Failed to delete chunk {}: {}", i, e)))?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for FileSystemBackend {
    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        mut metadata: ObjectMetadata,
    ) -> GovernanceResult<ObjectMetadata> {
        // Encrypt and store data
        let mut encrypted_obj = self.store_encrypted_data(key, &metadata.version_id, &data).await?;

        // Update metadata
        metadata.size = data.len() as u64;
        metadata.modified_at = chrono::Utc::now();
        metadata.encrypted = true;
        metadata.encryption_algorithm = encrypted_obj.metadata.encryption_algorithm.clone();
        metadata.etag = format!("{:x}-{}", metadata.size, metadata.modified_at.timestamp());

        encrypted_obj.metadata = metadata.clone();

        // Save encrypted object metadata
        let metadata_path = self.get_metadata_path(key, &metadata.version_id);
        let metadata_json = serde_json::to_string_pretty(&encrypted_obj)
            .map_err(|e| GovernanceError::Storage(format!("Failed to serialize metadata: {}", e)))?;

        fs::write(&metadata_path, metadata_json)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to write metadata: {}", e)))?;

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

        // Load encrypted object
        let metadata_path = self.get_metadata_path(key, &version.version_id);
        let metadata_content = fs::read_to_string(&metadata_path)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to read metadata: {}", e)))?;

        let encrypted_obj: EncryptedObject = serde_json::from_str(&metadata_content)
            .map_err(|e| GovernanceError::Storage(format!("Failed to parse metadata: {}", e)))?;

        // Decrypt data
        let data = self.retrieve_decrypted_data(key, &version.version_id, &encrypted_obj).await?;

        Ok((data, version.metadata.clone()))
    }

    async fn delete_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<()> {
        let mut versions = self.load_versions(key).await?;
        if versions.is_empty() {
            return Err(GovernanceError::ObjectNotFound(key.to_string()));
        }

        if let Some(vid) = version_id {
            // Delete specific version
            let version_to_delete = versions
                .iter()
                .find(|v| v.version_id == vid)
                .ok_or_else(|| GovernanceError::VersionNotFound(vid.to_string()))?
                .clone();

            // Check if can delete
            if !version_to_delete.metadata.can_delete() {
                return Err(GovernanceError::Storage(
                    "Object is under legal hold or retention".to_string(),
                ));
            }

            // Delete files
            let metadata_path = self.get_metadata_path(key, &vid);
            if metadata_path.exists() {
                fs::remove_file(&metadata_path)
                    .await
                    .map_err(|e| GovernanceError::Storage(format!("Failed to delete metadata: {}", e)))?;
            }

            // Delete chunks if applicable
            let metadata_content = fs::read_to_string(&metadata_path).await.ok();
            if let Some(content) = metadata_content {
                if let Ok(encrypted_obj) = serde_json::from_str::<EncryptedObject>(&content) {
                    if encrypted_obj.is_chunked {
                        if let Some(envelope_meta) = encrypted_obj.envelope_metadata {
                            let chunk_count = envelope_meta.chunk_count.unwrap_or(0);
                            self.delete_chunks(key, &vid, chunk_count).await?;
                        }
                    }
                }
            }

            versions.retain(|v| v.version_id != vid);
            if versions.is_empty() {
                let versions_path = self.get_versions_path(key);
                fs::remove_file(&versions_path)
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
        let versions_dir = self.base_path.join("versions");
        
        if !versions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&versions_dir)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to read versions directory: {}", e)))?;

        let mut results = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            GovernanceError::Storage(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let content = fs::read_to_string(&path).await.ok();
            if let Some(content) = content {
                if let Ok(versions) = serde_json::from_str::<Vec<ObjectVersion>>(&content) {
                    if let Some(latest) = versions.iter().find(|v| v.is_latest && !v.is_delete_marker) {
                        if latest.metadata.key.starts_with(prefix) {
                            results.push(latest.metadata.clone());
                        }
                    }
                }
            }

            if results.len() >= max_keys {
                break;
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
        let logs_dir = self.base_path.join("logs");
        let log_file = logs_dir.join("access.log");

        let log_json = serde_json::to_string(&log)
            .map_err(|e| GovernanceError::Storage(format!("Failed to serialize log: {}", e)))?;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to open log file: {}", e)))?;

        file.write_all(log_json.as_bytes())
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to write log: {}", e)))?;

        file.write_all(b"\n")
            .await
            .map_err(|e| GovernanceError::Storage(format!("Failed to write newline: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_backend() -> (FileSystemBackend, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let kek = FileSystemBackend::generate_kek();
        let backend = FileSystemBackend::new(temp_dir.path(), kek).unwrap();
        backend.initialize().await.unwrap();
        (backend, temp_dir)
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_small_file() {
        let (backend, _temp_dir) = create_test_backend().await;
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let metadata = ObjectMetadata::new(
            "test.txt".to_string(),
            13,
            "text/plain".to_string(),
            user_id,
            org_id,
        );

        let data = b"Hello, World!".to_vec();
        let put_result = backend.put_object("test.txt", data.clone(), metadata).await;
        assert!(put_result.is_ok());

        let stored_meta = put_result.unwrap();
        assert!(stored_meta.encrypted);
        assert_eq!(stored_meta.encryption_algorithm, Some("AES-256-GCM".to_string()));

        let (retrieved_data, _) = backend.get_object("test.txt", None).await.unwrap();
        assert_eq!(retrieved_data, data);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_large_file() {
        let (backend, _temp_dir) = create_test_backend().await;
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        // Create 2MB file
        let data = vec![0x42u8; 2 * 1024 * 1024];

        let metadata = ObjectMetadata::new(
            "large.bin".to_string(),
            data.len() as u64,
            "application/octet-stream".to_string(),
            user_id,
            org_id,
        );

        let put_result = backend.put_object("large.bin", data.clone(), metadata).await;
        assert!(put_result.is_ok());

        let stored_meta = put_result.unwrap();
        assert!(stored_meta.encrypted);
        assert_eq!(stored_meta.encryption_algorithm, Some("AES-256-GCM-Envelope".to_string()));

        let (retrieved_data, _) = backend.get_object("large.bin", None).await.unwrap();
        assert_eq!(retrieved_data.len(), data.len());
        assert_eq!(retrieved_data, data);
    }

    #[tokio::test]
    async fn test_versioning_with_encryption() {
        let (backend, _temp_dir) = create_test_backend().await;
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        // Version 1
        let metadata1 = ObjectMetadata::new("versioned.txt".to_string(), 2, "text/plain".to_string(), user_id, org_id);
        backend.put_object("versioned.txt", b"v1".to_vec(), metadata1).await.unwrap();

        // Version 2
        let metadata2 = ObjectMetadata::new("versioned.txt".to_string(), 2, "text/plain".to_string(), user_id, org_id);
        backend.put_object("versioned.txt", b"v2".to_vec(), metadata2).await.unwrap();

        let versions = backend.list_versions("versioned.txt").await.unwrap();
        assert_eq!(versions.len(), 2);

        // Get latest (v2)
        let (data_latest, _) = backend.get_object("versioned.txt", None).await.unwrap();
        assert_eq!(data_latest, b"v2");

        // Get specific version (v1)
        let (data_v1, _) = backend.get_object("versioned.txt", Some(versions[0].version_id)).await.unwrap();
        assert_eq!(data_v1, b"v1");
    }

    #[tokio::test]
    async fn test_copy_object_with_reencryption() {
        let (backend, _temp_dir) = create_test_backend().await;
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let metadata = ObjectMetadata::new("source.txt".to_string(), 4, "text/plain".to_string(), user_id, org_id);
        backend.put_object("source.txt", b"test".to_vec(), metadata).await.unwrap();

        let copied_meta = backend.copy_object("source.txt", "dest.txt").await.unwrap();
        assert_eq!(copied_meta.key, "dest.txt");
        assert!(copied_meta.encrypted);

        let (copied_data, _) = backend.get_object("dest.txt", None).await.unwrap();
        assert_eq!(copied_data, b"test");
    }
}
