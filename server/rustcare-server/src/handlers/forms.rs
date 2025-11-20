//! Dynamic Form Builder Handlers
//!
//! Handlers for managing dynamic form definitions and submissions
//! Supports forms for any module (healthcare, pharmacy, billing, etc.)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    error::{api_success, ApiError, ApiResponse},
    middleware::AuthContext,
    server::RustCareServer,
    types::pagination::PaginationParams,
    utils::query_builder::PaginatedQuery,
    validation::{RequestValidation},
    validate_field, validate_length, validate_required, validate_email,
};
use database_layer::{QueryExecutor, RlsContext};
use uuid::Uuid as UuidType;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Form definition response
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow)]
pub struct FormDefinition {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub form_name: String,
    pub form_slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub module_name: String,
    pub entity_type: Option<String>,
    pub form_schema: serde_json::Value,
    pub form_layout: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
    pub submission_handler: Option<String>,
    pub is_active: bool,
    pub is_template: bool,
    pub allow_multiple_submissions: bool,
    pub require_approval: bool,
    pub requires_permission: Option<String>,
    pub required_roles: Option<Vec<String>>,
    pub allowed_roles: Option<Vec<String>>,
    pub version: i32,
    pub parent_form_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

/// Form field definition (part of form_schema)
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct FormField {
    pub id: String,
    pub name: String,
    pub label: String,
    pub field_type: String, // 'text', 'email', 'number', 'select', 'checkbox', 'date', 'textarea', 'file', etc.
    pub required: bool,
    pub placeholder: Option<String>,
    pub help_text: Option<String>,
    pub default_value: Option<serde_json::Value>,
    pub options: Option<Vec<FormFieldOption>>,
    pub validation: Option<FieldValidation>,
    pub conditional_logic: Option<ConditionalLogic>,
    pub ui_config: Option<FieldUIConfig>,
    pub metadata: Option<serde_json::Value>,
}

/// Form field option (for select, radio, etc.)
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct FormFieldOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub disabled: Option<bool>,
}

/// Field validation rules
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct FieldValidation {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub pattern: Option<String>,          // Regex pattern
    pub custom_validator: Option<String>, // Custom validation function name
    pub error_message: Option<String>,
}

/// Conditional logic for fields
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ConditionalLogic {
    pub show_if: Option<ConditionalRule>,
    pub hide_if: Option<ConditionalRule>,
    pub required_if: Option<ConditionalRule>,
}

/// Conditional rule
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ConditionalRule {
    pub field: String,
    pub operator: String, // 'equals', 'not_equals', 'contains', 'greater_than', etc.
    pub value: serde_json::Value,
}

/// Field UI configuration
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct FieldUIConfig {
    pub width: Option<String>, // 'full', 'half', 'third', etc.
    pub column_span: Option<usize>,
    pub row_span: Option<usize>,
    pub css_class: Option<String>,
    pub icon: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

/// Create form definition request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFormDefinitionRequest {
    pub form_name: String,
    pub form_slug: String,
    pub display_name: String,
    pub description: Option<String>,
    pub module_name: String,
    pub entity_type: Option<String>,
    pub form_schema: serde_json::Value, // Array of FormField
    pub form_layout: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
    pub submission_handler: Option<String>,
    pub is_template: Option<bool>,
    pub allow_multiple_submissions: Option<bool>,
    pub require_approval: Option<bool>,
    pub requires_permission: Option<String>,
    pub required_roles: Option<Vec<String>>,
    pub allowed_roles: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub icon: Option<String>,
}

impl RequestValidation for CreateFormDefinitionRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.form_name, "Form name is required");
        validate_required!(self.form_slug, "Form slug is required");
        validate_required!(self.display_name, "Display name is required");
        validate_required!(self.module_name, "Module name is required");
        
        // Validate JSON schema manually since validate_required! doesn't work with JsonValue
        if self.form_schema.is_null() || (self.form_schema.is_string() && self.form_schema.as_str().unwrap_or("").trim().is_empty()) {
            return Err(ApiError::bad_request("Form schema is required"));
        }

        validate_length!(
            self.form_name,
            1,
            255,
            "Form name must be between 1 and 255 characters"
        );
        validate_length!(
            self.form_slug,
            1,
            100,
            "Form slug must be between 1 and 100 characters"
        );

        // Validate form_schema is an array
        if !self.form_schema.is_array() {
            return Err(ApiError::validation(
                "Form schema must be an array of fields",
            ));
        }

        // Validate slug format
        if !self
            .form_slug
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-')
        {
            return Err(ApiError::validation(
                "Form slug must contain only alphanumeric characters and hyphens",
            ));
        }

        Ok(())
    }
}

