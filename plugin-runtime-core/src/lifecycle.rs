//! Plugin lifecycle management
//! 
//! Manages the complete lifecycle of plugins from installation
//! through execution to cleanup and removal.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Plugin lifecycle manager
pub struct LifecycleManager {
    /// Active plugins registry
    plugins: Arc<RwLock<HashMap<Uuid, PluginEntry>>>,
    /// Lifecycle configuration
    config: LifecycleConfig,
}

/// Plugin lifecycle configuration
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Maximum concurrent plugins
    pub max_concurrent_plugins: usize,
    /// Plugin timeout for lifecycle operations
    pub lifecycle_timeout_seconds: u64,
    /// Enable automatic cleanup
    pub auto_cleanup_enabled: bool,
    /// Cleanup interval (seconds)
    pub cleanup_interval_seconds: u64,
}

/// Plugin registry entry
pub struct PluginEntry {
    /// Plugin metadata
    pub metadata: crate::api::PluginInfo,
    /// Current state
    pub state: PluginState,
    /// Plugin instance
    pub instance: Option<Box<dyn crate::api::PluginApi>>,
    /// Installation timestamp
    pub installed_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub last_accessed: DateTime<Utc>,
    /// Usage statistics
    pub usage_stats: UsageStatistics,
}

/// Plugin lifecycle state
#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    /// Plugin is installed but not loaded
    Installed,
    /// Plugin is loaded and initializing
    Loading,
    /// Plugin is ready for execution
    Ready,
    /// Plugin is currently executing
    Running,
    /// Plugin is paused
    Paused,
    /// Plugin encountered an error
    Error(String),
    /// Plugin is shutting down
    Stopping,
    /// Plugin is stopped
    Stopped,
}

/// Plugin usage statistics
#[derive(Debug, Clone, Default)]
pub struct UsageStatistics {
    /// Total executions
    pub total_executions: u64,
    /// Total execution time (milliseconds)
    pub total_execution_time_ms: u64,
    /// Average execution time (milliseconds)
    pub avg_execution_time_ms: f64,
    /// Last execution timestamp
    pub last_execution: Option<DateTime<Utc>>,
    /// Error count
    pub error_count: u64,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Install a new plugin
    pub async fn install_plugin(
        &self,
        plugin_info: crate::api::PluginInfo,
        plugin_instance: Box<dyn crate::api::PluginApi>,
    ) -> Result<Uuid, crate::error::PluginRuntimeError> {
        let mut plugins = self.plugins.write().await;
        
        // Check concurrent plugin limit
        if plugins.len() >= self.config.max_concurrent_plugins {
            return Err(crate::error::PluginRuntimeError::ResourceLimitExceeded(
                "Maximum concurrent plugins reached".to_string(),
            ));
        }
        
        let plugin_id = plugin_info.id;
        let entry = PluginEntry {
            metadata: plugin_info,
            state: PluginState::Installed,
            instance: Some(plugin_instance),
            installed_at: Utc::now(),
            last_accessed: Utc::now(),
            usage_stats: UsageStatistics::default(),
        };
        
        plugins.insert(plugin_id, entry);
        Ok(plugin_id)
    }
    
