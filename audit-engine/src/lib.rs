//! Comprehensive audit logging and compliance engine for RustCare Engine
//! 
//! This module provides enterprise-grade audit capabilities including:
//! - Tamper-evident audit logging with cryptographic integrity
//! - Compliance reporting for HIPAA, SOX, GDPR, and other regulations
//! - Real-time audit trail generation and monitoring
//! - Advanced search and filtering capabilities
//! - Data retention policies and automated archival
//! - Audit log integrity verification using Merkle trees
//! - Export capabilities for compliance audits
//! - Privacy controls and data anonymization
//! 
//! # Audit Event Types
//! 
//! - **Authentication Events**: Login, logout, password changes
//! - **Authorization Events**: Permission grants, access attempts
//! - **Data Access Events**: Read, write, delete operations
//! - **Administrative Events**: Configuration changes, user management
//! - **System Events**: Service starts/stops, errors, performance metrics
//! - **Business Events**: Transactions, approvals, state changes
//! 
//! # Example
//! 
//! ```rust
//! use audit_engine::{AuditEngine, AuditEntry, EventType, Subject};
//! use serde_json::json;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = AuditEngine::new().await?;
//!     
//!     // Log an audit event
//!     let entry = AuditEntry::new(
//!         EventType::DataAccess,
//!         Subject::user("alice"),
//!         "patient_record_viewed",
//!         json!({
//!             "patient_id": "P123456",
//!             "record_type": "medical_history",
//!             "access_reason": "treatment_planning"
//!         })
//!     );
//!     
//!     engine.log(entry).await?;
//!     
//!     // Search audit logs
//!     let results = engine.search()
//!         .subject("alice")
//!         .event_type(EventType::DataAccess)
//!         .date_range("2024-01-01", "2024-12-31")
//!         .execute()
//!         .await?;
//!     
//!     // Generate compliance report
//!     let report = engine.generate_compliance_report("HIPAA", "2024-Q1").await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod engine;
pub mod trail;
pub mod entry;
pub mod storage;
pub mod compliance;
pub mod merkle;
pub mod search;
pub mod export;
pub mod error;

pub use engine::*;
pub use trail::*;
pub use entry::*;
pub use error::*;