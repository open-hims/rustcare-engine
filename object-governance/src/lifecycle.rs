use crate::classification::DataClassification;
use crate::error::{GovernanceError, GovernanceResult};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Storage tier for object lifecycle management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageTier {
    /// Hot storage - frequent access, low latency
    Hot,
    /// Warm storage - infrequent access, moderate latency
    Warm,
    /// Cold storage - rare access, high latency, lower cost
    Cold,
    /// Archive - long-term retention, very high latency
    Archive,
}

/// Lifecycle action to take when transition occurs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleAction {
    /// Transition to different storage tier
    Transition(StorageTier),
    /// Delete the object permanently
    Delete,
    /// Archive and compress
    ArchiveAndCompress,
    /// Anonymize PII/PHI
    Anonymize,
    /// Trigger review for compliance
    ReviewForCompliance,
}

/// Lifecycle rule for automated object management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRule {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub classification: Option<DataClassification>,
    pub prefix: Option<String>,
    pub tags: Vec<String>,
    pub transitions: Vec<LifecycleTransition>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single transition in a lifecycle rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTransition {
    pub days_after_creation: u32,
    pub action: LifecycleAction,
}

impl LifecycleRule {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            enabled: true,
            classification: None,
            prefix: None,
            tags: Vec::new(),
            transitions: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn for_classification(mut self, classification: DataClassification) -> Self {
        self.classification = Some(classification);
        self
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn add_transition(mut self, days: u32, action: LifecycleAction) -> Self {
        self.transitions.push(LifecycleTransition {
            days_after_creation: days,
            action,
        });
        // Sort transitions by days
        self.transitions.sort_by_key(|t| t.days_after_creation);
        self
    }

    /// Check if this rule applies to the given object
    pub fn matches(
        &self,
        classification: Option<DataClassification>,
        key: &str,
        tags: &[String],
    ) -> bool {
        if !self.enabled {
            return false;
        }

        // Check classification filter
        if let Some(rule_classification) = self.classification {
            if classification != Some(rule_classification) {
                return false;
            }
        }

        // Check prefix filter
        if let Some(ref prefix) = self.prefix {
            if !key.starts_with(prefix) {
                return false;
            }
        }

        // Check tags filter
        if !self.tags.is_empty() {
            let has_all_tags = self.tags.iter().all(|tag| tags.contains(tag));
            if !has_all_tags {
                return false;
            }
        }

        true
    }

    /// Get the next action to take for an object created at the given time
    pub fn next_action(&self, created_at: DateTime<Utc>) -> Option<&LifecycleAction> {
        let now = Utc::now();
        let age_days = (now - created_at).num_days() as u32;

        self.transitions
            .iter()
            .find(|t| t.days_after_creation <= age_days)
            .map(|t| &t.action)
    }

    /// Validate the lifecycle rule
    pub fn validate(&self) -> GovernanceResult<()> {
        if self.name.is_empty() {
            return Err(GovernanceError::Lifecycle("Rule name cannot be empty".to_string()));
        }

        if self.transitions.is_empty() {
            return Err(GovernanceError::Lifecycle("Rule must have at least one transition".to_string()));
        }

        // Ensure transitions are properly ordered
        for window in self.transitions.windows(2) {
            if window[0].days_after_creation >= window[1].days_after_creation {
                return Err(GovernanceError::Lifecycle(
                    "Transitions must be in ascending order by days".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// Retention policy for compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub id: Uuid,
    pub name: String,
    pub classification: DataClassification,
    pub retain_days: u32,
    pub action_on_expiry: LifecycleAction,
    pub legal_hold_supported: bool,
    pub created_at: DateTime<Utc>,
}

impl RetentionPolicy {
    pub fn new(name: String, classification: DataClassification, retain_days: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            classification,
            retain_days,
            action_on_expiry: LifecycleAction::Delete,
            legal_hold_supported: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_action_on_expiry(mut self, action: LifecycleAction) -> Self {
        self.action_on_expiry = action;
        self
    }

    /// Check if the retention period has expired for an object
    pub fn is_expired(&self, created_at: DateTime<Utc>) -> bool {
        let now = Utc::now();
        let age = now - created_at;
        age >= Duration::days(self.retain_days as i64)
    }

    /// Validate against classification minimum retention requirements
    pub fn validate(&self) -> GovernanceResult<()> {
        if let Some(min_days) = self.classification.minimum_retention_days() {
            if self.retain_days < min_days {
                return Err(GovernanceError::Retention(format!(
                    "Retention period {} days is less than minimum {} days for {:?}",
                    self.retain_days, min_days, self.classification
                )));
            }
        }

        if let Some(max_days) = self.classification.maximum_retention_days() {
            if self.retain_days > max_days {
                return Err(GovernanceError::Retention(format!(
                    "Retention period {} days exceeds maximum {} days for {:?}",
                    self.retain_days, max_days, self.classification
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_rule_creation() {
        let rule = LifecycleRule::new("PHI Archive Rule".to_string())
            .for_classification(DataClassification::ProtectedHealthInformation)
            .with_prefix("medical-records/".to_string())
            .add_transition(90, LifecycleAction::Transition(StorageTier::Cold))
            .add_transition(365, LifecycleAction::ArchiveAndCompress)
            .add_transition(2555, LifecycleAction::ReviewForCompliance);

        assert!(rule.validate().is_ok());
        assert_eq!(rule.transitions.len(), 3);
        assert_eq!(rule.transitions[0].days_after_creation, 90);
    }

    #[test]
    fn test_lifecycle_rule_matching() {
        let rule = LifecycleRule::new("Test Rule".to_string())
            .for_classification(DataClassification::ProtectedHealthInformation)
            .with_prefix("patient/".to_string())
            .with_tag("phi".to_string());

        assert!(rule.matches(
            Some(DataClassification::ProtectedHealthInformation),
            "patient/12345/record.json",
            &["phi".to_string()]
        ));

        assert!(!rule.matches(
            Some(DataClassification::Public),
            "patient/12345/record.json",
            &["phi".to_string()]
        ));

        assert!(!rule.matches(
            Some(DataClassification::ProtectedHealthInformation),
            "reports/summary.pdf",
            &["phi".to_string()]
        ));
    }

    #[test]
    fn test_retention_policy_validation() {
        // Valid PHI retention (7 years minimum)
        let policy = RetentionPolicy::new(
            "PHI Retention".to_string(),
            DataClassification::ProtectedHealthInformation,
            2555,
        );
        assert!(policy.validate().is_ok());

        // Invalid - too short for PHI
        let policy = RetentionPolicy::new(
            "Invalid PHI".to_string(),
            DataClassification::ProtectedHealthInformation,
            365,
        );
        assert!(policy.validate().is_err());

        // Invalid - too long for PII (GDPR 3 years max)
        let policy = RetentionPolicy::new(
            "Invalid PII".to_string(),
            DataClassification::PersonallyIdentifiableInformation,
            365 * 5,
        );
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_retention_expiry() {
        let policy = RetentionPolicy::new(
            "Test Policy".to_string(),
            DataClassification::Internal,
            30,
        );

        let old_date = Utc::now() - Duration::days(31);
        assert!(policy.is_expired(old_date));

        let recent_date = Utc::now() - Duration::days(29);
        assert!(!policy.is_expired(recent_date));
    }
}
