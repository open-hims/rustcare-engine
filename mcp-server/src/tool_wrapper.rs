//! Wrapper to convert handler functions into MCP tools
//!
//! This module provides utilities to wrap RustCare handler functions
//! and expose them as MCP tools with automatic auth/Zanzibar integration.

use crate::tools::{McpTool, ToolInput, ToolResult, AuthContext, ZanzibarClient};
use crate::error::{McpResult, McpError};
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// Wrapper that converts a handler function into an MCP tool
pub struct HandlerToolWrapper {
    name: String,
    description: String,
    category: String,
    required_permission: Option<String>,
    sensitive: bool,
    input_schema: Value,
    output_schema: Option<Value>,
    render_type: Option<crate::protocol::RenderType>,
    response_type_name: Option<String>,
    handler_fn: Box<dyn Fn(Value, &AuthContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = McpResult<ToolResult>> + Send>> + Send + Sync>,
}

impl HandlerToolWrapper {
    /// Create a new wrapper from handler metadata
    pub fn new(
        name: String,
        description: String,
        category: String,
        required_permission: Option<String>,
        sensitive: bool,
        input_schema: Value,
        output_schema: Option<Value>,
        render_type: Option<crate::protocol::RenderType>,
        response_type_name: Option<String>,
        handler_fn: impl Fn(Value, &AuthContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = McpResult<ToolResult>> + Send>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name,
            description,
            category,
            required_permission,
            sensitive,
            input_schema,
            output_schema,
            render_type,
            response_type_name,
            handler_fn: Box::new(handler_fn),
        }
    }
}

#[async_trait]
impl McpTool for HandlerToolWrapper {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    fn category(&self) -> &str {
        &self.category
    }
    
    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }
    
    fn output_schema(&self) -> Option<Value> {
        self.output_schema.clone()
    }
    
    fn render_type(&self) -> Option<crate::protocol::RenderType> {
        self.render_type.clone()
    }
    
    fn response_type_name(&self) -> Option<&str> {
        self.response_type_name.as_deref()
    }
    
    fn required_permission(&self) -> Option<&str> {
        self.required_permission.as_deref()
    }
    
    fn is_sensitive(&self) -> bool {
        self.sensitive
    }
    
    async fn execute(
        &self,
        input: ToolInput,
        auth_context: &AuthContext,
        _zanzibar_client: Option<&dyn ZanzibarClient>,
    ) -> McpResult<ToolResult> {
        // Call the wrapped handler function
        (self.handler_fn)(input.arguments, auth_context).await
    }
}

/// Helper macro to create tool wrappers from handler functions
#[macro_export]
macro_rules! wrap_handler_as_tool {
    (
        name = $name:expr,
        description = $desc:expr,
        category = $cat:expr,
        permission = $perm:expr,
        sensitive = $sens:expr,
        response_type = $resp_type:expr,
        render_type = $render_type:expr,
        handler = $handler:path
    ) => {
        Box::new($crate::tool_wrapper::HandlerToolWrapper::new(
            $name.to_string(),
            $desc.to_string(),
            $cat.to_string(),
            $perm,
            $sens,
            // Input schema would be auto-generated from handler signature
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            // Output schema would be auto-generated from return type
            None,
            $render_type,
            $resp_type,
            |args: serde_json::Value, auth: &$crate::tools::AuthContext| {
                Box::pin(async move {
                    // Convert args to handler parameters and call handler
                    // This is a simplified version - actual implementation would
                    // parse args based on handler signature
                    $crate::error::McpResult::Ok($crate::protocol::ToolResult {
                        status: $crate::protocol::ToolStatus::Success,
                        data: Some(serde_json::json!({
                            "message": "Handler execution not yet implemented"
                        })),
                        error: None,
                        response_type: $resp_type.map(|rt| $crate::protocol::ResponseType {
                            type_name: rt.to_string(),
                            render_type: $render_type.clone(),
                            schema: None,
                        }),
                        rendered: None,
                    })
                })
            },
        ))
    };
}

