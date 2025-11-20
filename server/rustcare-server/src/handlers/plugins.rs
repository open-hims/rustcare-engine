//! Plugin Management Handlers
//!
//! Handlers for managing plugins: install, load, execute, list, etc.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    error::{ApiError, ApiResponse, api_success},
    server::RustCareServer,
    middleware::AuthContext,
    // handlers::ui_components::RegisterComponentRequest,
};

use plugin_runtime_core::{
    api::{PluginInfo, ApiInput, ApiOutput, ExecutionContext, PluginApi, PluginConfig, HealthStatus, HealthLevel},
    lifecycle::{LifecycleManager, PluginState},
    error::PluginRuntimeError,
};
use async_trait::async_trait;

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

/// Plugin information response
#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct PluginInfoResponse {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub api_version: String,
    pub state: String,
    pub installed_at: String,
    pub last_accessed: String,
}

/// Plugin list response
#[derive(Debug, Serialize, ToSchema)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginInfoResponse>,
    pub total: usize,
}

/// Execute plugin request
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecutePluginRequest {
    pub function_name: String,
    pub input_data: serde_json::Value,
    pub metadata: Option<HashMap<String, String>>,
}

/// Install plugin request
#[derive(Debug, Deserialize, ToSchema)]
pub struct InstallPluginRequest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub api_version: String,
    pub entry_point: String,
    pub plugin_type: String, // "wasm" or "native"
    pub plugin_data: Option<String>, // Base64 encoded plugin binary
    pub manifest_data: Option<serde_json::Value>, // Plugin manifest JSON
}

// ============================================================================
// PLUGIN INSTANCE WRAPPER
// ============================================================================

/// Simple plugin instance wrapper for testing/development
/// In production, this would wrap actual WASM or native plugin instances
struct SimplePluginInstance {
    info: PluginInfo,
    state: Arc<RwLock<PluginState>>,
}

#[async_trait]
impl PluginApi for SimplePluginInstance {
    async fn get_info(&self) -> Result<PluginInfo, PluginRuntimeError> {
        Ok(self.info.clone())
    }

    async fn initialize(&mut self, _config: PluginConfig) -> Result<(), PluginRuntimeError> {
        let mut state = self.state.write().await;
        *state = PluginState::Ready;
        Ok(())
    }

    async fn execute(&self, input: ApiInput) -> Result<ApiOutput, PluginRuntimeError> {
        // Simple echo implementation for testing
        // In production, this would execute actual plugin code
        let output = ApiOutput {
            request_id: input.request_id,
            data: serde_json::json!({
                "echo": input.data,
                "message": "Plugin executed successfully (simple implementation)",
                "plugin": self.info.name,
            }),
            metadata: input.metadata,
            statistics: plugin_runtime_core::ExecutionStatistics {
                duration_ms: 10,
                memory_used: 1024,
                cpu_time_ms: 5,
                operations_count: 1,
            },
        };
        Ok(output)
    }

    async fn cleanup(&self) -> Result<(), PluginRuntimeError> {
        let mut state = self.state.write().await;
        *state = PluginState::Stopped;
        Ok(())
    }

    async fn health_check(&self) -> Result<HealthStatus, PluginRuntimeError> {
        let state = self.state.read().await;
        let status = match *state {
            PluginState::Ready | PluginState::Running => HealthLevel::Healthy,
            PluginState::Error(_) => HealthLevel::Unhealthy,
            _ => HealthLevel::Warning,
        };

        Ok(HealthStatus {
            status,
            message: format!("Plugin state: {:?}", *state),
            checks: vec![],
            timestamp: chrono::Utc::now(),
        })
    }
}

// ============================================================================
// HANDLERS
// ============================================================================

