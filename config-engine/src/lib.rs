//! Dynamic configuration management engine for RustCare Engine
//! 
//! This module provides comprehensive configuration management with:
//! - Multiple configuration sources (files, environment, remote stores)
//! - Real-time configuration updates and hot-reloading
//! - Configuration validation and schema enforcement
//! - Encryption for sensitive configuration values
//! - Template-based configuration generation
//! - Multi-environment support (dev, staging, prod)
//! - Configuration versioning and rollback
//! - Audit trails for configuration changes
//! 
//! # Supported Sources
//! 
//! - **Local Files**: YAML, TOML, JSON configuration files
//! - **Environment Variables**: System and container environment
//! - **Remote Stores**: etcd, Consul, HashiCorp Vault
//! - **Databases**: PostgreSQL, MongoDB for large configurations
//! - **Cloud Services**: AWS Parameter Store, Azure Key Vault, GCP Secret Manager
//! 
//! # Example
//! 
//! ```rust
//! use config_engine::{ConfigEngine, ConfigSource};
//! use serde::Deserialize;
//! 
//! #[derive(Deserialize)]
//! struct AppConfig {
//!     database_url: String,
//!     api_key: String,
//!     log_level: String,
//! }
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut engine = ConfigEngine::new()
//!         .add_source(ConfigSource::file("config.yaml"))
//!         .add_source(ConfigSource::env())
//!         .add_source(ConfigSource::etcd("localhost:2379"))
//!         .build()
//!         .await?;
//!     
//!     let config: AppConfig = engine.get().await?;
//!     
//!     // Watch for changes
//!     let mut watcher = engine.watch::<AppConfig>().await?;
//!     while let Some(new_config) = watcher.next().await {
//!         println!("Configuration updated: {:?}", new_config);
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod engine;
pub mod providers;
pub mod watchers;
pub mod validation;
pub mod encryption;
pub mod templates;
pub mod error;

pub use engine::*;
pub use providers::*;
pub use error::*;

// Re-export all public types and traits for easy access
