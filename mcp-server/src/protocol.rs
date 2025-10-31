//! MCP Protocol definitions (JSON-RPC based)
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// MCP JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version
    #[serde(default = "default_jsonrpc_version")]
    pub jsonrpc: String,
    /// Request ID
    pub id: Option<String>,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

fn default_jsonrpc_version() -> String {
    "2.0".to_string()
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version
    #[serde(default = "default_jsonrpc_version")]
    pub jsonrpc: String,
    /// Request ID (echoes request)
    pub id: Option<String>,
    /// Result payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpProtocolError>,
}

/// MCP JSON-RPC error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProtocolError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP capability descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name
    pub name: String,
    /// Capability description
    pub description: String,
    /// Capability type
    pub capability_type: CapabilityType,
}

/// Capability types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    Tool,
    Resource,
    Prompt,
    Sampler,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema
    pub input_schema: serde_json::Value,
}

/// Tool execution input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Execution status
    pub status: ToolStatus,
    /// Result data
    pub data: Option<serde_json::Value>,
    /// Error message (if any)
    pub error: Option<String>,
}

/// Tool execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    Success,
    Error,
}

/// List of supported MCP methods
pub mod methods {
    pub const INITIALIZE: &str = "initialize";
    pub const LIST_CAPABILITIES: &str = "capabilities/list";
    pub const LIST_TOOLS: &str = "tools/list";
    pub const CALL_TOOL: &str = "tools/call";
    pub const LIST_RESOURCES: &str = "resources/list";
    pub const READ_RESOURCE: &str = "resources/read";
}

