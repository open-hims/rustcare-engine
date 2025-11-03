# RustCare MCP Server

Model Context Protocol (MCP) server for RustCare Engine, enabling AI agents and LLMs to interact with healthcare functions through a standardized protocol.

## Features

- **Decorator Pattern**: Use `#[mcp_tool]` attribute macros (similar to utoipa) to automatically expose handler functions as MCP tools
- **Automatic Auth Integration**: AuthContext is automatically extracted and passed to tools
- **Zanzibar Permission Checks**: Built-in integration with Zanzibar for fine-grained access control
- **Sensitive Endpoint Filtering**: Automatically excludes sensitive endpoints (secrets, KMS, auth) from public LLM access
- **Auto-Discovery**: Tools are automatically discovered at build time by scanning for `#[mcp_tool]` attributes

## Usage

### 1. Mark Handler Functions with `#[mcp_tool]`

```rust
use mcp_server::tools::{AuthContext, ToolResult};
use mcp_server::error::McpResult;

#[mcp_server::mcp_tool(
    name = "get_patient",
    description = "Retrieve patient information by ID",
    category = "healthcare",
    requires_permission = "patient:read",
    sensitive = false
)]
pub async fn get_patient_handler(
    patient_id: Uuid,
    auth: &AuthContext,
) -> McpResult<ToolResult> {
    // Handler implementation
    // Auth context is automatically provided
    Ok(ToolResult {
        status: ToolStatus::Success,
        data: Some(serde_json::json!({
            "patient_id": patient_id,
            "organization_id": auth.organization_id
        })),
        error: None,
    })
}
```

### 2. Sensitive Endpoints

Mark sensitive endpoints with `sensitive = true` to exclude them from public LLM access:

```rust
#[mcp_server::mcp_tool(
    name = "rotate_secret",
    description = "Rotate a secret key",
    category = "secrets",
    requires_permission = "secrets:rotate",
    sensitive = true  // Excluded from public tool lists
)]
pub async fn rotate_secret_handler(...) -> McpResult<ToolResult> {
    // Implementation
}
```

### 3. Permission Checks

Tools automatically check Zanzibar permissions before execution:

```rust
#[mcp_server::mcp_tool(
    name = "create_medical_record",
    description = "Create a new medical record",
    category = "healthcare",
    requires_permission = "medical_record:write",  // Zanzibar permission checked
    sensitive = false
)]
pub async fn create_medical_record_handler(...) -> McpResult<ToolResult> {
    // Permission is automatically checked before this executes
}
```

## Architecture

```
┌─────────────────────────────────────────┐
│         LLM / AI Agent                   │
│     (Claude, GPT, etc.)                  │
└──────────────┬──────────────────────────┘
               │ MCP Protocol (JSON-RPC)
               │
┌──────────────▼──────────────────────────┐
│         MCP Server                       │
│  - Tool Registry                         │
│  - Auth Context Extraction               │
│  - Zanzibar Permission Checks            │
│  - Sensitive Endpoint Filtering           │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│      RustCare Handlers                   │
│  (marked with #[mcp_tool])               │
└──────────────────────────────────────────┘
```

## Auto-Discovery

The build script (`build.rs`) automatically scans handler files for `#[mcp_tool]` attributes and generates tool registration code. This means:

- No manual tool registration needed
- Tools are discovered at compile time
- Type-safe tool definitions
- Automatic schema generation from function signatures

## Sensitive Endpoint Filtering

The following are automatically excluded from public tool lists:

- Authentication endpoints (`/auth/*`)
- Secrets management (`/secrets/*`)
- Key management (`/kms/*`)
- Encryption/decryption operations
- Deletion operations (can be configured)
- Admin operations

Tools can still be accessed programmatically if the LLM has proper permissions, but they won't appear in public tool discovery.

## Integration with Zanzibar

MCP tools automatically integrate with Zanzibar for permission checks:

1. Tool declares required permission via `requires_permission`
2. Before execution, MCP server checks Zanzibar
3. If permission check fails, tool execution is blocked
4. Auth context (user_id, organization_id) is automatically provided

## Example: Complete Handler

```rust
use axum::{extract::State, Json};
use rustcare_server::server::RustCareServer;
use rustcare_server::middleware::AuthContext;
use rustcare_server::error::{ApiResponse, api_success};

#[mcp_server::mcp_tool(
    name = "list_pharmacies",
    description = "List all pharmacies for the organization",
    category = "pharmacy",
    requires_permission = "pharmacy:read",
    sensitive = false
)]
#[utoipa::path(
    get,
    path = "/api/v1/pharmacy/pharmacies",
    responses(
        (status = 200, description = "Pharmacies retrieved", body = Vec<Pharmacy>)
    ),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn list_pharmacies(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Query(params): Query<ListPharmaciesParams>,
) -> Result<Json<ApiResponse<Vec<Pharmacy>>>, ApiError> {
    // Existing handler implementation
    // This function is now also available as an MCP tool!
}
```

## Building

```bash
cd mcp-server
cargo build
```

The build script will automatically discover all `#[mcp_tool]` decorated functions and generate tool registration code.

## Running

```bash
cargo run --bin mcp-server
```

The MCP server will start and expose tools via JSON-RPC over stdio or HTTP (configurable).

## Connecting an LLM

Most modern LLMs support MCP. Example configuration:

```json
{
  "mcpServers": {
    "rustcare": {
      "command": "cargo",
      "args": ["run", "--bin", "mcp-server", "--manifest-path", "mcp-server/Cargo.toml"]
    }
  }
}
```

## Security Considerations

- All tools require authentication (AuthContext)
- Sensitive tools are excluded from public discovery
- Zanzibar permissions are checked before execution
- Audit logging is automatic for all tool executions
- Organization-scoped: tools only access data for the authenticated user's organization

