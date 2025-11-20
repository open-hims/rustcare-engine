use crate::classification::{ClassificationMetadata, DataClassification};
use crate::error::GovernanceResult;
use crate::lifecycle::{LifecycleRule, RetentionPolicy};
use crate::storage::{ObjectMetadata, StorageBackend};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// Policy engine for governance rules
pub struct PolicyEngine {
    lifecycle_rules: Vec<LifecycleRule>,
    retention_policies: Vec<RetentionPolicy>,
    storage_backend: Arc<dyn StorageBackend>,
}

impl PolicyEngine {
    pub fn new(storage_backend: Arc<dyn StorageBackend>) -> Self {
        Self {
            lifecycle_rules: Vec::new(),
            retention_policies: Vec::new(),
            storage_backend,
        }
    }

    /// Add a lifecycle rule
    pub fn add_lifecycle_rule(&mut self, rule: LifecycleRule) -> GovernanceResult<()> {
        rule.validate()?;
        self.lifecycle_rules.push(rule);
        Ok(())
    }

    /// Add a retention policy
    pub fn add_retention_policy(&mut self, policy: RetentionPolicy) -> GovernanceResult<()> {
        policy.validate()?;
        self.retention_policies.push(policy);
        Ok(())
    }

    /// Get applicable lifecycle rules for an object
    pub fn get_applicable_rules(&self, metadata: &ObjectMetadata) -> Vec<&LifecycleRule> {
        let classification = metadata.classification.as_ref().map(|c| c.classification);
        let tag_keys: Vec<String> = metadata.tags.keys().cloned().collect();

        self.lifecycle_rules
            .iter()
            .filter(|rule| rule.matches(classification, &metadata.key, &tag_keys))
            .collect()
    }

    /// Get applicable retention policy for an object
    pub fn get_retention_policy(&self, classification: DataClassification) -> Option<&RetentionPolicy> {
        self.retention_policies
            .iter()
            .find(|p| p.classification == classification)
    }

    /// Evaluate policies for an object and return recommended actions
    pub async fn evaluate_object(&self, key: &str) -> GovernanceResult<Vec<PolicyAction>> {
        let metadata = self.storage_backend.head_object(key, None).await?;
        let mut actions = Vec::new();

        // Check lifecycle rules
        let applicable_rules = self.get_applicable_rules(&metadata);
        for rule in applicable_rules {
            if let Some(action) = rule.next_action(metadata.created_at) {
                actions.push(PolicyAction::LifecycleTransition {
                    key: key.to_string(),
                    rule_id: rule.id,
                    action: action.clone(),
                    scheduled_at: Utc::now(),
                });
            }
        }

        // Check retention policies
        if let Some(classification) = metadata.classification.as_ref() {
            if let Some(policy) = self.get_retention_policy(classification.classification) {
                if policy.is_expired(metadata.created_at) && metadata.can_delete() {
                    actions.push(PolicyAction::RetentionExpired {
                        key: key.to_string(),
                        policy_id: policy.id,
                        action: policy.action_on_expiry.clone(),
                        expired_at: Utc::now(),
                    });
                }
            }
        }

        // Check legal holds and retention locks
        if metadata.legal_hold {
            info!("Object {} is under legal hold, no deletion allowed", key);
        }

        if let Some(retention_until) = metadata.retention_until {
            if Utc::now() < retention_until {
                info!("Object {} is under retention until {}", key, retention_until);
            }
        }

        Ok(actions)
    }

    /// Execute a policy action
    pub async fn execute_action(&self, action: &PolicyAction) -> GovernanceResult<()> {
        match action {
            PolicyAction::LifecycleTransition { key, action, .. } => {
                self.execute_lifecycle_action(key, action).await?;
            }
            PolicyAction::RetentionExpired { key, action, .. } => {
                self.execute_lifecycle_action(key, action).await?;
            }
        }
        Ok(())
    }

    async fn execute_lifecycle_action(
        &self,
        key: &str,
        action: &crate::lifecycle::LifecycleAction,
    ) -> GovernanceResult<()> {
        use crate::lifecycle::LifecycleAction;

        match action {
            LifecycleAction::Transition(tier) => {
                info!("Transitioning {} to {:?} storage tier", key, tier);
                // In real implementation, would call storage backend to change tier
                Ok(())
            }
            LifecycleAction::Delete => {
                info!("Deleting {} due to policy", key);
                self.storage_backend.delete_object(key, None).await?;
                Ok(())
            }
            LifecycleAction::ArchiveAndCompress => {
                info!("Archiving and compressing {}", key);
                // In real implementation, would compress and move to archive tier
                Ok(())
            }
            LifecycleAction::Anonymize => {
                info!("Anonymizing PII/PHI in {}", key);
                // In real implementation, would apply anonymization transformations
                Ok(())
            }
            LifecycleAction::ReviewForCompliance => {
                warn!("Object {} requires manual compliance review", key);
                // In real implementation, would create a review ticket
                Ok(())
            }
        }
    }

