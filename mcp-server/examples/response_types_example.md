# MCP Tool Response Types and Render Types

## Response Type Configuration

MCP tools can specify their response type and how it should be rendered for LLMs.

### Basic Usage with Response Type

```rust
#[mcp_macros::mcp_tool(
    name = "get_patient",
    description = "Retrieve patient information",
    category = "healthcare",
    requires_permission = "patient:read",
    sensitive = false,
    response_type = "Patient",      // Type name for the response
    render_type = "json"            // How to render: json, markdown, table, list, html, text
)]
pub async fn get_patient(...) -> Result<Json<ApiResponse<Patient>>, ApiError> {
    // Returns Patient type, rendered as JSON
}
```

### Render Types

#### 1. JSON (Default)
```rust
render_type = "json"
```
- Returns structured JSON data
- Best for: API responses, structured data
- Example: `{"id": "...", "name": "..."}`

#### 2. Markdown
```rust
render_type = "markdown"
```
- Returns markdown-formatted text
- Best for: Documentation, formatted text
- Example: `## Patient\n\n**Name:** John Doe\n**ID:** 123`

#### 3. Table
```rust
render_type = "table"
```
- Returns markdown table format
- Best for: Lists of entities, tabular data
- Example:
  ```markdown
  | Name | ID | Status |
  |------|----|----|
  | John | 1  | Active |
  ```

#### 4. List
```rust
render_type = "list"
```
- Returns simple list format
- Best for: Simple enumerations
- Example:
  ```
  1. Pharmacy A
  2. Pharmacy B
  3. Pharmacy C
  ```

#### 5. HTML
```rust
render_type = "html"
```
- Returns HTML formatted response
- Best for: Rich formatting, UI components
- Example: `<table><tr><td>...</td></tr></table>`

#### 6. Text
```rust
render_type = "text"
```
- Returns plain text
- Best for: Simple messages, notifications
- Example: `Patient retrieved successfully`

#### 7. Structured (CSV, TSV, etc.)
```rust
render_type = "structured"
```
- Returns structured format (CSV, TSV, etc.)
- Best for: Data export, spreadsheet compatibility

### Examples

#### List of Pharmacies (Table Format)
```rust
#[mcp_macros::mcp_tool(
    name = "list_pharmacies",
    description = "List all pharmacies",
    category = "pharmacy",
    response_type = "Vec<Pharmacy>",
    render_type = "table"  // Show as table for easy reading
)]
pub async fn list_pharmacies(...) -> Result<Json<ApiResponse<Vec<Pharmacy>>>, ApiError> {
    // Returns table: | Name | Address | City |
}
```

#### Patient Details (Markdown)
```rust
#[mcp_macros::mcp_tool(
    name = "get_patient_details",
    description = "Get detailed patient information",
    category = "healthcare",
    response_type = "Patient",
    render_type = "markdown"  // Formatted markdown for readability
)]
pub async fn get_patient_details(...) -> Result<Json<ApiResponse<Patient>>, ApiError> {
    // Returns formatted markdown
}
```

#### Simple Notification (Text)
```rust
#[mcp_macros::mcp_tool(
    name = "send_notification",
    description = "Send a notification",
    category = "notifications",
    response_type = "NotificationStatus",
    render_type = "text"  // Simple text response
)]
pub async fn send_notification(...) -> Result<Json<ApiResponse<NotificationStatus>>, ApiError> {
    // Returns: "Notification sent successfully"
}
```

## Response Type Information in Tool Schema

The MCP tool schema includes:

```json
{
  "name": "get_patient",
  "description": "Retrieve patient information",
  "input_schema": {
    "type": "object",
    "properties": {
      "patient_id": {"type": "string"}
    }
  },
  "output_schema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"},
      "name": {"type": "string"}
    }
  },
  "render_type": "json"
}
```

## Benefits

1. **LLM Understanding**: LLMs know how to present the data
2. **Consistent Formatting**: Same data type always rendered the same way
3. **User Experience**: Appropriate format for the data type
4. **Type Safety**: Response types are declared and validated

