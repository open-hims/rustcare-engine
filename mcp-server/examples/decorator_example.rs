//! Example showing how to use the MCP tool pattern
//!
//! This demonstrates how handler functions can be exposed as MCP tools.
//! 
//! NOTE: The #[mcp_tool] macro is conceptual and would be implemented
//! via the mcp-macros crate. This example shows the intended usage pattern.

fn main() {
    println!("MCP Tool Decorator Pattern Example");
    println!("===================================\n");
    
    println!("This example demonstrates how MCP tools would be registered:");
    println!("  1. Use #[mcp_tool] attribute on handler functions");
    println!("  2. Specify metadata (name, description, category, permissions)");
    println!("  3. Mark sensitive tools that should be excluded from LLM access");
    println!("  4. Tools are auto-discovered and registered\n");
    
    println!("Example usage:");
    println!(r#"
    #[mcp_tool(
        name = "list_pharmacies",
        description = "List all pharmacies",
        category = "pharmacy",
        requires_permission = "pharmacy:read",
        sensitive = false
    )]
    pub async fn list_pharmacies(
        params: ToolInput,
        auth: &AuthContext,
    ) -> McpResult<ToolResult> {{
        // Implementation
    }}
    "#);
    
    println!("\nFor a complete implementation, see:");
    println!("  - mcp-server/src/tools.rs");
    println!("  - mcp-server/src/tool_wrapper.rs");
    println!("  - mcp-macros/ (for procedural macros)");
}
