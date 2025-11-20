//! UI Components Registry Handlers
//!
//! Handlers for registering and managing UI components discovered from decorators

use crate::error::{api_success, api_success_with_meta, ApiError, ApiResponse};
use crate::middleware::AuthContext;
use crate::server::RustCareServer;
use crate::types::pagination::PaginationParams;
use crate::validation::RequestValidation;
use crate::{validate_field, validate_length, validate_required};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Register UI component request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterComponentRequest {
    pub component_name: String,
    pub component_path: String,
    pub component_type: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub route_path: Option<String>,
    pub category: String,
    pub requires_permission: Option<String>,
    pub required_roles: Option<Vec<String>>,
    pub sensitive: Option<bool>,
    pub icon: Option<String>,
    pub tags: Option<Vec<String>>,
    pub component_props: Option<serde_json::Value>,
    pub parent_component: Option<String>,
}

impl RequestValidation for RegisterComponentRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.component_name, "Component name is required");
        validate_required!(self.component_path, "Component path is required");
        validate_required!(self.component_type, "Component type is required");
        validate_required!(self.category, "Category is required");

        validate_length!(
            self.component_name,
            1,
            255,
            "Component name must be between 1 and 255 characters"
        );

        // Validate component_type
        let valid_types = [
            "page",
            "component",
            "button",
            "form",
            "modal",
            "widget",
            "action",
        ];
        validate_field!(
            self.component_type,
            valid_types.contains(&self.component_type.as_str()),
            format!("Component type must be one of: {}", valid_types.join(", "))
        );

        Ok(())
    }
}

/// Register component action (button, link, etc.) request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterActionRequest {
    pub component_path: String,
    pub action_name: String,
    pub action_type: String,
    pub display_label: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub requires_permission: Option<String>,
    pub required_roles: Option<Vec<String>>,
    pub sensitive: Option<bool>,
    pub action_config: Option<serde_json::Value>,
    pub variant: Option<String>,
    pub size: Option<String>,
    pub display_order: Option<i32>,
}

impl RequestValidation for RegisterActionRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.component_path, "Component path is required");
        validate_required!(self.action_name, "Action name is required");
        validate_required!(self.action_type, "Action type is required");
        validate_required!(self.display_label, "Display label is required");

        Ok(())
    }
}

/// List UI components query params
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListComponentsParams {
    pub component_type: Option<String>,
    pub category: Option<String>,
    pub parent_component: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Register a UI component
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::UI_COMPONENTS_REGISTER,
    request_body = RegisterComponentRequest,
    responses(
        (status = 201, description = "Component registered successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "ui-components",
    security(("bearer_auth" = []))
)]
pub async fn register_component(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(request): Json<RegisterComponentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Validate request
    request.validate()?;

    // Get or find parent component ID if specified
    let parent_component_id: Option<Uuid> = if let Some(ref parent_name) = request.parent_component
    {
        sqlx::query_scalar::<_, Option<Uuid>>(
            r#"
            SELECT id FROM ui_components
            WHERE organization_id = $1
              AND component_name = $2
              AND is_deleted = false
            LIMIT 1
            "#,
        )
        .bind(auth.organization_id)
        .bind(parent_name)
        .fetch_optional(&server.db_pool)
        .await?
        .flatten()
    } else {
        None
    };

    // Register component in database
    let component_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ui_components (
            id, organization_id, component_name, component_path, component_type,
            display_name, description, route_path, category,
            requires_permission, required_roles, sensitive, icon, tags,
            component_props, parent_component_id, auto_discovered, registered_by, is_active
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, true, $16, true
        )
        ON CONFLICT (organization_id, component_path)
        DO UPDATE SET
            component_name = EXCLUDED.component_name,
            component_type = EXCLUDED.component_type,
            display_name = EXCLUDED.display_name,
            description = EXCLUDED.description,
            route_path = EXCLUDED.route_path,
            category = EXCLUDED.category,
            requires_permission = EXCLUDED.requires_permission,
            required_roles = EXCLUDED.required_roles,
            sensitive = EXCLUDED.sensitive,
            icon = EXCLUDED.icon,
            tags = EXCLUDED.tags,
            component_props = EXCLUDED.component_props,
            parent_component_id = EXCLUDED.parent_component_id,
            updated_at = NOW()
        RETURNING id
        "#
    )
    .bind(auth.organization_id)
    .bind(&request.component_name)
    .bind(&request.component_path)
    .bind(&request.component_type)
    .bind(request.display_name.as_deref().unwrap_or(&request.component_name))
    .bind(request.description.as_deref())
    .bind(request.route_path.as_deref())
    .bind(&request.category)
    .bind(request.requires_permission.as_deref())
    .bind(request.required_roles.as_deref())
    .bind(request.sensitive.unwrap_or(false))
    .bind(request.icon.as_deref())
    .bind(request.tags.as_deref())
    .bind(request.component_props)
    .bind(parent_component_id)
    .bind(auth.user_id)
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to register component: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(api_success(serde_json::json!({
            "component_id": component_id,
            "message": "Component registered successfully"
        }))),
    ))
}

