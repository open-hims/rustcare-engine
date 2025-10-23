//! Business services layer for RustCare Engine
//!
//! Services orchestrate between infrastructure (email, events, storage)
//! and domain logic (repositories, models).

pub mod organization_service;

pub use organization_service::OrganizationService;
