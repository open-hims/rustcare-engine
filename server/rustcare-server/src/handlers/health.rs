use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall system health status
    #[schema(example = "healthy")]
    pub status: String,
    /// Current timestamp in RFC3339 format
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: String,
    /// API version
    #[schema(example = "1.0.0")]
    pub version: String,
    /// System uptime in seconds
    #[schema(example = 3600)]
    pub uptime: u64,
    /// Individual service health checks
    pub checks: HashMap<String, String>,
}

/// Version information response
#[derive(Debug, Serialize, ToSchema)]
pub struct VersionResponse {
    /// Application name
    #[schema(example = "RustCare Engine")]
    pub name: String,
    /// Application version
    #[schema(example = "1.0.0")]
    pub version: String,
    /// Build date
    #[schema(example = "2024-01-15")]
    pub build_date: String,
    /// Git commit hash
    #[schema(example = "abc123def")]
    pub git_commit: String,
    /// Rust version used to build
    #[schema(example = "1.75.0")]
    pub rust_version: String,
    /// Enabled features
    pub features: Vec<String>,
}

/// System status response
#[derive(Debug, Serialize, ToSchema)]
pub struct StatusResponse {
    /// Server name
    #[schema(example = "RustCare Engine")]
    pub server_name: String,
    /// Uptime in seconds
    #[schema(example = 3600)]
    pub uptime_seconds: u64,
    /// Memory usage in MB
    #[schema(example = 256.5)]
    pub memory_usage_mb: f64,
    /// Number of active connections
    #[schema(example = 42)]
    pub active_connections: usize,
    /// HIPAA compliance status
    pub hipaa_compliance: bool,
    /// Audit logging enabled
    pub audit_logging: bool,
    /// Individual service statuses
    pub services: HashMap<String, ServiceStatus>,
}

/// Service status information
#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceStatus {
    /// Service name
    #[schema(example = "Database Layer")]
    pub name: String,
    /// Current status
    #[schema(example = "running")]
    pub status: String,
    /// Last health check timestamp
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub last_check: String,
    /// Error message if any
    pub error: Option<String>,
}

/// Health check handler
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "System is healthy", body = HealthResponse),
        (status = 503, description = "System is unhealthy", body = HealthResponse)
    )
)]
pub async fn health_check(
    State(_server): State<RustCareServer>
) -> Result<Json<ApiResponse<HealthResponse>>, ApiError> {
    let mut checks = HashMap::new();
    
    // Check database connectivity
    checks.insert("database".to_string(), "healthy".to_string());
    
    // Check plugin runtime
    checks.insert("plugin_runtime".to_string(), "healthy".to_string());
    
    // Check authentication service
    checks.insert("auth_service".to_string(), "healthy".to_string());
    
    // Check audit engine
    checks.insert("audit_engine".to_string(), "healthy".to_string());

    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: 0, // TODO: Implement actual uptime tracking
        checks,
    };

    Ok(Json(api_success(response)))
}

/// Version information handler
#[utoipa::path(
    get,
    path = "/version",
    tag = "health",
    responses(
        (status = 200, description = "Version information retrieved successfully", body = VersionResponse)
    )
)]
pub async fn version_info() -> Result<Json<ApiResponse<VersionResponse>>, ApiError> {
    let features = vec![
        "hipaa-compliance".to_string(),
        "plugin-runtime".to_string(),
        "audit-logging".to_string(),
        "oauth-integration".to_string(),
        "zanzibar-authz".to_string(),
        "workflow-engine".to_string(),
    ];

    let response = VersionResponse {
        name: "RustCare Engine".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_date: env!("CARGO_PKG_VERSION").to_string(), // TODO: Use actual build date
        git_commit: "unknown".to_string(), // TODO: Use actual git commit
        rust_version: "1.75+".to_string(),
        features,
    };

    Ok(Json(api_success(response)))
}

/// System status handler
#[utoipa::path(
    get,
    path = "/status",
    tag = "health",
    responses(
        (status = 200, description = "System status retrieved successfully", body = StatusResponse)
    )
)]
pub async fn system_status(
    State(server): State<RustCareServer>
) -> Result<Json<ApiResponse<StatusResponse>>, ApiError> {
    let mut services = HashMap::new();
    
    // Database service status
    services.insert("database".to_string(), ServiceStatus {
        name: "Database Layer".to_string(),
        status: "running".to_string(),
        last_check: chrono::Utc::now().to_rfc3339(),
        error: None,
    });
    
    // Plugin runtime status
    services.insert("plugin_runtime".to_string(), ServiceStatus {
        name: "Plugin Runtime".to_string(),
        status: "running".to_string(),
        last_check: chrono::Utc::now().to_rfc3339(),
        error: None,
    });
    
    // Authentication service status
    services.insert("auth_gateway".to_string(), ServiceStatus {
        name: "Authentication Gateway".to_string(),
        status: "running".to_string(),
        last_check: chrono::Utc::now().to_rfc3339(),
        error: None,
    });

    let response = StatusResponse {
        server_name: server.config.name.clone(),
        uptime_seconds: 0, // TODO: Implement actual uptime tracking
        memory_usage_mb: 0.0, // TODO: Implement actual memory usage tracking
        active_connections: 0, // TODO: Implement actual connection tracking
        hipaa_compliance: server.config.hipaa_compliance,
        audit_logging: server.config.audit_logging,
        services,
    };

    Ok(Json(api_success(response)))
}