/// Register a component action (button, link, etc.)
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::UI_COMPONENTS_REGISTER_ACTION,
    request_body = RegisterActionRequest,
    responses(
        (status = 201, description = "Action registered successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "ui-components",
    security(("bearer_auth" = []))
)]
pub async fn register_component_action(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(request): Json<RegisterActionRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), ApiError> {
    // Validate request
    request.validate()?;

    // Find component by path
    let component_id: Uuid = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id FROM ui_components
        WHERE organization_id = $1
          AND component_path = $2
          AND is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(auth.organization_id)
    .bind(&request.component_path)
    .fetch_optional(&server.db_pool)
    .await?
    .ok_or_else(|| ApiError::not_found("component"))?;

    // Register action
    let action_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ui_component_actions (
            id, component_id, action_name, action_type,
            display_label, description, icon,
            requires_permission, required_roles, sensitive,
            action_config, variant, size, display_order, is_active
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, true
        )
        ON CONFLICT (component_id, action_name)
        DO UPDATE SET
            action_type = EXCLUDED.action_type,
            display_label = EXCLUDED.display_label,
            description = EXCLUDED.description,
            icon = EXCLUDED.icon,
            requires_permission = EXCLUDED.requires_permission,
            required_roles = EXCLUDED.required_roles,
            sensitive = EXCLUDED.sensitive,
            action_config = EXCLUDED.action_config,
            variant = EXCLUDED.variant,
            size = EXCLUDED.size,
            display_order = EXCLUDED.display_order,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(component_id)
    .bind(&request.action_name)
    .bind(&request.action_type)
    .bind(&request.display_label)
    .bind(request.description.as_deref())
    .bind(request.icon.as_deref())
    .bind(request.requires_permission.as_deref())
    .bind(request.required_roles.as_deref())
    .bind(request.sensitive.unwrap_or(false))
    .bind(request.action_config)
    .bind(request.variant.as_deref())
    .bind(request.size.as_deref())
    .bind(request.display_order.unwrap_or(0))
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to register action: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(api_success(serde_json::json!({
            "action_id": action_id,
            "message": "Action registered successfully"
        }))),
    ))
}

/// List UI components
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::UI_COMPONENTS,
    responses(
        (status = 200, description = "Components retrieved successfully"),
        (status = 401, description = "Unauthorized"),
    ),
    params(ListComponentsParams),
    tag = "ui-components",
    security(("bearer_auth" = []))
)]
pub async fn list_components(
    State(server): State<RustCareServer>,
    Query(params): Query<ListComponentsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    // Note: Using sqlx::query! macro for compile-time query checking
    // PaginatedQuery doesn't work well with query! macro, so using raw SQL with pagination
    let components = sqlx::query!(
        r#"
        SELECT 
            id, component_name, component_path, component_type,
            display_name, description, route_path, category,
            requires_permission, icon, tags, parent_component_id
        FROM ui_components
        WHERE organization_id = $1
          AND is_active = true
          AND is_deleted = false
          AND ($2::text IS NULL OR component_type = $2)
          AND ($3::text IS NULL OR category = $3)
          AND ($4::uuid IS NULL OR parent_component_id = $4)
        ORDER BY category, component_name
        LIMIT $5 OFFSET $6
        "#
    )
    .bind(auth.organization_id)
    .bind(params.component_type.as_deref())
    .bind(params.category.as_deref())
    .bind(params.parent_component.and_then(|_| None::<uuid::Uuid>))
    .bind(params.pagination.limit() as i64)
    .bind(params.pagination.offset() as i64)
    .fetch_all(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to list components: {}", e)))?;

    let result: Vec<serde_json::Value> = components
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "component_name": row.component_name,
                "component_path": row.component_path,
                "component_type": row.component_type,
                "display_name": row.display_name,
                "description": row.description,
                "route_path": row.route_path,
                "category": row.category,
                "requires_permission": row.requires_permission,
                "icon": row.icon,
                "tags": row.tags,
            })
        })
        .collect();

    // Get total count for pagination metadata
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM ui_components
        WHERE organization_id = $1
          AND is_active = true
          AND is_deleted = false
          AND ($2::text IS NULL OR component_type = $2)
          AND ($3::text IS NULL OR category = $3)
          AND ($4::uuid IS NULL OR parent_component_id = $4)
        "#,
    )
    .bind(auth.organization_id)
    .bind(params.component_type.as_deref())
    .bind(params.category.as_deref())
    .bind(params.parent_component.and_then(|_| None::<uuid::Uuid>))
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to count components: {}", e)))?;

    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(api_success_with_meta(result, metadata)))
}
