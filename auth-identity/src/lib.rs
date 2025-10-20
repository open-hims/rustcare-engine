//! Identity management and user authentication module for RustCare Engine
//! 
//! This module provides core identity management functionality including:
//! - User registration and authentication
//! - Password management and hashing
//! - JWT token generation and validation
//! - User profile management
//! - Role-based access control preparation
//! 
//! # Example
//! 
//! ```rust
//! use auth_identity::{IdentityService, User};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let service = IdentityService::new().await?;
//!     
//!     let user = service.register_user("user@example.com", "password123").await?;
//!     let token = service.authenticate("user@example.com", "password123").await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod models;
pub mod repository;
pub mod service;
pub mod handlers;
pub mod config;
pub mod error;

pub use models::*;
pub use service::*;
pub use error::*;