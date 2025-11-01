//! Accounting Service for Healthcare Financial Management
//! 
//! Provides financial accounting capabilities including:
//! - Accounts Receivable (A/R) tracking
//! - General Ledger integration
//! - Financial reporting
//! - Month-end reconciliation
//! - Chart of Accounts management

pub mod service;
pub mod models;
pub mod ledger;
pub mod receivables;
pub mod reporting;
pub mod error;

pub use service::*;
pub use models::*;
pub use ledger::*;
pub use receivables::*;
pub use reporting::*;
pub use error::*;