    /// Load and initialize a plugin
    pub async fn load_plugin(&self, plugin_id: Uuid) -> Result<(), crate::error::PluginRuntimeError> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(entry) = plugins.get_mut(&plugin_id) {
            entry.state = PluginState::Loading;
            
            if let Some(instance) = &mut entry.instance {
                let config = crate::api::PluginConfig {
                    parameters: HashMap::new(),
                    settings: HashMap::new(),
                    resource_limits: None,
                };
                
                match instance.initialize(config).await {
                    Ok(_) => {
                        entry.state = PluginState::Ready;
                        entry.last_accessed = Utc::now();
                        Ok(())
                    }
                    Err(e) => {
                        entry.state = PluginState::Error(e.to_string());
                        Err(e)
                    }
                }
            } else {
                entry.state = PluginState::Error("No plugin instance available".to_string());
                Err(crate::error::PluginRuntimeError::InvalidState("No plugin instance".to_string()))
            }
        } else {
            Err(crate::error::PluginRuntimeError::PluginNotFound(plugin_id))
        }
    }
    
    /// Execute a plugin
    pub async fn execute_plugin(
        &self,
        plugin_id: Uuid,
        input: crate::api::ApiInput,
    ) -> Result<crate::api::ApiOutput, crate::error::PluginRuntimeError> {
        let start_time = std::time::Instant::now();
        
        // Update state to running
        {
            let mut plugins = self.plugins.write().await;
            if let Some(entry) = plugins.get_mut(&plugin_id) {
                if entry.state != PluginState::Ready {
                    return Err(crate::error::PluginRuntimeError::InvalidState(
                        format!("Plugin not ready, current state: {:?}", entry.state),
                    ));
                }
                entry.state = PluginState::Running;
                entry.last_accessed = Utc::now();
            } else {
                return Err(crate::error::PluginRuntimeError::PluginNotFound(plugin_id));
            }
        }
        
        // Execute plugin
        let result = {
            let plugins = self.plugins.read().await;
            if let Some(entry) = plugins.get(&plugin_id) {
                if let Some(instance) = &entry.instance {
                    instance.execute(input).await
                } else {
                    Err(crate::error::PluginRuntimeError::InvalidState("No plugin instance".to_string()))
                }
            } else {
                Err(crate::error::PluginRuntimeError::PluginNotFound(plugin_id))
            }
        };
        
        // Update state and statistics
        {
            let mut plugins = self.plugins.write().await;
            if let Some(entry) = plugins.get_mut(&plugin_id) {
                let duration = start_time.elapsed();
                entry.usage_stats.total_executions += 1;
                entry.usage_stats.total_execution_time_ms += duration.as_millis() as u64;
                entry.usage_stats.avg_execution_time_ms = 
                    entry.usage_stats.total_execution_time_ms as f64 / entry.usage_stats.total_executions as f64;
                entry.usage_stats.last_execution = Some(Utc::now());
                
                match &result {
                    Ok(_) => {
                        entry.state = PluginState::Ready;
                    }
                    Err(_) => {
                        entry.usage_stats.error_count += 1;
                        entry.state = PluginState::Error("Execution failed".to_string());
                    }
                }
            }
        }
        
        result
    }
    
    /// Stop and unload a plugin
    pub async fn stop_plugin(&self, plugin_id: Uuid) -> Result<(), crate::error::PluginRuntimeError> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(entry) = plugins.get_mut(&plugin_id) {
            entry.state = PluginState::Stopping;
            
            if let Some(instance) = &entry.instance {
                match instance.cleanup().await {
                    Ok(_) => {
                        entry.state = PluginState::Stopped;
                        Ok(())
                    }
                    Err(e) => {
                        entry.state = PluginState::Error(e.to_string());
                        Err(e)
                    }
                }
            } else {
                entry.state = PluginState::Stopped;
                Ok(())
            }
        } else {
            Err(crate::error::PluginRuntimeError::PluginNotFound(plugin_id))
        }
    }
    
    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: Uuid) -> Result<(), crate::error::PluginRuntimeError> {
        // First stop the plugin
        self.stop_plugin(plugin_id).await?;
        
        // Remove from registry
        let mut plugins = self.plugins.write().await;
        plugins.remove(&plugin_id);
        
        Ok(())
    }
    
    /// Get plugin state
    pub async fn get_plugin_state(&self, plugin_id: Uuid) -> Result<PluginState, crate::error::PluginRuntimeError> {
        let plugins = self.plugins.read().await;
        
        if let Some(entry) = plugins.get(&plugin_id) {
            Ok(entry.state.clone())
        } else {
            Err(crate::error::PluginRuntimeError::PluginNotFound(plugin_id))
        }
    }
    
    /// List all plugins with their states
    pub async fn list_plugins(&self) -> HashMap<Uuid, (crate::api::PluginInfo, PluginState)> {
        let plugins = self.plugins.read().await;
        
        plugins
            .iter()
            .map(|(id, entry)| (*id, (entry.metadata.clone(), entry.state.clone())))
            .collect()
    }
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            max_concurrent_plugins: 10,
            lifecycle_timeout_seconds: 30,
            auto_cleanup_enabled: true,
            cleanup_interval_seconds: 300, // 5 minutes
        }
    }
}