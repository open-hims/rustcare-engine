//! Business services layer for RustCare Engine
//!
//! Services orchestrate between infrastructure (email, events, storage)
//! and domain logic (repositories, models).

pub mod organization_service;
pub mod compliance_service;
pub mod audit;

pub use organization_service::OrganizationService;
pub use compliance_service::ComplianceService;
pub use audit::AuditService;
