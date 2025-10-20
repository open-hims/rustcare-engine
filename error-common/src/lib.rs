//! Common error handling utilities for RustCare Engine
//! 
//! This module provides standardized error types, error codes, and utilities
//! used across all RustCare Engine modules. It ensures consistent error handling,
//! proper error context preservation, and secure error reporting.
//! 
//! # Key Features
//! 
//! - **Standardized Error Types**: Common error patterns across modules
//! - **Error Codes**: Structured error codes for API responses
//! - **Context Preservation**: Detailed error context without sensitive data exposure
//! - **Error Sanitization**: Automatic PII removal from error messages
//! - **Error Recovery**: Structured retry and recovery mechanisms
//! - **Observability**: Integration with tracing and metrics
//! - **Healthcare Compliance**: HIPAA-compliant error handling
//! 
//! # Error Categories
//! 
//! - **ValidationError**: Input validation and data format errors
//! - **AuthenticationError**: Authentication and session errors
//! - **AuthorizationError**: Permission and access control errors
//! - **DatabaseError**: Database connection and query errors
//! - **NetworkError**: HTTP, gRPC, and network communication errors
//! - **BusinessLogicError**: Domain-specific business rule violations
//! - **SystemError**: Infrastructure and system-level errors
//! - **ComplianceError**: Regulatory and compliance violations
//! 
//! # Example
//! 
//! ```rust
//! use error_common::{RustCareError, ErrorCode, ErrorContext};
//! 
//! fn validate_patient_data(data: &str) -> Result<PatientData, RustCareError> {
//!     if data.is_empty() {
//!         return Err(RustCareError::validation()
//!             .with_code(ErrorCode::INVALID_INPUT)
//!             .with_message("Patient data cannot be empty")
//!             .with_context("field", "patient_data")
//!             .build());
//!     }
//!     
//!     // Validation logic...
//!     Ok(PatientData::parse(data)?)
//! }
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), RustCareError> {
//!     match validate_patient_data("") {
//!         Ok(data) => println!("Valid data: {:?}", data),
//!         Err(e) => {
//!             // Error is automatically sanitized and logged
//!             tracing::error!(
//!                 error_code = %e.code(),
//!                 error_type = %e.error_type(),
//!                 "Validation failed"
//!             );
//!             return Err(e);
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod types;
pub mod context;
pub mod codes;
pub mod reporting;
pub mod recovery;
pub mod sanitization;

pub use types::*;
pub use context::*;
pub use codes::*;
pub use reporting::*;