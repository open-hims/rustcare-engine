//! MCP Tools Registry - Auto-registration to database
//!
//! Automatically stores discovered MCP tools in the database when they're registered.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use serde_json::Value;
use crate::protocol::RenderType;
use crate::error::{McpResult, McpError};

/// MCP Tool registry entry for database storage
#[derive(Debug, Clone)]
pub struct McpToolRegistry {
    pub tool_name: String,
    pub handler_function: String,
    pub handler_file: String,
    pub description: String,
    pub category: String,
    pub response_type: Option<String>,
    pub render_type: Option<RenderType>,
    pub requires_permission: Option<String>,
    pub sensitive: bool,
    pub input_schema: Option<Value>,
    pub output_schema: Option<Value>,
}

/// Registry service for storing MCP tools in database
pub struct McpToolRegistryService {
    db_pool: PgPool,
}

impl McpToolRegistryService {
    /// Create a new registry service
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Register a tool in the database
    pub async fn register_tool(
        &self,
        tool: &McpToolRegistry,
        organization_id: Uuid,
        registered_by: Option<Uuid>,
    ) -> McpResult<Uuid> {
        let tool_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO mcp_tools (
                id, organization_id, tool_name, handler_function, handler_file,
                description, category, response_type, render_type,
                requires_permission, sensitive, input_schema, output_schema,
                auto_discovered, registered_by, is_active
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, true, $14, true
            )
            ON CONFLICT (organization_id, tool_name) 
            DO UPDATE SET
                handler_function = EXCLUDED.handler_function,
                handler_file = EXCLUDED.handler_file,
                description = EXCLUDED.description,
                category = EXCLUDED.category,
                response_type = EXCLUDED.response_type,
                render_type = EXCLUDED.render_type,
                requires_permission = EXCLUDED.requires_permission,
                sensitive = EXCLUDED.sensitive,
                input_schema = EXCLUDED.input_schema,
                output_schema = EXCLUDED.output_schema,
                updated_at = NOW()
            RETURNING id
            "#,
            tool_id,
            organization_id,
            tool.tool_name,
            tool.handler_function,
            tool.handler_file,
            tool.description,
            tool.category,
            tool.response_type,
            tool.render_type.as_ref().map(|rt| format!("{:?}", rt).to_lowercase()),
            tool.requires_permission,
            tool.sensitive,
            tool.input_schema,
            tool.output_schema,
            registered_by,
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| McpError::Registry(format!("Failed to register tool: {}", e)))?;

        Ok(tool_id)
    }

    /// Get all tools for an organization
    pub async fn list_tools(
        &self,
        organization_id: Uuid,
        include_sensitive: bool,
    ) -> McpResult<Vec<McpToolRegistry>> {
        let tools = sqlx::query!(
            r#"
            SELECT 
                tool_name, handler_function, handler_file, description, category,
                response_type, render_type, requires_permission, sensitive,
                input_schema, output_schema
            FROM mcp_tools
            WHERE organization_id = $1
              AND is_active = true
              AND is_deleted = false
              AND ($2 = true OR sensitive = false)
            ORDER BY category, tool_name
            "#,
            organization_id,
            include_sensitive,
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| McpError::Registry(format!("Failed to list tools: {}", e)))?;

        Ok(tools.into_iter().map(|row| McpToolRegistry {
            tool_name: row.tool_name,
            handler_function: row.handler_function,
            handler_file: row.handler_file,
            description: row.description.unwrap_or_default(),
            category: row.category,
            response_type: row.response_type,
            render_type: row.render_type.and_then(|rt| parse_render_type(&rt)),
            requires_permission: row.requires_permission,
            sensitive: row.sensitive,
            input_schema: row.input_schema,
            output_schema: row.output_schema,
        }).collect())
    }

    /// Deregister a tool (soft delete)
    pub async fn deregister_tool(
        &self,
        organization_id: Uuid,
        tool_name: &str,
    ) -> McpResult<()> {
        sqlx::query!(
            r#"
            UPDATE mcp_tools
            SET is_deleted = true, deleted_at = NOW(), updated_at = NOW()
            WHERE organization_id = $1 AND tool_name = $2
            "#,
            organization_id,
            tool_name,
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| McpError::Registry(format!("Failed to deregister tool: {}", e)))?;

        Ok(())
    }
}

fn parse_render_type(s: &str) -> Option<RenderType> {
    match s.to_lowercase().as_str() {
        "json" => Some(RenderType::Json),
        "markdown" => Some(RenderType::Markdown),
        "html" => Some(RenderType::Html),
        "table" => Some(RenderType::Table),
        "list" => Some(RenderType::List),
        "text" => Some(RenderType::Text),
        _ => None,
    }
}

