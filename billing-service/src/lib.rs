//! Billing Service for Healthcare Revenue Cycle Management
//! 
//! Provides comprehensive billing capabilities including:
//! - Charge capture from clinical encounters
//! - Claims generation (UB-04, HCFA-1500, 837P/I)
//! - Payment processing and reconciliation
//! - Denial management and appeals
//! - Revenue reporting and analytics

pub mod service;
pub mod models;
pub mod claims;
pub mod payment;
pub mod reporting;
pub mod error;

pub use service::*;
pub use models::*;
pub use claims::*;
pub use payment::*;
pub use reporting::*;
pub use error::*;

