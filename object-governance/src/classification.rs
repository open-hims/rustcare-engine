use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Data classification levels for healthcare compliance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataClassification {
    /// Public data - no restrictions
    Public,
    /// Internal use only
    Internal,
    /// Confidential business data
    Confidential,
    /// Protected Health Information (PHI) - HIPAA regulated
    ProtectedHealthInformation,
    /// Personally Identifiable Information (PII) - GDPR/CCPA regulated
    PersonallyIdentifiableInformation,
    /// Financial data - PCI-DSS regulated
    Financial,
    /// Research data with special handling requirements
    Research,
}

impl DataClassification {
    /// Check if this classification requires encryption at rest
    pub fn requires_encryption(&self) -> bool {
        matches!(
            self,
            DataClassification::ProtectedHealthInformation
                | DataClassification::PersonallyIdentifiableInformation
                | DataClassification::Financial
        )
    }

    /// Check if this classification requires audit logging
    pub fn requires_audit(&self) -> bool {
        !matches!(self, DataClassification::Public)
    }

    /// Get minimum retention period in days
    pub fn minimum_retention_days(&self) -> Option<u32> {
        match self {
            DataClassification::ProtectedHealthInformation => Some(2555), // 7 years HIPAA
            DataClassification::Financial => Some(2555),                  // 7 years
            DataClassification::Research => Some(365 * 10),               // 10 years
            _ => None,
        }
    }

    /// Get maximum retention period in days (for privacy compliance)
    pub fn maximum_retention_days(&self) -> Option<u32> {
        match self {
            DataClassification::PersonallyIdentifiableInformation => Some(365 * 3), // 3 years GDPR
            _ => None,
        }
    }
}

/// Classification metadata for an object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationMetadata {
    pub classification: DataClassification,
    pub classified_at: chrono::DateTime<chrono::Utc>,
    pub classified_by: Option<Uuid>,
    pub confidence: f32,
    pub tags: Vec<String>,
    pub sensitivity_labels: Vec<String>,
}

impl ClassificationMetadata {
    pub fn new(classification: DataClassification) -> Self {
        Self {
            classification,
            classified_at: chrono::Utc::now(),
            classified_by: None,
            confidence: 1.0,
            tags: Vec::new(),
            sensitivity_labels: Vec::new(),
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.classified_by = Some(user_id);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn add_sensitivity_label(&mut self, label: String) {
        if !self.sensitivity_labels.contains(&label) {
            self.sensitivity_labels.push(label);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_encryption_requirements() {
        assert!(DataClassification::ProtectedHealthInformation.requires_encryption());
        assert!(DataClassification::PersonallyIdentifiableInformation.requires_encryption());
        assert!(DataClassification::Financial.requires_encryption());
        assert!(!DataClassification::Public.requires_encryption());
        assert!(!DataClassification::Internal.requires_encryption());
    }

    #[test]
    fn test_retention_periods() {
        assert_eq!(
            DataClassification::ProtectedHealthInformation.minimum_retention_days(),
            Some(2555)
        );
        assert_eq!(
            DataClassification::PersonallyIdentifiableInformation.maximum_retention_days(),
            Some(365 * 3)
        );
        assert_eq!(DataClassification::Public.minimum_retention_days(), None);
    }

    #[test]
    fn test_classification_metadata() {
        let mut metadata = ClassificationMetadata::new(DataClassification::ProtectedHealthInformation)
            .with_confidence(0.95)
            .with_tags(vec!["patient-data".to_string(), "diagnosis".to_string()]);

        metadata.add_sensitivity_label("PHI".to_string());
        metadata.add_tag("medical-record".to_string());

        assert_eq!(metadata.classification, DataClassification::ProtectedHealthInformation);
        assert_eq!(metadata.confidence, 0.95);
        assert_eq!(metadata.tags.len(), 3);
        assert_eq!(metadata.sensitivity_labels.len(), 1);
    }
}
