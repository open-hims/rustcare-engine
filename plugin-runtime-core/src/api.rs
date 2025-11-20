//! Plugin API definitions and interfaces
//! 
//! Defines the standard API that plugins can use to interact
//! with the RustCare healthcare platform.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use utoipa::ToSchema;

/// Main plugin API interface
#[async_trait]
pub trait PluginApi: Send + Sync {
    /// Get plugin information
    async fn get_info(&self) -> Result<PluginInfo, crate::error::PluginRuntimeError>;
    
    /// Initialize plugin with configuration
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), crate::error::PluginRuntimeError>;
    
    /// Execute plugin with input data
    async fn execute(&self, input: ApiInput) -> Result<ApiOutput, crate::error::PluginRuntimeError>;
    
    /// Cleanup plugin resources
    async fn cleanup(&self) -> Result<(), crate::error::PluginRuntimeError>;
    
    /// Health check
    async fn health_check(&self) -> Result<HealthStatus, crate::error::PluginRuntimeError>;
}

/// Plugin information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin ID
    pub id: Uuid,
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
    /// Supported API version
    pub api_version: String,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Configuration parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Plugin-specific settings
    pub settings: HashMap<String, String>,
    /// Resource limits
    pub resource_limits: Option<crate::sandbox::ResourceLimits>,
}

/// API input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInput {
    /// Request ID for tracking
    pub request_id: Uuid,
    /// Input data payload
    pub data: serde_json::Value,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Execution context
    pub context: ExecutionContext,
}

/// API output data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiOutput {
    /// Request ID (matches input)
    pub request_id: Uuid,
    /// Output data payload
    pub data: serde_json::Value,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Execution statistics
    pub statistics: ExecutionStatistics,
}

/// Execution context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// User ID (if available)
    pub user_id: Option<Uuid>,
    /// Session ID
    pub session_id: Option<String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional context data
    pub context_data: HashMap<String, serde_json::Value>,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionStatistics {
    /// Execution duration (milliseconds)
    pub duration_ms: u64,
    /// Memory used (bytes)
    pub memory_used: usize,
    /// CPU time (milliseconds)
    pub cpu_time_ms: u64,
    /// Operations performed
    pub operations_count: u32,
}

/// Health status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health status
    pub status: HealthLevel,
    /// Status message
    pub message: String,
    /// Detailed checks
    pub checks: Vec<HealthCheck>,
    /// Last check timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health level enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthLevel {
    /// Plugin is healthy and ready
    Healthy,
    /// Plugin has warnings but is functional
    Warning,
    /// Plugin is unhealthy and may not function properly
    Unhealthy,
    /// Plugin is in unknown state
    Unknown,
}

/// Individual health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthLevel,
    /// Check message
    pub message: String,
    /// Check duration (milliseconds)
    pub duration_ms: u64,
}