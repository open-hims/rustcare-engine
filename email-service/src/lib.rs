pub mod service;
pub mod templates;
pub mod encryption;
pub mod compliance;
pub mod delivery;
pub mod authentication;
pub mod tracking;
pub mod queue;
pub mod error;

pub use service::*;
pub use templates::*;
pub use encryption::*;
pub use compliance::*;
pub use error::*;

/// HIPAA-compliant email service with encryption and comprehensive audit logging
/// 
/// This module provides enterprise-grade email capabilities specifically designed
/// for healthcare environments, leveraging Stalwart Labs' excellent email libraries
/// for robust, secure, and compliant email handling.
/// 
/// # Key Features
/// 
/// - **HIPAA Compliance**: Automatic PHI detection and encryption
/// - **End-to-End Encryption**: TLS and S/MIME support for email security
/// - **DKIM/SPF/DMARC**: Complete email authentication with Stalwart's mail-auth
/// - **Template Engine**: Handlebars-based templating with PII redaction
/// - **Delivery Tracking**: Comprehensive delivery status and bounce handling
/// - **Audit Logging**: Complete audit trail of all email operations
/// - **Queue Management**: Reliable email queue with retry and DLQ support
/// - **Multi-Provider**: Support for SMTP, SES, SendGrid, and other providers
/// - **Rate Limiting**: Configurable rate limits and throttling
/// 
/// # Email Security
/// 
/// - **Automatic Encryption**: PHI content is automatically encrypted
/// - **Digital Signatures**: DKIM signing for email authenticity
/// - **Content Filtering**: Automatic PII/PHI detection and redaction
/// - **Secure Transmission**: Enforced TLS for all email transmission
/// - **Access Controls**: Role-based access to email functionality
/// - **Retention Policies**: Configurable email retention for compliance
/// 
/// # Supported Providers
/// 
/// - **SMTP**: Direct SMTP server integration
/// - **Amazon SES**: AWS Simple Email Service
/// - **SendGrid**: Twilio SendGrid integration
/// - **Mailgun**: Mailgun email service
/// - **Postmark**: Postmark transactional email
/// - **Microsoft Graph**: Office 365 email API
/// - **Custom**: Extensible provider system
/// 
/// # Example Usage
/// 
/// ```rust
/// use email_service::{EmailService, EmailMessage, EmailConfig, EncryptionPolicy};
/// use serde_json::json;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = EmailConfig::new()
///         .with_provider("smtp")
///         .with_host("smtp.example.com")
///         .with_credentials("username", "password")
///         .with_encryption_policy(EncryptionPolicy::RequireForPHI)
///         .with_dkim_signing(true)
///         .with_audit_logging(true);
///     
///     let email_service = EmailService::new(config).await?;
///     
///     // Send a simple email
///     let message = EmailMessage::new()
///         .to("doctor@hospital.com")
///         .from("system@rustcare.dev")
///         .subject("Patient Appointment Reminder")
///         .template("appointment_reminder")
///         .context(json!({
///             "patient_name": "John Doe",
///             "appointment_date": "2024-10-25",
///             "doctor_name": "Dr. Smith"
///         }));
///     
///     let result = email_service.send(message).await?;
///     println!("Email sent: {}", result.message_id);
///     
///     Ok(())
/// 
/// # Template System
/// 
/// ```rust
/// use email_service::{TemplateEngine, TemplateConfig};
/// 
/// let template_engine = TemplateEngine::new(TemplateConfig {
///     template_dir: "templates/".into(),
///     auto_redact_pii: true,
///     escape_html: true,
///     cache_enabled: true,
/// });
/// 
/// // Templates automatically redact PII
/// let rendered = template_engine.render("patient_discharge", json!({
///     "patient_name": "John Doe",          // Will be redacted in logs
///     "ssn": "123-45-6789",              // Will be redacted
///     "discharge_date": "2024-10-25",     // Safe to log
///     "hospital_name": "General Hospital" // Safe to log
/// })).await?;
/// 
/// # Email Encryption
/// 
/// ```rust
/// use email_service::{EmailEncryption, EncryptionType};
/// 
/// // Automatic PHI detection and encryption
/// let encryption = EmailEncryption::new()
///     .with_automatic_detection(true)
///     .with_encryption_type(EncryptionType::SMIME)
///     .with_certificate_path("certs/email.p12");
///     
/// let encrypted_message = encryption.encrypt_if_needed(&message).await?;
/// 
/// # Compliance Features
/// 
/// ```rust
/// use email_service::{ComplianceLogger, EmailAuditEvent};
/// 
/// // Comprehensive audit logging
/// let audit_event = EmailAuditEvent::new()
///     .with_user_id("doctor123")
///     .with_action("email_sent")
///     .with_recipient("patient@example.com")
///     .with_template("appointment_reminder")
///     .with_phi_detected(true)
///     .with_encryption_used(true);
///     
/// ComplianceLogger::log_email_event(audit_event).await?;
/// 
/// # Queue Management
/// 
/// ```rust
/// use email_service::{EmailQueue, QueueConfig, RetryPolicy};
/// 
/// let queue_config = QueueConfig::new()
///     .with_max_retries(3)
///     .with_retry_policy(RetryPolicy::Exponential)
///     .with_dead_letter_queue(true)
///     .with_batch_size(100);
///     
/// let queue = EmailQueue::new(queue_config).await?;
/// 
/// // Queue will automatically retry failed sends
/// queue.enqueue(message).await?;
/// 
/// # Configuration Example
/// 
/// ```yaml
/// email:
///   provider: "smtp"
///   smtp:
///     host: "smtp.example.com"
///     port: 587
///     username: "${SMTP_USERNAME}"
///     password: "${SMTP_PASSWORD}"
///     tls: true
///   
///   encryption:
///     policy: "require_for_phi"
///     smime:
///       certificate_path: "certs/email.p12"
///       private_key_path: "certs/email.key"
///   
///   authentication:
///     dkim:
///       enabled: true
///       domain: "rustcare.dev"
///       selector: "default"
///       private_key_path: "certs/dkim.key"
///   
///   compliance:
///     audit_all_emails: true
///     pii_detection: true
///     retention_days: 2555  # 7 years
///     redact_logs: true
///   
///   queue:
///     max_retries: 3
///     retry_delay: "30s"
///     batch_size: 50
///     dead_letter_queue: true
///   
///   templates:
///     directory: "templates/"
///     cache_enabled: true
///     auto_escape: true
///     pii_redaction: true
/// ```

/// Main email service configuration
pub struct EmailConfig {
    /// SMTP server configuration
    pub smtp_host: String,
}