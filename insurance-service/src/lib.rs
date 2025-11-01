//! Insurance Service for Healthcare
//! 
//! Provides insurance management capabilities including:
//! - Eligibility verification (real-time and batch)
//! - Prior authorization management
//! - Insurance plan configuration
//! - Benefit coverage tracking
//! - Pre-certification workflows

pub mod service;
pub mod models;
pub mod eligibility;
pub mod authorization;
pub mod error;

pub use service::*;
pub use models::*;
pub use eligibility::*;
pub use authorization::*;
pub use error::*;

