//! Plugin runtime environment management
//! 
//! Provides the core runtime environment for executing plugins safely
//! with resource limits and sandboxing capabilities.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Plugin runtime environment
pub struct PluginRuntime {
    /// Runtime ID
    pub id: Uuid,
    /// Runtime configuration
    pub config: RuntimeConfig,
    /// Active plugin instances
    pub plugins: Arc<RwLock<HashMap<Uuid, Box<dyn PluginInstance>>>>,
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum memory per plugin (bytes)
    pub max_memory: usize,
    /// Maximum CPU time per execution (milliseconds)
    pub max_cpu_time: u64,
    /// Enable sandboxing
    pub sandbox_enabled: bool,
    /// Plugin timeout (seconds)
    pub timeout_seconds: u64,
}

/// Plugin instance trait
pub trait PluginInstance: Send + Sync {
    /// Execute plugin with given input
    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, crate::error::PluginRuntimeError>;
    
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Check if plugin is healthy
    fn health_check(&self) -> bool;
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory: 128 * 1024 * 1024, // 128MB
            max_cpu_time: 5000,            // 5 seconds
            sandbox_enabled: true,
            timeout_seconds: 30,
        }
    }
}