/// List all installed plugins
#[utoipa::path(
    get,
    path = "/api/v1/plugins",
    responses(
        (status = 200, description = "List of plugins", body = PluginListResponse)
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_plugins(
    State(server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PluginListResponse>>, ApiError> {
    let runtime = server.get_plugin_runtime();
    let plugins_map = runtime.list_plugins().await;
    
    let plugins: Vec<PluginInfoResponse> = plugins_map
        .into_iter()
        .map(|(id, (info, state))| {
            PluginInfoResponse {
                id,
                name: info.name,
                version: info.version,
                description: info.description,
                author: info.author,
                api_version: info.api_version,
                state: format!("{:?}", state),
                installed_at: chrono::Utc::now().to_rfc3339(), // TODO: Get actual timestamp from entry
                last_accessed: chrono::Utc::now().to_rfc3339(), // TODO: Get actual timestamp from entry
            }
        })
        .collect();
    
    let response = PluginListResponse {
        plugins: plugins.clone(),
        total: plugins.len(),
    };
    
    Ok(Json(api_success(response)))
}

/// Get plugin information by ID
#[utoipa::path(
    get,
    path = "/api/v1/plugins/{plugin_id}",
    responses(
        (status = 200, description = "Plugin information", body = PluginInfoResponse),
        (status = 404, description = "Plugin not found")
    ),
    params(
        ("plugin_id" = Uuid, Path, description = "Plugin ID")
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_plugin(
    Path(plugin_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PluginInfoResponse>>, ApiError> {
    let runtime = server.get_plugin_runtime();
    let plugins_map = runtime.list_plugins().await;
    
    if let Some((info, state)) = plugins_map.get(&plugin_id) {
        let response = PluginInfoResponse {
            id: plugin_id,
            name: info.name.clone(),
            version: info.version.clone(),
            description: info.description.clone(),
            author: info.author.clone(),
            api_version: info.api_version.clone(),
            state: format!("{:?}", state),
            installed_at: chrono::Utc::now().to_rfc3339(),
            last_accessed: chrono::Utc::now().to_rfc3339(),
        };
        Ok(Json(api_success(response)))
    } else {
        Err(ApiError::not_found("Plugin not found"))
    }
}

/// Install a new plugin
#[utoipa::path(
    post,
    path = "/api/v1/plugins",
    request_body = InstallPluginRequest,
    responses(
        (status = 201, description = "Plugin installed", body = PluginInfoResponse)
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn install_plugin(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(req): Json<InstallPluginRequest>,
) -> Result<Json<ApiResponse<PluginInfoResponse>>, ApiError> {
    // Validate request
    if req.name.is_empty() {
        return Err(ApiError::validation("Plugin name is required"));
    }
    if req.version.is_empty() {
        return Err(ApiError::validation("Plugin version is required"));
    }
    if req.author.is_empty() {
        return Err(ApiError::validation("Plugin author is required"));
    }

    // Create plugin info
    let plugin_id = Uuid::new_v4();
    let plugin_info = PluginInfo {
        id: plugin_id,
        name: req.name.clone(),
        version: req.version.clone(),
        description: req.description.clone(),
        author: req.author.clone(),
        api_version: req.api_version.clone(),
    };

    // Create plugin instance
    // TODO: In production, create actual WASM or native plugin instance based on plugin_type
    // For now, use simple wrapper for testing
    let plugin_state = Arc::new(RwLock::new(PluginState::Installed));
    let plugin_instance: Box<dyn PluginApi> = Box::new(SimplePluginInstance {
        info: plugin_info.clone(),
        state: plugin_state.clone(),
    });

    // Install plugin via lifecycle manager
    let runtime = server.get_plugin_runtime();
    runtime.install_plugin(plugin_info.clone(), plugin_instance)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to install plugin: {}", e)))?;

    // Register UI components if plugin provides them
    // In production, this would call the plugin's register_ui_components function
    // For now, we'll register components based on plugin metadata
    if req.plugin_type == "wasm" {
        // TODO: Execute plugin's register_ui_components function and register components
        // This is a placeholder showing the integration pattern
        tracing::info!(
            plugin_id = %plugin_id,
            "Plugin may register UI components - check plugin manifest"
        );
    }

    // Log installation
    tracing::info!(
        plugin_id = %plugin_id,
        plugin_name = %plugin_info.name,
        user_id = %auth.user_id,
        "Plugin installed successfully"
    );

    // Return plugin info
    let response = PluginInfoResponse {
        id: plugin_id,
        name: plugin_info.name,
        version: plugin_info.version,
        description: plugin_info.description,
        author: plugin_info.author,
        api_version: plugin_info.api_version,
        state: "Installed".to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        last_accessed: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(api_success(response)))
}

/// Load and initialize a plugin
#[utoipa::path(
    post,
    path = "/api/v1/plugins/{plugin_id}/load",
    responses(
        (status = 200, description = "Plugin loaded successfully")
    ),
    params(
        ("plugin_id" = Uuid, Path, description = "Plugin ID")
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn load_plugin(
    Path(plugin_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    let runtime = server.get_plugin_runtime();
    
    runtime.load_plugin(plugin_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to load plugin: {}", e)))?;

    tracing::info!(plugin_id = %plugin_id, "Plugin loaded successfully");
    Ok(StatusCode::OK)
}

/// Execute a plugin function
#[utoipa::path(
    post,
    path = "/api/v1/plugins/{plugin_id}/execute",
    request_body = ExecutePluginRequest,
    responses(
        (status = 200, description = "Plugin executed successfully", body = ApiOutput)
    ),
    params(
        ("plugin_id" = Uuid, Path, description = "Plugin ID")
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn execute_plugin(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Path(plugin_id): Path<Uuid>,
    Json(req): Json<ExecutePluginRequest>,
) -> Result<Json<ApiResponse<ApiOutput>>, ApiError> {
    // Create execution context from auth
    let context = ExecutionContext {
        user_id: Some(auth.user_id),
        session_id: None,
        timestamp: chrono::Utc::now(),
        context_data: HashMap::new(),
    };
    
    let input = ApiInput {
        request_id: Uuid::new_v4(),
        data: req.input_data,
        metadata: req.metadata.unwrap_or_default(),
        context,
    };
    
    // Execute plugin via lifecycle manager
    let runtime = server.get_plugin_runtime();
    let output = runtime.execute_plugin(plugin_id, input)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to execute plugin: {}", e)))?;

    tracing::info!(
        plugin_id = %plugin_id,
        function = %req.function_name,
        user_id = %auth.user_id,
        "Plugin executed successfully"
    );

    Ok(Json(api_success(output)))
}

/// Stop and unload a plugin
#[utoipa::path(
    post,
    path = "/api/v1/plugins/{plugin_id}/stop",
    responses(
        (status = 200, description = "Plugin stopped successfully")
    ),
    params(
        ("plugin_id" = Uuid, Path, description = "Plugin ID")
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn stop_plugin(
    Path(plugin_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    let runtime = server.get_plugin_runtime();
    
    runtime.stop_plugin(plugin_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to stop plugin: {}", e)))?;

    tracing::info!(plugin_id = %plugin_id, "Plugin stopped successfully");
    Ok(StatusCode::OK)
}

/// Uninstall a plugin
#[utoipa::path(
    delete,
    path = "/api/v1/plugins/{plugin_id}",
    responses(
        (status = 204, description = "Plugin uninstalled successfully")
    ),
    params(
        ("plugin_id" = Uuid, Path, description = "Plugin ID")
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn uninstall_plugin(
    Path(plugin_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    let runtime = server.get_plugin_runtime();
    
    runtime.uninstall_plugin(plugin_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to uninstall plugin: {}", e)))?;

    tracing::info!(plugin_id = %plugin_id, "Plugin uninstalled successfully");
    Ok(StatusCode::NO_CONTENT)
}

/// Get plugin health status
#[utoipa::path(
    get,
    path = "/api/v1/plugins/{plugin_id}/health",
    responses(
        (status = 200, description = "Plugin health status")
    ),
    params(
        ("plugin_id" = Uuid, Path, description = "Plugin ID")
    ),
    tag = "plugins",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn plugin_health(
    Path(plugin_id): Path<Uuid>,
    State(_server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // TODO: Get plugin health via runtime
    // For now, return basic health status
    let health = serde_json::json!({
        "plugin_id": plugin_id,
        "status": "healthy",
        "message": "Plugin is operational"
    });
    
    Ok(Json(api_success(health)))
}
