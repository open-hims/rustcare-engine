//! HIPAA-compliant email service with encryption and comprehensive audit logging
//! 
//! This module provides enterprise-grade email capabilities specifically designed
//! for healthcare environments.
//! 
//! # Key Features
//! 
//! - **Multi-Provider Support**: SMTP, Gmail, SES, SendGrid, Mailgun, Mailchimp, Postmark, Resend
//! - **HIPAA Compliance**: Automatic PHI detection and encryption
//! - **End-to-End Encryption**: TLS and S/MIME support for email security
//! - **Template Engine**: Handlebars-based templating with PII redaction
//! - **Delivery Tracking**: Comprehensive delivery status and bounce handling
//! - **Audit Logging**: Complete audit trail of all email operations

pub mod service;
pub mod templates;
pub mod encryption;
pub mod compliance;
pub mod delivery;
pub mod authentication;
pub mod tracking;
pub mod queue;
pub mod error;
pub mod verification;

pub use service::*;
pub use templates::*;
pub use encryption::*;
pub use compliance::*;
pub use error::*;
pub use verification::verify_mailbox_exists;
