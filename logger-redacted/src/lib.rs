pub mod redactor;
pub mod formatters;
pub mod filters;
pub mod compliance;
pub mod macros;
pub mod audit;
pub mod config;

pub use redactor::*;
pub use formatters::*;
pub use filters::*;
pub use compliance::*;
pub use config::*;

/// HIPAA-compliant logging system with automatic PII redaction
/// 
/// This module provides a sophisticated logging system designed specifically for
/// healthcare applications where PII (Personally Identifiable Information) and
/// PHI (Protected Health Information) must be automatically detected and redacted
/// from log messages to maintain HIPAA compliance.
/// 
/// # Key Features
/// 
/// - **Automatic PII/PHI Detection**: ML-powered detection of sensitive data
/// - **Real-time Redaction**: Sanitizes log messages before writing
/// - **Compliance Logging**: Separate audit logs for compliance requirements
/// - **Pattern-based Filtering**: Configurable regex patterns for data types
/// - **Hash-based Correlation**: Redacted values can be correlated using hashes
/// - **Performance Optimized**: Minimal impact on application performance
/// - **Structured Logging**: JSON-formatted logs with proper field separation
/// - **Log Retention**: Automatic log rotation and retention policies
/// 
/// # Detected Data Types
/// 
/// - **Email Addresses**: user@example.com → u***@e*****.com
/// - **Phone Numbers**: (555) 123-4567 → (***) ***-****
/// - **SSN**: 123-45-6789 → ***-**-****
/// - **Credit Cards**: 4111-1111-1111-1111 → ****-****-****-1111
/// - **IP Addresses**: 192.168.1.1 → 192.***.*.***
/// - **Medical Record Numbers**: MRN123456 → MRN******
/// - **Names**: Pattern-based name detection and redaction
/// - **Addresses**: Street addresses and postal codes
/// - **Custom Patterns**: Configurable organization-specific patterns
/// 
/// # Example
/// 
/// ```rust
/// use logger_redacted::{RedactedLogger, LogLevel, PiiRedactionConfig};
/// use tracing::{info, error};
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize redacted logger
///     let config = PiiRedactionConfig::default()
///         .with_email_redaction(true)
///         .with_phone_redaction(true)
///         .with_custom_pattern(r"\bMRN\d+", "MRN[REDACTED]");
///     
///     RedactedLogger::init(config).await?;
///     
///     // These logs will be automatically redacted
///     info!(
///         user_id = "user123",
///         "User john.doe@example.com logged in from 192.168.1.100"
///     );
///     // Output: "User j***@e*****.com logged in from 192.***.*.***"
///     
///     error!(
///         patient_mrn = "MRN123456", // This field will be hashed
///         "Failed to process patient with phone (555) 123-4567"
///     );
///     // Output: "Failed to process patient with phone (***) ***-****"
///     
///     Ok(())
/// 
/// # Compliance Features
/// 
/// ```rust
/// use logger_redacted::{compliance_log, AuditEvent, ComplianceLevel};
/// 
/// // Separate compliance logging
/// compliance_log!(
///     level = ComplianceLevel::HIPAA,
///     event = AuditEvent::DataAccess,
///     user_id = "hashed_user_id",
///     resource = "patient_record",
///     action = "view",
///     "Patient record accessed"
/// );
/// 
/// # Configuration
/// 
/// ```yaml
/// logging:
///   redaction:
///     enabled: true
///     patterns:
///       email: true
///       phone: true
///       ssn: true
///       credit_card: true
///       ip_address: true
///       custom:
///         - pattern: "\\bMRN\\d+"
///           replacement: "MRN[REDACTED]"
///         - pattern: "\\bPatient\\s+\\w+"
///           replacement: "Patient [NAME]"
///   
///   compliance:
///     audit_log_file: "/var/log/rustcare/audit.log"
///     retention_days: 2555  # 7 years for HIPAA
///     encryption: true
///     
///   performance:
///     async_logging: true
///     buffer_size: 8192
///     batch_size: 100
/// ```

/// Main logging configuration structure
pub struct LoggerConfig {
    /// Enable PII redaction
    pub redaction_enabled: bool,
    /// Async logging mode
    pub async_logging: bool,
}