//! Plugin loader implementation
//! 
//! Handles loading plugins from various sources including WASM modules
//! and native shared libraries with security validation.

use std::path::Path;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Plugin loader for different plugin types
pub struct PluginLoader {
    /// Loader configuration
    config: LoaderConfig,
}

/// Loader configuration
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Plugin directory path
    pub plugin_dir: String,
    /// Enable signature verification
    pub verify_signatures: bool,
    /// Allowed plugin types
    pub allowed_types: Vec<PluginType>,
}

/// Plugin type enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PluginType {
    /// WebAssembly plugin
    Wasm,
    /// Native shared library
    Native,
    /// JavaScript plugin
    JavaScript,
}

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin unique identifier
    pub id: Uuid,
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Entry point file
    pub entry_point: String,
    /// Required permissions
    pub permissions: Vec<String>,
    /// Plugin dependencies
    pub dependencies: Vec<String>,
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new(config: LoaderConfig) -> Self {
        Self { config }
    }
    
    /// Load plugin from path
    pub async fn load_plugin(&self, _path: &Path) -> Result<Box<dyn crate::runtime::PluginInstance>, crate::error::PluginRuntimeError> {
        // Implementation for loading plugins
        todo!("Implement plugin loading")
    }
    
    /// Validate plugin manifest
    pub fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), crate::error::PluginRuntimeError> {
        // Validate plugin manifest
        if manifest.name.is_empty() {
            return Err(crate::error::PluginRuntimeError::InvalidManifest("Plugin name cannot be empty".to_string()));
        }
        
        if !self.config.allowed_types.contains(&manifest.plugin_type) {
            return Err(crate::error::PluginRuntimeError::UnsupportedPluginType);
        }
        
        Ok(())
    }
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            plugin_dir: "./plugins".to_string(),
            verify_signatures: true,
            allowed_types: vec![PluginType::Wasm, PluginType::Native],
        }
    }
}