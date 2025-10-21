// Core modules
pub mod error;
pub mod classification;
pub mod lifecycle;
pub mod storage;
pub mod backends;
pub mod policies;
pub mod governance;

// Re-exports
pub use error::{GovernanceError, GovernanceResult};
pub use classification::{ClassificationMetadata, DataClassification};
pub use lifecycle::{LifecycleAction, LifecycleRule, RetentionPolicy, StorageTier};
pub use storage::{AccessLog, ObjectMetadata, ObjectVersion, StorageBackend, InMemoryStorageBackend};
pub use backends::FileSystemBackend;

#[cfg(feature = "s3-backend")]
pub use backends::S3Backend;
pub use policies::{AutoClassifier, PolicyAction, PolicyEngine};
pub use governance::GovernanceEngine;

/// Comprehensive data governance and lifecycle management for RustCare Engine
/// 
/// This module provides enterprise-grade data governance capabilities including:
/// - Data classification and sensitivity labeling
/// - Automated data discovery and cataloging
/// - Data lineage tracking and impact analysis
/// - Privacy controls and GDPR compliance (Right to be Forgotten)
/// - Retention policies and automated archival/deletion
/// - Data quality monitoring and validation
/// - Data masking and anonymization
/// - Cross-border data transfer controls
/// - Consent management and purpose limitation
/// 
/// # Core Features
/// 
/// - **Data Discovery**: Automated scanning and cataloging of data sources
/// - **Classification**: ML-powered data sensitivity classification
/// - **Lineage**: End-to-end data flow tracking and dependency mapping
/// - **Retention**: Policy-based data retention and disposal
/// - **Privacy**: GDPR/CCPA compliance automation
/// - **Quality**: Data validation, profiling, and anomaly detection
/// - **Access Control**: Integration with authorization engine
/// - **Audit**: Comprehensive compliance reporting
/// 
/// # Example
/// 
/// ```rust
/// use object_governance::{GovernanceEngine, DataClassification, RetentionPolicy};
/// use chrono::Duration;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let engine = GovernanceEngine::new().await?;
///     
///     // Discover and classify data
///     let discovered = engine.discover_data_sources().await?;
///     for source in discovered {
///         let classification = engine.classify_data(&source).await?;
///         println!("Data source: {}, Classification: {:?}", source.name, classification);
///     }
///     
///     // Apply retention policy
///     let policy = RetentionPolicy::new()
///         .for_classification(DataClassification::PersonalData)
///         .retain_for(Duration::days(2555)) // 7 years
///         .then_delete();
///     
///     engine.apply_retention_policy(policy).await?;
///     
///     // Handle privacy requests
///     let request = engine.handle_data_subject_request(
///         "delete",
///         "user@example.com"
///     ).await?;
///     
///     Ok(())
/// }
/// ```

/// Main governance configuration structure
pub struct GovernanceConfig {
    /// Enable data classification
    pub classification_enabled: bool,
    /// Retention policy in days
    pub retention_days: u32,
}

/// Initialize governance with default configuration
pub fn init() -> GovernanceConfig {
    GovernanceConfig {
        classification_enabled: true,
        retention_days: 2555, // 7 years for HIPAA
    }
}