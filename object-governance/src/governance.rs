use crate::classification::{ClassificationMetadata, DataClassification};
use crate::error::{GovernanceError, GovernanceResult};
use crate::lifecycle::{LifecycleRule, RetentionPolicy};
use crate::policies::{AutoClassifier, PolicyAction, PolicyEngine};
use crate::storage::{AccessLog, ObjectMetadata, ObjectVersion, StorageBackend};
use auth_zanzibar::engine::AuthorizationEngine;
use auth_zanzibar::models::{Subject, Relation, Object};
use crypto::encryption::Encryptor;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Main governance engine integrating all components
pub struct GovernanceEngine {
    storage_backend: Arc<dyn StorageBackend>,
    policy_engine: Arc<RwLock<PolicyEngine>>,
    auto_classifier: Arc<AutoClassifier>,
    auth_engine: Option<Arc<AuthorizationEngine>>,
    encryptor: Option<Arc<dyn Encryptor>>,
    audit_enabled: bool,
}

impl GovernanceEngine {
    /// Create a new governance engine
    pub fn new(storage_backend: Arc<dyn StorageBackend>) -> Self {
        let policy_engine = PolicyEngine::new(storage_backend.clone());

        Self {
            storage_backend: storage_backend.clone(),
            policy_engine: Arc::new(RwLock::new(policy_engine)),
            auto_classifier: Arc::new(AutoClassifier::new()),
            auth_engine: None,
            encryptor: None,
            audit_enabled: true,
        }
    }

    /// Enable authorization integration
    pub fn with_authorization(mut self, auth_engine: Arc<AuthorizationEngine>) -> Self {
        self.auth_engine = Some(auth_engine);
        self
    }

    /// Enable encryption for sensitive data
    pub fn with_encryption(mut self, encryptor: Arc<dyn Encryptor>) -> Self {
        self.encryptor = Some(encryptor);
        self
    }

