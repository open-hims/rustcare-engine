//! Example showing how to use the #[mcp_tool] decorator pattern
//!
//! This demonstrates how handler functions can be automatically exposed
//! as MCP tools using attribute macros similar to utoipa.

use mcp_server::tools::{McpTool, ToolInput, ToolResult, AuthContext};
use mcp_server::error::McpResult;
use uuid::Uuid;
use serde_json::Value;

// Example handler function with MCP tool decorator
#[mcp_server::mcp_tool(
    name = "list_pharmacies",
    description = "List all pharmacies for the organization",
    category = "pharmacy",
    requires_permission = "pharmacy:read",
    sensitive = false
)]
pub async fn list_pharmacies_handler(
    _params: Value,
    _auth: &AuthContext,
) -> McpResult<ToolResult> {
    // Handler implementation
    Ok(ToolResult {
        status: mcp_server::protocol::ToolStatus::Success,
        data: Some(serde_json::json!({
            "pharmacies": []
        })),
        error: None,
    })
}

// Example of a sensitive endpoint that should be excluded
#[mcp_server::mcp_tool(
    name = "rotate_secret",
    description = "Rotate a secret key",
    category = "secrets",
    requires_permission = "secrets:rotate",
    sensitive = true  // This will exclude it from public LLM access
)]
pub async fn rotate_secret_handler(
    _params: Value,
    _auth: &AuthContext,
) -> McpResult<ToolResult> {
    // This tool will be filtered out from public tool lists
    Ok(ToolResult {
        status: mcp_server::protocol::ToolStatus::Success,
        data: None,
        error: None,
    })
}

// Example showing how tools are automatically registered
fn main() {
    use mcp_server::tools::ToolsRegistry;
    
    let mut registry = ToolsRegistry::new();
    
    // Tools marked with #[mcp_tool] are automatically discovered
    // and registered by build.rs scanning the codebase
    
    // List available tools (sensitive ones excluded)
    let tools = registry.list(false);
    println!("Available tools: {:?}", tools);
}