/// Update form definition request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFormDefinitionRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub form_schema: Option<serde_json::Value>,
    pub form_layout: Option<serde_json::Value>,
    pub validation_rules: Option<serde_json::Value>,
    pub submission_handler: Option<String>,
    pub is_active: Option<bool>,
    pub require_approval: Option<bool>,
    pub requires_permission: Option<String>,
    pub required_roles: Option<Vec<String>>,
    pub allowed_roles: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub icon: Option<String>,
}

/// Form submission request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitFormRequest {
    pub form_definition_id: Uuid,
    pub submission_data: serde_json::Value,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub notes: Option<String>,
}

/// Form submission response
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow)]
pub struct FormSubmission {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub form_definition_id: Uuid,
    pub submission_data: serde_json::Value,
    pub submission_status: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub submitted_by: Option<Uuid>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub form_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// List form definitions query params
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListFormsParams {
    pub module_name: Option<String>,
    pub entity_type: Option<String>,
    pub is_template: Option<bool>,
    pub is_active: Option<bool>,
    pub category: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// List form definitions
#[utoipa::path(
    get,
    path = "/api/v1/forms",
    responses(
        (status = 200, description = "List of form definitions", body = Vec<FormDefinition>)
    ),
    params(
        ("module_name" = Option<String>, Query, description = "Filter by module"),
        ("entity_type" = Option<String>, Query, description = "Filter by entity type"),
        ("is_template" = Option<bool>, Query, description = "Filter templates"),
        ("is_active" = Option<bool>, Query, description = "Filter active forms")
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_form_definitions(
    State(server): State<RustCareServer>,
    Query(params): Query<ListFormsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<FormDefinition>>>, ApiError> {
    let mut query_builder = PaginatedQuery::new_with_base_filter(
        "SELECT 
            id, organization_id, form_name, form_slug, display_name, description,
            module_name, entity_type, form_schema, form_layout, validation_rules,
            submission_handler, is_active, is_template, allow_multiple_submissions,
            require_approval, requires_permission, required_roles, allowed_roles,
            version, parent_form_id, tags, category, icon,
            created_at, updated_at, created_by
         FROM form_definitions
         WHERE organization_id = ",
        "organization_id",
        auth.organization_id,
    );

    query_builder.filter_not_deleted();

    // Apply filters
    query_builder.filter_eq("module_name", params.module_name.as_deref().map(str::to_owned));
    query_builder.filter_eq("entity_type", params.entity_type.as_deref().map(str::to_owned));
    query_builder.filter_eq("is_template", params.is_template);
    query_builder.filter_eq("is_active", params.is_active);
    query_builder.filter_eq("category", params.category.as_deref().map(str::to_owned));

    query_builder.order_by("created_at", "DESC");

    let forms = query_builder
        .build_query_as::<FormDefinition>()
        .fetch_all(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch forms: {}", e)))?;

    Ok(Json(api_success(forms)))
}

/// Get form definition by ID
#[utoipa::path(
    get,
    path = "/api/v1/forms/{form_id}",
    responses(
        (status = 200, description = "Form definition", body = FormDefinition),
        (status = 404, description = "Form not found")
    ),
    params(
        ("form_id" = Uuid, Path, description = "Form definition ID")
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_form_definition(
    Path(form_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<FormDefinition>>, ApiError> {
    // Create RLS context
    let rls_context = RlsContext::new()
        .with_user_id(auth.user_id)
        .with_tenant_id(auth.organization_id.to_string())
        .with_organization_id(auth.organization_id);

    let executor = server.query_executor_with_rls(rls_context);

    let form = executor
        .fetch_optional_with(
            "SELECT 
                id, organization_id, form_name, form_slug, display_name, description,
                module_name, entity_type, form_schema, form_layout, validation_rules,
                submission_handler, is_active, is_template, allow_multiple_submissions,
                require_approval, requires_permission, required_roles, allowed_roles,
                version, parent_form_id, tags, category, icon,
                created_at, updated_at, created_by
             FROM form_definitions
             WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL",
            |q| q.bind(form_id).bind(auth.organization_id),
        )
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch form: {}", e)))?;

    match form {
        Some(f) => Ok(Json(api_success(f))),
        None => Err(ApiError::not_found("Form definition not found")),
    }
}

/// Get form definition by slug
#[utoipa::path(
    get,
    path = "/api/v1/forms/slug/{form_slug}",
    responses(
        (status = 200, description = "Form definition", body = FormDefinition),
        (status = 404, description = "Form not found")
    ),
    params(
        ("form_slug" = String, Path, description = "Form slug")
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_form_definition_by_slug(
    Path(form_slug): Path<String>,
    State(server): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<FormDefinition>>, ApiError> {
    let form = sqlx::query_as::<_, FormDefinition>(
        "SELECT 
            id, organization_id, form_name, form_slug, display_name, description,
            module_name, entity_type, form_schema, form_layout, validation_rules,
            submission_handler, is_active, is_template, allow_multiple_submissions,
            require_approval, requires_permission, required_roles, allowed_roles,
            version, parent_form_id, tags, category, icon,
            created_at, updated_at, created_by
         FROM form_definitions
         WHERE form_slug = $1 AND organization_id = $2 AND deleted_at IS NULL AND is_active = true
         ORDER BY version DESC
         LIMIT 1",
    )
    .bind(&form_slug)
    .bind(auth.organization_id)
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch form: {}", e)))?;

    match form {
        Some(f) => Ok(Json(api_success(f))),
        None => Err(ApiError::not_found("Form definition not found")),
    }
}

/// Create form definition
#[utoipa::path(
    post,
    path = "/api/v1/forms",
    request_body = CreateFormDefinitionRequest,
    responses(
        (status = 201, description = "Form definition created", body = FormDefinition)
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_form_definition(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(req): Json<CreateFormDefinitionRequest>,
) -> Result<Json<ApiResponse<FormDefinition>>, ApiError> {
    req.validate()?;

    // Check if slug already exists
    let existing = sqlx::query!(
        "SELECT id FROM form_definitions 
         WHERE form_slug = $1 AND organization_id = $2 AND deleted_at IS NULL",
        req.form_slug,
        auth.organization_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing form: {}", e)))?;

    if existing.is_some() {
        return Err(ApiError::conflict("A form with this slug already exists"));
    }

    // Create form definition
    let form_id = Uuid::new_v4();
    let form = sqlx::query_as::<_, FormDefinition>(
        "INSERT INTO form_definitions (
            id, organization_id, form_name, form_slug, display_name, description,
            module_name, entity_type, form_schema, form_layout, validation_rules,
            submission_handler, is_template, allow_multiple_submissions,
            require_approval, requires_permission, required_roles, allowed_roles,
            tags, category, icon, created_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
        )
        RETURNING 
            id, organization_id, form_name, form_slug, display_name, description,
            module_name, entity_type, form_schema, form_layout, validation_rules,
            submission_handler, is_active, is_template, allow_multiple_submissions,
            require_approval, requires_permission, required_roles, allowed_roles,
            version, parent_form_id, tags, category, icon,
            created_at, updated_at, created_by"
    )
    .bind(form_id)
    .bind(auth.organization_id)
    .bind(&req.form_name)
    .bind(&req.form_slug)
    .bind(&req.display_name)
    .bind(&req.description)
    .bind(&req.module_name)
    .bind(&req.entity_type)
    .bind(&req.form_schema)
    .bind(&req.form_layout)
    .bind(&req.validation_rules)
    .bind(&req.submission_handler)
    .bind(req.is_template.unwrap_or(false))
    .bind(req.allow_multiple_submissions.unwrap_or(true))
    .bind(req.require_approval.unwrap_or(false))
    .bind(&req.requires_permission)
    .bind(&req.required_roles)
    .bind(&req.allowed_roles)
    .bind(&req.tags)
    .bind(&req.category)
    .bind(&req.icon)
    .bind(auth.user_id)
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create form: {}", e)))?;

    tracing::info!(
        form_id = %form_id,
        form_name = %req.form_name,
        module_name = %req.module_name,
        user_id = %auth.user_id,
        "Form definition created"
    );

    Ok(Json(api_success(form)))
}

/// Update form definition
#[utoipa::path(
    put,
    path = "/api/v1/forms/{form_id}",
    request_body = UpdateFormDefinitionRequest,
    responses(
        (status = 200, description = "Form definition updated", body = FormDefinition)
    ),
    params(
        ("form_id" = Uuid, Path, description = "Form definition ID")
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_form_definition(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Path(form_id): Path<Uuid>,
    Json(req): Json<UpdateFormDefinitionRequest>,
) -> Result<Json<ApiResponse<FormDefinition>>, ApiError> {
    // Verify form exists and belongs to organization
    let existing = sqlx::query!(
        "SELECT id, version FROM form_definitions 
         WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL",
        form_id,
        auth.organization_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check form: {}", e)))?;

    let existing = match existing {
        Some(e) => e,
        None => return Err(ApiError::not_found("Form definition not found")),
    };

    // Build update query using QueryBuilder for dynamic updates
    let mut query_builder = sqlx::QueryBuilder::new("UPDATE form_definitions SET ");

    let mut has_updates = false;
    let mut bind_count = 1;

    if let Some(ref v) = req.display_name {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("display_name = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.description {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("description = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.form_schema {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("form_schema = ");
        query_builder.push_bind(v);
        query_builder.push(format!(", version = {}", existing.version + 1));
        has_updates = true;
    }
    if let Some(ref v) = req.form_layout {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("form_layout = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.validation_rules {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("validation_rules = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.submission_handler {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("submission_handler = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(v) = req.is_active {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("is_active = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(v) = req.require_approval {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("require_approval = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.requires_permission {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("requires_permission = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.required_roles {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("required_roles = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.allowed_roles {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("allowed_roles = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.tags {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("tags = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.category {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("category = ");
        query_builder.push_bind(v);
        has_updates = true;
    }
    if let Some(ref v) = req.icon {
        if has_updates {
            query_builder.push(", ");
        }
        query_builder.push("icon = ");
        query_builder.push_bind(v);
        has_updates = true;
    }

    if !has_updates {
        return Err(ApiError::validation("No fields to update"));
    }

    query_builder.push(", updated_at = NOW()");
    query_builder.push(" WHERE id = ");
    query_builder.push_bind(form_id);
    query_builder.push(" AND organization_id = ");
    query_builder.push_bind(auth.organization_id);
    query_builder.push(" AND deleted_at IS NULL");
    query_builder.push(
        " RETURNING 
            id, organization_id, form_name, form_slug, display_name, description,
            module_name, entity_type, form_schema, form_layout, validation_rules,
            submission_handler, is_active, is_template, allow_multiple_submissions,
            require_approval, requires_permission, required_roles, allowed_roles,
            version, parent_form_id, tags, category, icon,
            created_at, updated_at, created_by",
    );

    let form = query_builder
        .build_query_as::<FormDefinition>()
        .fetch_one(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update form: {}", e)))?;

    Ok(Json(api_success(form)))
}

/// Submit form data
#[utoipa::path(
    post,
    path = "/api/v1/forms/submit",
    request_body = SubmitFormRequest,
    responses(
        (status = 201, description = "Form submitted successfully", body = FormSubmission)
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn submit_form(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(req): Json<SubmitFormRequest>,
) -> Result<Json<ApiResponse<FormSubmission>>, ApiError> {
    // Verify form definition exists and is active
    let form_def = sqlx::query!(
        "SELECT id, version, require_approval, allow_multiple_submissions, submission_handler
         FROM form_definitions 
         WHERE id = $1 AND organization_id = $2 AND is_active = true AND deleted_at IS NULL",
        req.form_definition_id,
        auth.organization_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to verify form: {}", e)))?;

    let form_def = match form_def {
        Some(f) => f,
        None => return Err(ApiError::not_found("Form definition not found or inactive")),
    };

    // Check if multiple submissions are allowed
    if !form_def.allow_multiple_submissions {
        let existing = sqlx::query!(
            "SELECT id FROM form_submissions 
             WHERE form_definition_id = $1 AND organization_id = $2 
             AND submitted_by = $3 AND deleted_at IS NULL",
            req.form_definition_id,
            auth.organization_id,
            auth.user_id
        )
        .fetch_optional(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check existing submission: {}", e)))?;

        if existing.is_some() {
            return Err(ApiError::conflict(
                "Multiple submissions not allowed for this form",
            ));
        }
    }

    // Validate entity association if provided
    if (req.entity_type.is_some() && req.entity_id.is_none())
        || (req.entity_type.is_none() && req.entity_id.is_some())
    {
        return Err(ApiError::validation(
            "Both entity_type and entity_id must be provided together",
        ));
    }

    // Determine submission status
    let submission_status = if form_def.require_approval {
        "submitted" // Requires approval
    } else {
        "submitted" // Auto-approved
    };

    // Create submission
    let submission_id = Uuid::new_v4();
    let submission = sqlx::query_as::<_, FormSubmission>(
        "INSERT INTO form_submissions (
            id, organization_id, form_definition_id, submission_data,
            submission_status, entity_type, entity_id, submitted_by,
            submitted_at, form_version, notes
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
        )
        RETURNING 
            id, organization_id, form_definition_id, submission_data,
            submission_status, entity_type, entity_id, submitted_by,
            submitted_at, approved_by, approved_at, form_version,
            created_at, updated_at",
    )
    .bind(submission_id)
    .bind(auth.organization_id)
    .bind(req.form_definition_id)
    .bind(&req.submission_data)
    .bind(&submission_status)
    .bind(&req.entity_type)
    .bind(&req.entity_id)
    .bind(auth.user_id)
    .bind(if submission_status == "submitted" {
        Some(Utc::now())
    } else {
        None
    })
    .bind(form_def.version)
    .bind(&req.notes)
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create submission: {}", e)))?;

    // TODO: Call submission handler if specified
    if let Some(ref handler) = form_def.submission_handler {
        tracing::info!(
            submission_id = %submission_id,
            handler = %handler,
            "Would call submission handler"
        );
    }

    tracing::info!(
        submission_id = %submission_id,
        form_id = %req.form_definition_id,
        user_id = %auth.user_id,
        "Form submitted successfully"
    );

    Ok(Json(api_success(submission)))
}

/// List form submissions
#[utoipa::path(
    get,
    path = "/api/v1/forms/{form_id}/submissions",
    responses(
        (status = 200, description = "List of form submissions", body = Vec<FormSubmission>)
    ),
    params(
        ("form_id" = Uuid, Path, description = "Form definition ID")
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_form_submissions(
    Path(form_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<FormSubmission>>>, ApiError> {
    // Verify form belongs to organization
    let form_exists = sqlx::query!(
        "SELECT id FROM form_definitions 
         WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL",
        form_id,
        auth.organization_id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to verify form: {}", e)))?;

    if form_exists.is_none() {
        return Err(ApiError::not_found("Form definition not found"));
    }

    let submissions = sqlx::query_as::<_, FormSubmission>(
        "SELECT 
            id, organization_id, form_definition_id, submission_data,
            submission_status, entity_type, entity_id, submitted_by,
            submitted_at, approved_by, approved_at, form_version,
            created_at, updated_at
         FROM form_submissions
         WHERE form_definition_id = $1 AND organization_id = $2 AND deleted_at IS NULL
         ORDER BY created_at DESC",
    )
    .bind(form_id)
    .bind(auth.organization_id)
    .fetch_all(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch submissions: {}", e)))?;

    Ok(Json(api_success(submissions)))
}

/// Delete form definition (soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/forms/{form_id}",
    responses(
        (status = 204, description = "Form definition deleted")
    ),
    params(
        ("form_id" = Uuid, Path, description = "Form definition ID")
    ),
    tag = "forms",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_form_definition(
    Path(form_id): Path<Uuid>,
    State(server): State<RustCareServer>,
    auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    let rows_affected = sqlx::query(
        "UPDATE form_definitions 
         SET deleted_at = NOW() 
         WHERE id = $1 AND organization_id = $2 AND deleted_at IS NULL",
    )
    .bind(form_id)
    .bind(auth.organization_id)
    .execute(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to delete form: {}", e)))?
    .rows_affected();

    if rows_affected == 0 {
        Err(ApiError::not_found("Form definition not found"))
    } else {
        tracing::info!(form_id = %form_id, user_id = %auth.user_id, "Form definition deleted");
        Ok(StatusCode::NO_CONTENT)
    }
}
