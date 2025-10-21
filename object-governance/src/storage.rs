use crate::classification::{ClassificationMetadata, DataClassification};
use crate::error::{GovernanceError, GovernanceResult};
use crate::lifecycle::{LifecycleRule, RetentionPolicy, StorageTier};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Object metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub version_id: Uuid,
    pub size: u64,
    pub content_type: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub owner_id: Uuid,
    pub organization_id: Uuid,
    pub classification: Option<ClassificationMetadata>,
    pub storage_tier: StorageTier,
    pub encrypted: bool,
    pub encryption_algorithm: Option<String>,
    pub tags: HashMap<String, String>,
    pub legal_hold: bool,
    pub retention_until: Option<DateTime<Utc>>,
    pub custom_metadata: HashMap<String, String>,
}

impl ObjectMetadata {
    pub fn new(
        key: String,
        size: u64,
        content_type: String,
        owner_id: Uuid,
        organization_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            key,
            version_id: Uuid::new_v4(),
            size,
            content_type,
            etag: Uuid::new_v4().to_string(),
            created_at: now,
            modified_at: now,
            owner_id,
            organization_id,
            classification: None,
            storage_tier: StorageTier::Hot,
            encrypted: false,
            encryption_algorithm: None,
            tags: HashMap::new(),
            legal_hold: false,
            retention_until: None,
            custom_metadata: HashMap::new(),
        }
    }

    pub fn with_classification(mut self, classification: ClassificationMetadata) -> Self {
        self.classification = Some(classification);
        self
    }

    pub fn with_encryption(mut self, algorithm: String) -> Self {
        self.encrypted = true;
        self.encryption_algorithm = Some(algorithm);
        self
    }

    pub fn with_storage_tier(mut self, tier: StorageTier) -> Self {
        self.storage_tier = tier;
        self
    }

    pub fn add_tag(&mut self, key: String, value: String) {
        self.tags.insert(key, value);
    }

    pub fn set_legal_hold(&mut self, hold: bool) {
        self.legal_hold = hold;
    }

    pub fn set_retention(&mut self, until: DateTime<Utc>) {
        self.retention_until = Some(until);
    }

    /// Check if object can be deleted (not under legal hold or retention)
    pub fn can_delete(&self) -> bool {
        if self.legal_hold {
            return false;
        }

        if let Some(retention_until) = self.retention_until {
            if Utc::now() < retention_until {
                return false;
            }
        }

        true
    }
}

/// Object version for versioning support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectVersion {
    pub version_id: Uuid,
    pub metadata: ObjectMetadata,
    pub is_latest: bool,
    pub is_delete_marker: bool,
}

/// Access log entry for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLog {
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub key: String,
    pub version_id: Option<Uuid>,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: u16,
    pub bytes_transferred: u64,
    pub error_message: Option<String>,
}

impl AccessLog {
    pub fn new(
        operation: String,
        key: String,
        user_id: Uuid,
        organization_id: Uuid,
        status: u16,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            operation,
            key,
            version_id: None,
            user_id,
            organization_id,
            ip_address: None,
            user_agent: None,
            status,
            bytes_transferred: 0,
            error_message: None,
        }
    }

    pub fn with_version(mut self, version_id: Uuid) -> Self {
        self.version_id = Some(version_id);
        self
    }

    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes_transferred = bytes;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error_message = Some(error);
        self
    }
}

/// Storage backend trait for S3-compatible operations
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Put an object
    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        metadata: ObjectMetadata,
    ) -> GovernanceResult<ObjectMetadata>;

    /// Get an object
    async fn get_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<(Vec<u8>, ObjectMetadata)>;

    /// Delete an object
    async fn delete_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<()>;

    /// List objects with prefix
    async fn list_objects(&self, prefix: &str, max_keys: usize) -> GovernanceResult<Vec<ObjectMetadata>>;

    /// Get object metadata
    async fn head_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<ObjectMetadata>;

    /// List object versions
    async fn list_versions(&self, key: &str) -> GovernanceResult<Vec<ObjectVersion>>;

    /// Copy an object
    async fn copy_object(&self, source_key: &str, dest_key: &str) -> GovernanceResult<ObjectMetadata>;

    /// Log access
    async fn log_access(&self, log: AccessLog) -> GovernanceResult<()>;
}

/// In-memory storage backend for development/testing
pub struct InMemoryStorageBackend {
    objects: Arc<RwLock<HashMap<String, Vec<ObjectVersion>>>>,
    access_logs: Arc<RwLock<Vec<AccessLog>>>,
}

impl InMemoryStorageBackend {
    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(HashMap::new())),
            access_logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get access logs (for testing/development)
    pub async fn get_access_logs(&self) -> Vec<AccessLog> {
        self.access_logs.read().await.clone()
    }

    /// Clear all objects (for testing)
    pub async fn clear(&self) {
        self.objects.write().await.clear();
        self.access_logs.write().await.clear();
    }
}

impl Default for InMemoryStorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for InMemoryStorageBackend {
    async fn put_object(
        &self,
        key: &str,
        data: Vec<u8>,
        mut metadata: ObjectMetadata,
    ) -> GovernanceResult<ObjectMetadata> {
        let mut objects = self.objects.write().await;

        // Update metadata
        metadata.size = data.len() as u64;
        metadata.modified_at = Utc::now();
        // Simple etag based on size and timestamp
        metadata.etag = format!("{:x}-{}", metadata.size, metadata.modified_at.timestamp());

        // Create new version
        let version = ObjectVersion {
            version_id: metadata.version_id,
            metadata: metadata.clone(),
            is_latest: true,
            is_delete_marker: false,
        };

        // Get or create version list
        let versions = objects.entry(key.to_string()).or_insert_with(Vec::new);

        // Mark all existing versions as not latest
        for v in versions.iter_mut() {
            v.is_latest = false;
        }

        // Add new version
        versions.push(version);

        Ok(metadata)
    }