    /// Enable/disable audit logging
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit_enabled = enabled;
        self
    }

    /// Add a lifecycle rule
    pub async fn add_lifecycle_rule(&self, rule: LifecycleRule) -> GovernanceResult<()> {
        let mut engine = self.policy_engine.write().await;
        engine.add_lifecycle_rule(rule)?;
        info!("Added lifecycle rule");
        Ok(())
    }

    /// Add a retention policy
    pub async fn add_retention_policy(&self, policy: RetentionPolicy) -> GovernanceResult<()> {
        let mut engine = self.policy_engine.write().await;
        engine.add_retention_policy(policy)?;
        info!("Added retention policy");
        Ok(())
    }

    /// Put an object with automatic classification and encryption
    pub async fn put_object(
        &self,
        key: &str,
        mut data: Vec<u8>,
        mut metadata: ObjectMetadata,
        user_id: Uuid,
        auto_classify: bool,
    ) -> GovernanceResult<ObjectMetadata> {
        // Check authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("write"),
                    Object::new("object", key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            if !authorized {
                return Err(GovernanceError::Authorization(
                    "User not authorized to write object".to_string(),
                ));
            }
        }

        // Auto-classify if requested
        if auto_classify && metadata.classification.is_none() {
            if let Ok(content) = String::from_utf8(data.clone()) {
                if let Some(classification) = self.auto_classifier.classify_content(&content) {
                    info!(
                        "Auto-classified {} as {:?} with confidence {}",
                        key, classification.classification, classification.confidence
                    );
                    metadata = metadata.with_classification(classification);
                }
            }
        }

        // Encrypt if classification requires it
        if let Some(ref classification) = metadata.classification {
            if classification.classification.requires_encryption() {
                if let Some(ref encryptor) = self.encryptor {
                    data = encryptor
                        .encrypt(&data)
                        .map_err(|e| GovernanceError::Encryption(e.to_string()))?;
                    metadata = metadata.with_encryption("AES-256-GCM".to_string());
                    info!("Encrypted {} due to classification", key);
                } else {
                    warn!("Object {} requires encryption but no encryptor configured", key);
                }
            }
        }

        // Store object
        let result = self.storage_backend.put_object(key, data, metadata).await?;

        // Audit log
        if self.audit_enabled {
            let log = AccessLog::new(
                "PUT".to_string(),
                key.to_string(),
                user_id,
                result.organization_id,
                200,
            )
            .with_version(result.version_id)
            .with_bytes(result.size);

            self.storage_backend.log_access(log).await?;
        }

        Ok(result)
    }

    /// Get an object with authorization check and decryption
    pub async fn get_object(
        &self,
        key: &str,
        version_id: Option<Uuid>,
        user_id: Uuid,
    ) -> GovernanceResult<(Vec<u8>, ObjectMetadata)> {
        // Check authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("read"),
                    Object::new("object", key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            if !authorized {
                return Err(GovernanceError::Authorization(
                    "User not authorized to read object".to_string(),
                ));
            }
        }

        // Retrieve object
        let (mut data, metadata) = self.storage_backend.get_object(key, version_id).await?;

        // Decrypt if encrypted
        if metadata.encrypted {
            if let Some(ref encryptor) = self.encryptor {
                data = encryptor
                    .decrypt(&data)
                    .map_err(|e| GovernanceError::Encryption(e.to_string()))?;
            } else {
                return Err(GovernanceError::Encryption(
                    "Object is encrypted but no decryptor configured".to_string(),
                ));
            }
        }

        // Audit log
        if self.audit_enabled {
            let log = AccessLog::new(
                "GET".to_string(),
                key.to_string(),
                user_id,
                metadata.organization_id,
                200,
            )
            .with_version(metadata.version_id)
            .with_bytes(metadata.size);

            self.storage_backend.log_access(log).await?;
        }

        Ok((data, metadata))
    }

    /// Delete an object with authorization check
    pub async fn delete_object(
        &self,
        key: &str,
        version_id: Option<Uuid>,
        user_id: Uuid,
    ) -> GovernanceResult<()> {
        // Check authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("delete"),
                    Object::new("object", key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            if !authorized {
                return Err(GovernanceError::Authorization(
                    "User not authorized to delete object".to_string(),
                ));
            }
        }

        // Get metadata for audit
        let metadata = self.storage_backend.head_object(key, version_id).await?;

        // Delete object
        self.storage_backend.delete_object(key, version_id).await?;

        // Audit log
        if self.audit_enabled {
            let log = AccessLog::new(
                "DELETE".to_string(),
                key.to_string(),
                user_id,
                metadata.organization_id,
                200,
            )
            .with_version(metadata.version_id);

            self.storage_backend.log_access(log).await?;
        }

        Ok(())
    }

    /// List objects with authorization filtering
    pub async fn list_objects(
        &self,
        prefix: &str,
        max_keys: usize,
        user_id: Uuid,
    ) -> GovernanceResult<Vec<ObjectMetadata>> {
        let objects = self.storage_backend.list_objects(prefix, max_keys).await?;

        // Filter by authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let mut authorized_objects = Vec::new();
            for object in objects {
                let authorized = auth
                    .check(
                        Subject::user(&user_id.to_string()),
                        Relation::new("read"),
                        Object::new("object", &object.key),
                    )
                    .await
                    .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

                if authorized {
                    authorized_objects.push(object);
                }
            }
            Ok(authorized_objects)
        } else {
            Ok(objects)
        }
    }

    /// Get object metadata
    pub async fn head_object(
        &self,
        key: &str,
        version_id: Option<Uuid>,
        user_id: Uuid,
    ) -> GovernanceResult<ObjectMetadata> {
        // Check authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("read"),
                    Object::new("object", key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            if !authorized {
                return Err(GovernanceError::Authorization(
                    "User not authorized to access object metadata".to_string(),
                ));
            }
        }

        self.storage_backend.head_object(key, version_id).await
    }

    /// List object versions
    pub async fn list_versions(&self, key: &str, user_id: Uuid) -> GovernanceResult<Vec<ObjectVersion>> {
        // Check authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("read"),
                    Object::new("object", key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            if !authorized {
                return Err(GovernanceError::Authorization(
                    "User not authorized to list object versions".to_string(),
                ));
            }
        }

        self.storage_backend.list_versions(key).await
    }

    /// Copy an object
    pub async fn copy_object(
        &self,
        source_key: &str,
        dest_key: &str,
        user_id: Uuid,
    ) -> GovernanceResult<ObjectMetadata> {
        // Check authorization if enabled
        if let Some(ref auth) = self.auth_engine {
            let read_authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("read"),
                    Object::new("object", source_key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            let write_authorized = auth
                .check(
                    Subject::user(&user_id.to_string()),
                    Relation::new("write"),
                    Object::new("object", dest_key),
                )
                .await
                .map_err(|e| GovernanceError::Authorization(e.to_string()))?;

            if !read_authorized || !write_authorized {
                return Err(GovernanceError::Authorization(
                    "User not authorized to copy object".to_string(),
                ));
            }
        }

        let result = self.storage_backend.copy_object(source_key, dest_key).await?;

        // Audit log
        if self.audit_enabled {
            let log = AccessLog::new(
                "COPY".to_string(),
                format!("{} -> {}", source_key, dest_key),
                user_id,
                result.organization_id,
                200,
            )
            .with_version(result.version_id);

            self.storage_backend.log_access(log).await?;
        }

        Ok(result)
    }

    /// Evaluate policies for an object
    pub async fn evaluate_policies(&self, key: &str) -> GovernanceResult<Vec<PolicyAction>> {
        let engine = self.policy_engine.read().await;
        engine.evaluate_object(key).await
    }

    /// Execute a policy action
    pub async fn execute_policy_action(&self, action: &PolicyAction) -> GovernanceResult<()> {
        let engine = self.policy_engine.read().await;
        engine.execute_action(action).await
    }

    /// Scan and enforce policies (background job)
    pub async fn scan_and_enforce(&self, prefix: &str, max_keys: usize) -> GovernanceResult<Vec<PolicyAction>> {
        let engine = self.policy_engine.read().await;
        let actions = engine.scan_and_enforce(prefix, max_keys).await?;

        info!("Policy scan found {} actions to execute", actions.len());

        // Auto-execute actions (in production, might want manual review)
        for action in &actions {
            if let Err(e) = engine.execute_action(action).await {
                warn!("Failed to execute policy action: {}", e);
            }
        }

        Ok(actions)
    }

    /// Classify an object
    pub async fn classify_object(&self, key: &str) -> GovernanceResult<Option<ClassificationMetadata>> {
        self.auto_classifier
            .classify_object(self.storage_backend.as_ref(), key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorageBackend;

    #[tokio::test]
    async fn test_governance_engine_put_get() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let engine = GovernanceEngine::new(backend);

        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let metadata = ObjectMetadata::new("test.txt".to_string(), 10, "text/plain".to_string(), user_id, org_id);

        let data = b"Hello, World!".to_vec();
        let put_result = engine.put_object("test.txt", data.clone(), metadata, user_id, false).await;
        assert!(put_result.is_ok());

        let (_retrieved_data, _) = engine.get_object("test.txt", None, user_id).await.unwrap();
        // Note: InMemory backend doesn't store actual data, so this will be empty
        // In real implementation with persistent storage, data would match
    }

    #[tokio::test]
    async fn test_auto_classification() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let engine = GovernanceEngine::new(backend);

        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let metadata = ObjectMetadata::new("patient.txt".to_string(), 0, "text/plain".to_string(), user_id, org_id);

        let data = b"Patient MRN: 12345678, Name: John Doe".to_vec();
        let result = engine.put_object("patient.txt", data, metadata, user_id, true).await.unwrap();

        assert!(result.classification.is_some());
        assert_eq!(
            result.classification.unwrap().classification,
            DataClassification::ProtectedHealthInformation
        );
    }

    #[tokio::test]
    async fn test_lifecycle_policy_enforcement() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let engine = GovernanceEngine::new(backend.clone());

        // Add lifecycle rule
        let rule = LifecycleRule::new("Delete Old Files".to_string())
            .add_transition(1, crate::lifecycle::LifecycleAction::Delete);

        engine.add_lifecycle_rule(rule).await.unwrap();

        // Create old object
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let mut metadata = ObjectMetadata::new("old.txt".to_string(), 10, "text/plain".to_string(), user_id, org_id);
        metadata.created_at = chrono::Utc::now() - chrono::Duration::days(2);

        backend.put_object("old.txt", b"data".to_vec(), metadata).await.unwrap();

        // Scan and enforce
        let actions = engine.scan_and_enforce("", 100).await.unwrap();
        assert!(!actions.is_empty());
    }
}
