//! Zanzibar-style authorization engine for RustCare Engine
//! 
//! This module implements a Google Zanzibar-inspired authorization system providing:
//! - Relationship-based access control (ReBAC)
//! - Fine-grained permissions with inheritance
//! - Efficient authorization checks with graph traversal
//! - Schema validation and consistency checking
//! - Support for complex permission hierarchies
//! 
//! # Core Concepts
//! 
//! - **Object**: Any resource that can be protected (e.g., document, folder, organization)
//! - **Subject**: Any entity that can have permissions (e.g., user, group, service account)
//! - **Relation**: The type of relationship between subject and object (e.g., owner, editor, viewer)
//! - **Tuple**: A relationship statement: "subject has relation to object"
//! 
//! # Example
//! 
//! ```rust
//! use auth_zanzibar::{AuthorizationEngine, Tuple, Object, Subject, Relation};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = AuthorizationEngine::new().await?;
//!     
//!     // Define relationships
//!     let tuple = Tuple::new(
//!         Subject::user("alice"),
//!         Relation::new("editor"),
//!         Object::new("document", "doc1"),
//!     );
//!     
//!     // Write relationship
//!     engine.write_tuple(tuple).await?;
//!     
//!     // Check permission
//!     let allowed = engine.check(
//!         Subject::user("alice"),
//!         Relation::new("edit"),
//!         Object::new("document", "doc1"),
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod models;
pub mod engine;
pub mod repository;
pub mod schema;
pub mod check;
pub mod expand;
pub mod error;

pub use models::*;
pub use engine::*;
pub use schema::*;
pub use error::*;