//! Complete example showing MCP tool with response types and render types
//!
//! This demonstrates the full feature set of MCP tools including:
//! - Response type declarations
//! - Render type specifications (table, csv, json)
//! - Sensitive tool filtering
//! - Permission requirements

fn main() {
    println!("Complete MCP Tool Example");
    println!("=========================\n");
    
    println!("This example shows advanced MCP tool features:");
    println!("  1. Response Types - Specify structured output types");
    println!("  2. Render Types - Control how data is displayed (table/csv/json)");
    println!("  3. Sensitive Filtering - Exclude tools from public LLM access");
    println!("  4. Permission Checks - Integrate with Zanzibar authorization\n");
    
    println!("Example: List pharmacies with table rendering:");
    println!(r#"
    #[mcp_tool(
        name = "list_pharmacies",
        description = "List all pharmacies",
        category = "pharmacy",
        requires_permission = "pharmacy:read",
        sensitive = false,
        response_type = "Vec<Pharmacy>",
        render_type = "table"  // Renders as markdown table
    )]
    pub async fn list_pharmacies(
        State(server): State<RustCareServer>,
        auth: AuthContext,
        Query(params): Query<ListPharmaciesParams>,
    ) -> Result<Json<ApiResponse<Vec<Pharmacy>>>, ApiError> {{
        // Implementation returns structured data
        // MCP server automatically renders it as a table
    }}
    "#);
    
    println!("\nRender types available:");
    println!("  - table: Markdown table format (good for structured data)");
    println!("  - csv: Comma-separated values");
    println!("  - tsv: Tab-separated values");
    println!("  - json: Raw JSON (default)");
    
    println!("\nSensitive tools example:");
    println!(r#"
    #[mcp_tool(
        name = "rotate_secret",
        description = "Rotate encryption keys",
        category = "secrets",
        requires_permission = "secrets:rotate",
        sensitive = true  // Excluded from public tool lists
    )]
    "#);
    
    println!("\nFor implementation details, see:");
    println!("  - mcp-server/src/render.rs (rendering logic)");
    println!("  - mcp-server/src/protocol.rs (MCP protocol types)");
    println!("  - rustcare-server/src/handlers/ (actual tool implementations)");
}
