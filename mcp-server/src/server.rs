//! MCP Server implementation
use crate::protocol::{McpRequest, McpResponse};
use crate::tools::ToolsRegistry;
use crate::capabilities::CapabilitiesRegistry;
use crate::error::{McpError, McpResult};
use async_channel::{Receiver, Sender};
use tracing::{info, debug, error};

/// MCP Server
pub struct Server {
    capabilities: CapabilitiesRegistry,
    tools: ToolsRegistry,
    running: bool,
}

impl Server {
    /// Create a new MCP server
    pub fn new() -> Self {
        info!("Initializing MCP Server");
        
        Self {
            capabilities: CapabilitiesRegistry::new(),
            tools: ToolsRegistry::new(),
            running: false,
        }
    }

    /// Start the MCP server
    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Starting MCP Server");
        
        // TODO: Implement actual server startup
        // - Initialize transport (stdio or HTTP)
        // - Start JSON-RPC message loop
        // - Handle incoming requests
        
        Ok(())
    }

    /// Stop the MCP server
    pub async fn stop(&self) -> anyhow::Result<()> {
        info!("Stopping MCP Server");
        
        // TODO: Implement graceful shutdown
        // - Signal shutdown to all handlers
        // - Close connections
        // - Clean up resources
        
        Ok(())
    }

    /// Handle an MCP request
    pub async fn handle_request(&self, request: McpRequest) -> McpResult<McpResponse> {
        debug!(method = %request.method, "Handling MCP request");
        
        let result = match request.method.as_str() {
            crate::protocol::methods::INITIALIZE => {
                self.handle_initialize().await?
            }
            crate::protocol::methods::LIST_CAPABILITIES => {
                serde_json::to_value(self.capabilities.list())?
            }
            crate::protocol::methods::LIST_TOOLS => {
                serde_json::to_value(self.tools.list())?
            }
            crate::protocol::methods::CALL_TOOL => {
                let call_params: serde_json::Value = serde_json::from_value(request.params)?;
                
                // Extract tool input and auth context
                let tool_input: crate::protocol::ToolInput = serde_json::from_value(
                    call_params.get("input").cloned().unwrap_or_default()
                )?;
                
                // Extract auth context (would come from MCP request headers/auth)
                let auth_context = crate::tools::AuthContext {
                    user_id: uuid::Uuid::nil(), // TODO: Extract from request
                    organization_id: uuid::Uuid::nil(), // TODO: Extract from request
                    roles: vec![],
                    permissions: vec![],
                    email: None,
                };
                
                let result = self.tools.execute(tool_input, &auth_context, None).await?;
                
                // Render result if render_type is specified
                let rendered_result = if let Some(render_type) = result.response_type.as_ref().and_then(|rt| rt.render_type.as_ref()) {
                    let rendered = crate::render::render_result(&result, Some(render_type));
                    crate::protocol::ToolResult {
                        rendered: Some(rendered),
                        ..result
                    }
                } else {
                    result
                };
                
                serde_json::to_value(rendered_result)?
            }
            _ => {
                return Err(McpError::Protocol(
                    format!("Unknown method: {}", request.method)
                ));
            }
        };
        
        Ok(McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        })
    }

    /// Handle initialize request
    async fn handle_initialize(&self) -> McpResult<serde_json::Value> {
        Ok(serde_json::json!({
            "protocol_version": "2024-11-05",
            "server_info": {
                "name": "rustcare-mcp-server",
                "version": "0.1.0"
            },
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {},
                "sampling": {}
            }
        }))
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