    /// Scan all objects and evaluate policies (background job)
    pub async fn scan_and_enforce(&self, prefix: &str, max_keys: usize) -> GovernanceResult<Vec<PolicyAction>> {
        let objects = self.storage_backend.list_objects(prefix, max_keys).await?;
        let mut all_actions = Vec::new();

        for object in objects {
            let actions = self.evaluate_object(&object.key).await?;
            all_actions.extend(actions);
        }

        Ok(all_actions)
    }
}

/// Actions recommended by policy evaluation
#[derive(Debug, Clone)]
pub enum PolicyAction {
    LifecycleTransition {
        key: String,
        rule_id: Uuid,
        action: crate::lifecycle::LifecycleAction,
        scheduled_at: DateTime<Utc>,
    },
    RetentionExpired {
        key: String,
        policy_id: Uuid,
        action: crate::lifecycle::LifecycleAction,
        expired_at: DateTime<Utc>,
    },
}

/// Auto-classification engine (simplified version)
pub struct AutoClassifier {
    patterns: Vec<ClassificationPattern>,
}

impl AutoClassifier {
    pub fn new() -> Self {
        Self {
            patterns: Self::default_patterns(),
        }
    }

    fn default_patterns() -> Vec<ClassificationPattern> {
        vec![
            ClassificationPattern {
                name: "SSN".to_string(),
                pattern: regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
                classification: DataClassification::PersonallyIdentifiableInformation,
                confidence: 0.9,
            },
            ClassificationPattern {
                name: "Credit Card".to_string(),
                pattern: regex::Regex::new(r"\b\d{4}[- ]?\d{4}[- ]?\d{4}[- ]?\d{4}\b").unwrap(),
                classification: DataClassification::Financial,
                confidence: 0.85,
            },
            ClassificationPattern {
                name: "Medical Record Number".to_string(),
                pattern: regex::Regex::new(r"\bMRN[:\s]*\d{6,10}\b").unwrap(),
                classification: DataClassification::ProtectedHealthInformation,
                confidence: 0.95,
            },
            ClassificationPattern {
                name: "Email".to_string(),
                pattern: regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
                classification: DataClassification::PersonallyIdentifiableInformation,
                confidence: 0.7,
            },
        ]
    }

    /// Classify content based on patterns
    pub fn classify_content(&self, content: &str) -> Option<ClassificationMetadata> {
        let mut best_match: Option<(&ClassificationPattern, usize)> = None;

        for pattern in &self.patterns {
            if let Some(_matches) = pattern.pattern.find_iter(content).next() {
                let count = pattern.pattern.find_iter(content).count();
                if let Some((_, current_count)) = best_match {
                    if count > current_count {
                        best_match = Some((pattern, count));
                    }
                } else {
                    best_match = Some((pattern, count));
                }
            }
        }

        best_match.map(|(pattern, count)| {
            let confidence = (pattern.confidence * (1.0 + (count as f32 * 0.1))).min(1.0);
            ClassificationMetadata::new(pattern.classification)
                .with_confidence(confidence)
                .with_tags(vec![pattern.name.clone()])
        })
    }

    /// Classify an object by scanning its content
    pub async fn classify_object(
        &self,
        storage: &dyn StorageBackend,
        key: &str,
    ) -> GovernanceResult<Option<ClassificationMetadata>> {
        let (data, _) = storage.get_object(key, None).await?;

        // Only classify text content (limit to first 1MB for performance)
        let content = if data.len() <= 1_000_000 {
            String::from_utf8_lossy(&data).to_string()
        } else {
            String::from_utf8_lossy(&data[..1_000_000]).to_string()
        };

        Ok(self.classify_content(&content))
    }
}

impl Default for AutoClassifier {
    fn default() -> Self {
        Self::new()
    }
}

struct ClassificationPattern {
    name: String,
    pattern: regex::Regex,
    classification: DataClassification,
    confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorageBackend;

    #[test]
    fn test_auto_classifier() {
        let classifier = AutoClassifier::new();

        let content = "Patient MRN: 12345678, SSN: 123-45-6789";
        let result = classifier.classify_content(content);

        assert!(result.is_some());
        let metadata = result.unwrap();
        assert!(matches!(
            metadata.classification,
            DataClassification::ProtectedHealthInformation | DataClassification::PersonallyIdentifiableInformation
        ));
        assert!(metadata.confidence > 0.7);
    }

    #[tokio::test]
    async fn test_policy_engine_evaluation() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let mut engine = PolicyEngine::new(backend.clone());

        // Add lifecycle rule
        let rule = crate::lifecycle::LifecycleRule::new("Test Rule".to_string())
            .for_classification(DataClassification::Internal)
            .add_transition(1, crate::lifecycle::LifecycleAction::Delete);

        engine.add_lifecycle_rule(rule).unwrap();

        // Create old object
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let mut metadata = ObjectMetadata::new("old.txt".to_string(), 10, "text/plain".to_string(), user_id, org_id);
        metadata.classification = Some(ClassificationMetadata::new(DataClassification::Internal));
        metadata.created_at = Utc::now() - chrono::Duration::days(2);

        backend.put_object("old.txt", b"data".to_vec(), metadata).await.unwrap();

        // Evaluate policies
        let actions = engine.evaluate_object("old.txt").await.unwrap();
        assert!(!actions.is_empty());
    }
}
