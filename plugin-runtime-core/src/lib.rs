//! Plugin runtime and extension system for RustCare Engine (Phase 1.5)
//! 
//! This module provides a secure, sandboxed plugin system supporting:
//! - WebAssembly (WASM) plugins for maximum security and portability
//! - Native dynamic library plugins for performance-critical extensions
//! - Plugin lifecycle management (install, enable, disable, update, uninstall)
//! - Resource isolation and security sandboxing
//! - Plugin API versioning and backward compatibility
//! - Inter-plugin communication through event bus
//! - Plugin marketplace and registry integration
//! - Hot-plugging without service restart
//! - Resource quotas and rate limiting
//! 
//! # Plugin Types
//! 
//! - **Authentication Plugins**: Custom auth providers and MFA methods
//! - **Workflow Plugins**: Custom task types and business logic
//! - **Data Connectors**: External system integrations
//! - **UI Extensions**: Custom dashboards and user interfaces
//! - **Compliance Plugins**: Industry-specific regulations
//! - **Analytics Plugins**: Custom metrics and reporting
//! - **Notification Plugins**: Alert channels and communication
//! 
//! # Security Model
//! 
//! - Capability-based security with explicit permission grants
//! - Resource isolation using WASM sandbox
//! - Network access controls and proxy routing
//! - File system access restrictions
//! - Memory and CPU quotas per plugin
//! - Code signing and integrity verification
//! - Runtime monitoring and anomaly detection
//! 
//! # Example
//! 
//! ```rust
//! use plugin_runtime_core::{PluginRuntime, PluginManifest, SecurityPolicy};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let runtime = PluginRuntime::new()
//!         .with_wasm_support()
//!         .with_native_support()
//!         .with_security_policy(SecurityPolicy::strict())
//!         .build()
//!         .await?;
//!     
//!     // Load a plugin
//!     let manifest = PluginManifest::from_file("plugins/auth-saml/manifest.toml")?;
//!     let plugin = runtime.load_plugin(manifest).await?;
//!     
//!     // Enable the plugin
//!     runtime.enable_plugin(plugin.id()).await?;
//!     
//!     // Call plugin function
//!     let result = runtime.call_plugin(
//!         plugin.id(),
//!         "authenticate",
//!         serde_json::json!({
//!             "saml_response": "..."
//!         })
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod runtime;
pub mod loader;
pub mod sandbox;
pub mod api;
pub mod lifecycle;
pub mod security;
pub mod wasm;
pub mod native;
pub mod error;

pub use runtime::*;
pub use loader::*;
pub use sandbox::*;
pub use api::*;
pub use error::*;