    async fn get_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<(Vec<u8>, ObjectMetadata)> {
        let objects = self.objects.read().await;

        let versions = objects
            .get(key)
            .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?;

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

        // In a real implementation, we'd fetch the actual data
        // For now, return empty data
        Ok((Vec::new(), version.metadata.clone()))
    }

    async fn delete_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<()> {
        let mut objects = self.objects.write().await;

        let versions = objects
            .get_mut(key)
            .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?;

        if let Some(vid) = version_id {
            // Delete specific version
            versions.retain(|v| v.version_id != vid);
            if versions.is_empty() {
                objects.remove(key);
            }
        } else {
            // Create delete marker
            let metadata = versions
                .iter()
                .find(|v| v.is_latest)
                .map(|v| v.metadata.clone())
                .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?;

            // Check if object can be deleted
            if !metadata.can_delete() {
                return Err(GovernanceError::Storage(
                    "Object is under legal hold or retention".to_string(),
                ));
            }

            let delete_marker = ObjectVersion {
                version_id: Uuid::new_v4(),
                metadata,
                is_latest: true,
                is_delete_marker: true,
            };

            for v in versions.iter_mut() {
                v.is_latest = false;
            }

            versions.push(delete_marker);
        }

        Ok(())
    }

    async fn list_objects(&self, prefix: &str, max_keys: usize) -> GovernanceResult<Vec<ObjectMetadata>> {
        let objects = self.objects.read().await;

        let mut results: Vec<ObjectMetadata> = objects
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .filter_map(|(_, versions)| {
                versions
                    .iter()
                    .find(|v| v.is_latest && !v.is_delete_marker)
                    .map(|v| v.metadata.clone())
            })
            .take(max_keys)
            .collect();

        results.sort_by(|a, b| a.key.cmp(&b.key));

        Ok(results)
    }

    async fn head_object(&self, key: &str, version_id: Option<Uuid>) -> GovernanceResult<ObjectMetadata> {
        let objects = self.objects.read().await;

        let versions = objects
            .get(key)
            .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?;

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
        let objects = self.objects.read().await;

        let versions = objects
            .get(key)
            .ok_or_else(|| GovernanceError::ObjectNotFound(key.to_string()))?;

        Ok(versions.clone())
    }

    async fn copy_object(&self, source_key: &str, dest_key: &str) -> GovernanceResult<ObjectMetadata> {
        let objects_read = self.objects.read().await;

        let source_versions = objects_read
            .get(source_key)
            .ok_or_else(|| GovernanceError::ObjectNotFound(source_key.to_string()))?;

        let source_version = source_versions
            .iter()
            .find(|v| v.is_latest && !v.is_delete_marker)
            .ok_or_else(|| GovernanceError::ObjectNotFound(source_key.to_string()))?;

        let mut new_metadata = source_version.metadata.clone();
        new_metadata.key = dest_key.to_string();
        new_metadata.version_id = Uuid::new_v4();
        new_metadata.created_at = Utc::now();
        new_metadata.modified_at = Utc::now();

        drop(objects_read);

        // Use put_object to add the copy
        self.put_object(dest_key, Vec::new(), new_metadata.clone()).await?;

        Ok(new_metadata)
    }

    async fn log_access(&self, log: AccessLog) -> GovernanceResult<()> {
        let mut logs = self.access_logs.write().await;
        logs.push(log);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_and_get_object() {
        let backend = InMemoryStorageBackend::new();
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let metadata = ObjectMetadata::new(
            "test.txt".to_string(),
            10,
            "text/plain".to_string(),
            user_id,
            org_id,
        );

        let data = b"Hello, World!".to_vec();
        let put_result = backend.put_object("test.txt", data.clone(), metadata).await;
        assert!(put_result.is_ok());

        let (retrieved_data, retrieved_metadata) = backend.get_object("test.txt", None).await.unwrap();
        assert_eq!(retrieved_metadata.key, "test.txt");
        assert_eq!(retrieved_metadata.size, 13); // Updated from actual data
    }

    #[tokio::test]
    async fn test_versioning() {
        let backend = InMemoryStorageBackend::new();
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        // Put version 1
        let metadata1 = ObjectMetadata::new("test.txt".to_string(), 5, "text/plain".to_string(), user_id, org_id);
        backend.put_object("test.txt", b"v1".to_vec(), metadata1).await.unwrap();

        // Put version 2
        let metadata2 = ObjectMetadata::new("test.txt".to_string(), 5, "text/plain".to_string(), user_id, org_id);
        backend.put_object("test.txt", b"v2".to_vec(), metadata2).await.unwrap();

        let versions = backend.list_versions("test.txt").await.unwrap();
        assert_eq!(versions.len(), 2);
        assert!(versions[1].is_latest);
        assert!(!versions[0].is_latest);
    }

    #[tokio::test]
    async fn test_legal_hold() {
        let backend = InMemoryStorageBackend::new();
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let mut metadata = ObjectMetadata::new("protected.txt".to_string(), 10, "text/plain".to_string(), user_id, org_id);
        metadata.set_legal_hold(true);

        backend.put_object("protected.txt", b"secret".to_vec(), metadata).await.unwrap();

        let result = backend.delete_object("protected.txt", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_access_logging() {
        let backend = InMemoryStorageBackend::new();
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let log = AccessLog::new("GET".to_string(), "test.txt".to_string(), user_id, org_id, 200)
            .with_bytes(1024);

        backend.log_access(log).await.unwrap();

        let logs = backend.get_access_logs().await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].operation, "GET");
        assert_eq!(logs[0].bytes_transferred, 1024);
    }
}
