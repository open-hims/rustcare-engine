# MCP Tool Decorator Usage Examples

## Basic Usage

```rust
use rustcare_server::handlers::pharmacy;
use rustcare_server::server::RustCareServer;
use rustcare_server::middleware::AuthContext;
use rustcare_server::error::{ApiResponse, ApiError};

// Mark handler function with #[mcp_tool] decorator
#[mcp_macros::mcp_tool(
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
    // Your existing handler code
    // This function is now automatically exposed as an MCP tool!
}
```

## Sensitive Endpoint (Excluded from Public Access)

```rust
#[mcp_macros::mcp_tool(
    name = "rotate_secret",
    description = "Rotate a secret key",
    category = "secrets",
    requires_permission = "secrets:rotate",
    sensitive = true  // This excludes it from public tool discovery
)]
pub async fn rotate_secret_handler(...) -> Result<...> {
    // Implementation
}
```

## With Zanzibar Permission Check

```rust
#[mcp_macros::mcp_tool(
    name = "create_medical_record",
    description = "Create a new medical record",
    category = "healthcare",
    requires_permission = "medical_record:write",  // Zanzibar checks this automatically
    sensitive = false
)]
pub async fn create_medical_record(...) -> Result<...> {
    // Permission is automatically checked before execution
    // If user doesn't have "medical_record:write" permission, execution is blocked
}
```

## Multiple Decorators

You can use both `#[utoipa::path]` and `#[mcp_tool]` together:

```rust
#[mcp_macros::mcp_tool(
    name = "get_patient",
    description = "Retrieve patient information",
    category = "healthcare",
    requires_permission = "patient:read",
    sensitive = false
)]
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/patients/{patient_id}",
    responses(
        (status = 200, description = "Patient retrieved", body = Patient)
    ),
    tag = "healthcare",
    security(("bearer_auth" = []))
)]
pub async fn get_patient(...) -> Result<...> {
    // Function serves both HTTP API and MCP tool
}
```

## How It Works

1. **Build Time**: `build.rs` scans all handler files for `#[mcp_tool]` attributes
2. **Code Generation**: Tool metadata is extracted and registration code is generated
3. **Runtime**: Tools are automatically registered in the MCP server
4. **Auth**: AuthContext is automatically extracted from MCP requests
5. **Permissions**: Zanzibar permission checks happen before tool execution
6. **Filtering**: Sensitive tools are excluded from public tool lists

## Benefits

- ✅ **Declarative**: Just add an attribute macro, no manual registration
- ✅ **Type Safe**: Tool schemas generated from function signatures
- ✅ **Automatic**: Tools discovered at build time
- ✅ **Secure**: Auth and permissions built-in
- ✅ **Consistent**: Same pattern as utoipa for API docs

