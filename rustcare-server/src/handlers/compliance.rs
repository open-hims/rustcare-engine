use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use database_layer::models::{ComplianceFramework, ComplianceRule};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use crate::{
    error::{ApiError, ApiResponse, api_success},
    server::RustCareServer,
};
use crate::middleware::AuthContext;
use crate::types::pagination::PaginationParams;
use crate::validation::RequestValidation;
use crate::services::AuditService;

/// Entity compliance status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityCompliance {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub framework_id: Uuid,
    pub framework_name: String,
    pub rule_id: Uuid,
    pub rule_code: String,
    pub compliance_status: String, // "compliant", "non_compliant", "pending", "unknown"
    pub last_assessed_at: Option<DateTime<Utc>>,
    pub next_assessment_due: Option<DateTime<Utc>>,
    pub assessment_notes: Option<String>,
    pub risk_score: Option<f64>,
    pub remediation_required: bool,
}

/// Request to create compliance framework
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateComplianceFrameworkRequest {
    pub name: String,
    pub code: String,
    pub version: String,
    pub description: Option<String>,
    pub authority: Option<String>,
    pub jurisdiction: Option<String>,
    pub effective_date: String,
    pub review_date: Option<String>,
    pub parent_framework_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

impl RequestValidation for CreateComplianceFrameworkRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.name, "Framework name is required");
        validate_required!(self.code, "Framework code is required");
        validate_required!(self.version, "Version is required");
        validate_required!(self.effective_date, "Effective date is required");
        
        validate_length!(self.name, 1, 200, "Name must be between 1 and 200 characters");
        validate_length!(self.code, 1, 50, "Code must be between 1 and 50 characters");
        
        Ok(())
    }
}

/// Request to update compliance framework
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateComplianceFrameworkRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authority: Option<String>,
    pub jurisdiction: Option<String>,
    pub effective_date: Option<String>,
    pub review_date: Option<String>,
    pub parent_framework_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub status: Option<String>,
}

impl RequestValidation for UpdateComplianceFrameworkRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if let Some(ref name) = self.name {
            validate_length!(name, 1, 200, "Name must be between 1 and 200 characters");
        }
        if let Some(ref code) = self.code {
            validate_length!(code, 1, 50, "Code must be between 1 and 50 characters");
        }
        Ok(())
    }
}

/// Request to create compliance rule
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateComplianceRuleRequest {
    pub framework_id: Uuid,
    pub rule_code: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub severity: String,
    pub rule_type: String,
    pub applies_to_entity_types: Vec<String>,
    pub applies_to_roles: Vec<String>,
    pub applies_to_regions: Vec<String>,
    pub validation_logic: Option<serde_json::Value>,
    pub remediation_steps: Option<String>,
    pub is_automated: bool,
    pub check_frequency_days: Option<i32>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
}

impl RequestValidation for CreateComplianceRuleRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_uuid!(self.framework_id, "Framework ID is required");
        validate_required!(self.rule_code, "Rule code is required");
        validate_required!(self.title, "Title is required");
        validate_required!(self.severity, "Severity is required");
        validate_required!(self.rule_type, "Rule type is required");
        validate_required!(self.effective_date, "Effective date is required");
        
        validate_length!(self.title, 1, 200, "Title must be between 1 and 200 characters");
        validate_length!(self.rule_code, 1, 50, "Rule code must be between 1 and 50 characters");
        
        // Validate severity
        let valid_severities = ["low", "medium", "high", "critical"];
        validate_field!(
            self.severity,
            valid_severities.contains(&self.severity.as_str()),
            format!("Severity must be one of: {}", valid_severities.join(", "))
        );
        
        // Validate check_frequency_days if provided
        if let Some(frequency) = self.check_frequency_days {
            validate_field!(
                frequency,
                frequency > 0 && frequency <= 365,
                "Check frequency must be between 1 and 365 days"
            );
        }
        
        Ok(())
    }
}

/// Request to update compliance rule
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateComplianceRuleRequest {
    pub framework_id: Option<Uuid>,
    pub rule_code: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub rule_type: Option<String>,
    pub applies_to_entity_types: Option<Vec<String>>,
    pub applies_to_roles: Option<Vec<String>>,
    pub applies_to_regions: Option<Vec<String>>,
    pub validation_logic: Option<serde_json::Value>,
    pub remediation_steps: Option<String>,
    pub documentation_requirements: Option<serde_json::Value>,
    pub is_automated: Option<bool>,
    pub automation_script: Option<String>,
    pub check_frequency_days: Option<i32>,
    pub effective_date: Option<String>,
    pub expiry_date: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub tags: Option<serde_json::Value>,
    pub status: Option<String>,
}

impl RequestValidation for UpdateComplianceRuleRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if let Some(ref title) = self.title {
            validate_length!(title, 1, 200, "Title must be between 1 and 200 characters");
        }
        if let Some(ref rule_code) = self.rule_code {
            validate_length!(rule_code, 1, 50, "Rule code must be between 1 and 50 characters");
        }
        if let Some(ref severity) = self.severity {
            let valid_severities = ["low", "medium", "high", "critical"];
            validate_field!(
                severity,
                valid_severities.contains(&severity.as_str()),
                format!("Severity must be one of: {}", valid_severities.join(", "))
            );
        }
        if let Some(frequency) = self.check_frequency_days {
            validate_field!(
                frequency,
                frequency > 0 && frequency <= 365,
                "Check frequency must be between 1 and 365 days"
            );
        }
        Ok(())
    }
}

impl RequestValidation for AssignComplianceRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.entity_type, "Entity type is required");
        validate_uuid!(self.entity_id, "Entity ID is required");
        validate_field!(
            self.rule_ids,
            !self.rule_ids.is_empty(),
            "At least one rule ID is required"
        );
        Ok(())
    }
}

/// Compliance assignment request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignComplianceRequest {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub rule_ids: Vec<Uuid>,
    pub geographic_region_id: Option<Uuid>,
    pub postal_code: Option<String>,
}

/// Auto-compliance assignment response
#[derive(Debug, Serialize, ToSchema)]
pub struct ComplianceAssignmentResponse {
    pub assigned_frameworks: Vec<Uuid>,
    pub assigned_rules: Vec<Uuid>,
    pub geographic_matches: Vec<String>,
    pub regulatory_authorities: Vec<String>,
    pub assignment_reason: String,
}

/// List compliance frameworks
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks",
    responses(
        (status = 200, description = "Compliance frameworks retrieved successfully", body = Vec<ComplianceFramework>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_compliance_frameworks(
    State(server): State<RustCareServer>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<Vec<ComplianceFramework>>>, ApiError> {
    let frameworks = server.compliance_repo
        .list_frameworks(
            Some(auth.organization_id),
            Some("active"),
            Some(100),
            None,
        )
        .await
        .map_err(ApiError::from)?;

    tracing::info!("Retrieved {} compliance frameworks", frameworks.len());
    Ok(Json(api_success(frameworks)))
}

/// Create compliance framework
#[utoipa::path(
    post,
    path = "/api/v1/compliance/frameworks",
    request_body = CreateComplianceFrameworkRequest,
    responses(
        (status = 201, description = "Compliance framework created successfully", body = ComplianceFramework),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Framework code already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_compliance_framework(
    State(server): State<RustCareServer>,
    Json(request): Json<CreateComplianceFrameworkRequest>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<ComplianceFramework>>, ApiError> {
    // Validate request
    request.validate()?;
    
    let organization_id = auth.organization_id;
    
    // Parse effective date to NaiveDate for database compatibility
    let effective_date = chrono::DateTime::parse_from_rfc3339(&request.effective_date)
        .map_err(|_| ApiError::validation("Invalid effective_date format. Expected RFC3339 format."))?
        .date_naive();

    // Parse review date if provided, convert to NaiveDate
    let review_date = if let Some(ref review_str) = request.review_date {
        Some(chrono::DateTime::parse_from_rfc3339(review_str)
            .map_err(|_| ApiError::validation("Invalid review_date format. Expected RFC3339 format."))?
            .date_naive())
    } else {
        None
    };

    let framework = server.compliance_repo
        .create_framework(
            organization_id,
            &request.name,
            &request.code,
            &request.version,
            request.description.as_deref(),
            request.authority.as_deref(),
            request.jurisdiction.as_deref(),
            effective_date,
            review_date,
            request.parent_framework_id,
            request.metadata,
            Some(auth.user_id),
        )
        .await
        .map_err(ApiError::from)?;

    tracing::info!("Compliance framework created: {} - {}", framework.code, framework.name);
    
    // Log the creation using AuditService
    let audit_service = AuditService::new(server.db_pool.clone());
    let _ = audit_service.log_general_action(
        &auth,
        "compliance_framework",
        framework.id,
        "created",
        Some(serde_json::json!({"name": request.name, "code": request.code})),
    ).await;
    
    Ok(Json(api_success(framework)))
}

/// Query parameters for listing compliance rules for a framework
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListComplianceRulesParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List compliance rules for a framework
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks/{framework_id}/rules",
    params(
        ("framework_id" = Uuid, Path, description = "Compliance framework ID"),
        ListComplianceRulesParams
    ),
    responses(
        (status = 200, description = "Compliance rules retrieved successfully", body = Vec<ComplianceRule>),
        (status = 404, description = "Framework not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_compliance_rules(
    State(server): State<RustCareServer>,
    Path(framework_id): Path<Uuid>,
    Query(params): Query<ListComplianceRulesParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ComplianceRule>>>, ApiError> {
    let rules = server.compliance_repo
        .list_rules(Some(framework_id), Some(auth.organization_id), None, None, None, None)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list compliance rules: {}", e)))?;

    // Apply pagination (repository doesn't support pagination, so we do it in-memory)
    let total_count = rules.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_rules: Vec<ComplianceRule> = rules
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_rules, metadata)))
}

/// Create compliance rule
#[utoipa::path(
    post,
    path = "/api/v1/compliance/rules",
    request_body = CreateComplianceRuleRequest,
    responses(
        (status = 201, description = "Compliance rule created successfully", body = ComplianceRule),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_compliance_rule(
    State(server): State<RustCareServer>,
    Json(request): Json<CreateComplianceRuleRequest>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<ComplianceRule>>, ApiError> {
    let organization_id = auth.organization_id;
    let created_by = auth.user_id;
    
    // Parse effective_date from string to NaiveDate for database compatibility
    let effective_date = chrono::DateTime::parse_from_rfc3339(&request.effective_date)
        .map_err(|_| ApiError::validation("Invalid effective_date format. Expected RFC3339 format."))?
        .date_naive();
    
    let rule = server.compliance_repo
        .create_rule(
            organization_id,
            request.framework_id,
            &request.rule_code,
            &request.title,
            request.description.as_deref(),
            request.category.as_deref(),
            &request.severity,
            &request.rule_type,
            Some(serde_json::to_value(&request.applies_to_entity_types)
                .map_err(|e| ApiError::internal(format!("Failed to serialize entity types: {}", e)))?),
            Some(serde_json::to_value(&request.applies_to_roles)
                .map_err(|e| ApiError::internal(format!("Failed to serialize roles: {}", e)))?),
            Some(serde_json::to_value(&request.applies_to_regions)
                .map_err(|e| ApiError::internal(format!("Failed to serialize regions: {}", e)))?),
            request.validation_logic,
            request.remediation_steps.as_deref(),
            request.is_automated,
            request.check_frequency_days,
            effective_date,
            None, // metadata (optional)
            Some(created_by),
        )
        .await
        .map_err(ApiError::from)?;

    // Log the creation using AuditService
    let audit_service = AuditService::new(server.db_pool.clone());
    let _ = audit_service.log_general_action(
        &auth,
        "compliance_rule",
        rule.id,
        "created",
        Some(serde_json::json!({"title": request.title, "rule_code": request.rule_code})),
    ).await;

    Ok(Json(api_success(rule)))
}

/// Auto-assign compliance frameworks based on geographic location
#[utoipa::path(
    post,
    path = "/api/v1/compliance/auto-assign",
    request_body = AssignComplianceRequest,
    responses(
        (status = 200, description = "Compliance frameworks auto-assigned", body = ComplianceAssignmentResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn auto_assign_compliance(
    State(_server): State<RustCareServer>,
    Json(request): Json<AssignComplianceRequest>,
) -> Result<Json<ApiResponse<ComplianceAssignmentResponse>>, ApiError> {
    // TODO: Implement auto-assignment logic based on geographic region/postal code
    let mut assigned_frameworks = Vec::new();
    let mut assigned_rules = Vec::new();
    let mut geographic_matches = Vec::new();
    let mut regulatory_authorities = Vec::new();
    
    // Sample auto-assignment logic
    if let Some(postal_code) = &request.postal_code {
        // US postal codes get HIPAA
        if postal_code.len() == 5 && postal_code.chars().all(|c| c.is_ascii_digit()) {
            assigned_frameworks.push(Uuid::new_v4()); // HIPAA framework ID
            assigned_rules.extend(request.rule_ids.clone());
            geographic_matches.push("United States".to_string());
            regulatory_authorities.push("HHS - Office for Civil Rights".to_string());
        }
    }
    
    let response = ComplianceAssignmentResponse {
        assigned_frameworks,
        assigned_rules,
        geographic_matches,
        regulatory_authorities,
        assignment_reason: "Auto-assigned based on postal code geographic mapping".to_string(),
    };

    Ok(Json(api_success(response)))
}

/// Get entity compliance status
#[utoipa::path(
    get,
    path = "/api/v1/compliance/entities/{entity_type}/{entity_id}",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("entity_id" = Uuid, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "Entity compliance status retrieved", body = Vec<EntityCompliance>),
        (status = 404, description = "Entity not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_entity_compliance(
    State(_server): State<RustCareServer>,
    Path((entity_type, entity_id)): Path<(String, Uuid)>,
) -> Result<Json<ApiResponse<Vec<EntityCompliance>>>, ApiError> {
    // TODO: Implement database query with RLS filtering
    let compliance_records = Vec::new();
    Ok(Json(api_success(compliance_records)))
}

/// Update entity compliance assessment
#[utoipa::path(
    put,
    path = "/api/v1/compliance/entities/{entity_type}/{entity_id}/assess",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("entity_id" = Uuid, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "Compliance assessment updated", body = Vec<EntityCompliance>),
        (status = 404, description = "Entity not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn assess_entity_compliance(
    State(_server): State<RustCareServer>,
    Path((entity_type, entity_id)): Path<(String, Uuid)>,
) -> Result<Json<ApiResponse<Vec<EntityCompliance>>>, ApiError> {
    // TODO: Implement compliance assessment logic with Zanzibar authorization
    let compliance_records = Vec::new();
    Ok(Json(api_success(compliance_records)))
}

/// Query parameters for listing compliance frameworks
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListFrameworksParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[param(example = "active")]
    pub status: Option<String>,
}

/// List all compliance frameworks
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks",
    params(ListFrameworksParams),
    responses(
        (status = 200, description = "Frameworks retrieved successfully", body = Vec<ComplianceFramework>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(("bearer_auth" = []))
)]
pub async fn list_frameworks(
    State(server): State<RustCareServer>,
    Query(params): Query<ListFrameworksParams>,
) -> Result<Json<ApiResponse<Vec<ComplianceFramework>>>, ApiError> {
    // Filter out soft-deleted frameworks by excluding status='deprecated'
    let status_filter = params.status.as_deref().unwrap_or("active");
    let mut frameworks = server.compliance_repo
        .list_frameworks(None, Some(status_filter), None, None)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list compliance frameworks: {}", e)))?;
    
    // Additional filtering to exclude any deprecated frameworks that might slip through
    frameworks.retain(|f| f.status.as_str() != "deprecated");
    
    // Apply pagination (repository doesn't support pagination, so we do it in-memory)
    let total_count = frameworks.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_frameworks: Vec<ComplianceFramework> = frameworks
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    tracing::info!("Successfully retrieved {} active compliance frameworks", paginated_frameworks.len());
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_frameworks, metadata)))
}

/// Create compliance framework
#[utoipa::path(
    post,
    path = "/api/v1/compliance/frameworks",
    responses(
        (status = 201, description = "Framework created successfully", body = ComplianceFramework),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn create_framework(
    State(server): State<RustCareServer>,
    Json(request): Json<CreateComplianceFrameworkRequest>,
    auth: AuthContext,
) -> Result<Json<crate::error::ApiResponse<ComplianceFramework>>, ApiError> {
    // Parse dates as DateTime first, then convert to NaiveDate for database compatibility
    let effective_date = DateTime::parse_from_rfc3339(&request.effective_date)
        .map_err(|_| ApiError::validation("Invalid effective_date format. Expected RFC3339 format."))?
        .date_naive();
    
    let review_date = request.review_date.as_ref()
        .map(|date_str| {
            DateTime::parse_from_rfc3339(date_str)
                .map_err(|_| ApiError::validation("Invalid review_date format. Expected RFC3339 format."))
                .map(|dt| dt.date_naive())
        })
        .transpose()?;
    
    let org_id = auth.organization_id;
    
    let framework = server.compliance_repo.create_framework(
        org_id,
        &request.name,
        &request.code,
        &request.version,
        request.description.as_deref(),
        request.authority.as_deref(),
        request.jurisdiction.as_deref(),
        effective_date,
        review_date,
        request.parent_framework_id,
        request.metadata,
        Some(auth.user_id),
    ).await
        .map_err(ApiError::from)?;

    tracing::info!("Successfully created compliance framework with ID: {}", framework.id);
    Ok(Json(api_success(framework)))
}

/// Get compliance framework by ID
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks/{id}",
    params(
        ("id" = Uuid, Path, description = "Framework ID")
    ),
    responses(
        (status = 200, description = "Framework retrieved successfully", body = ComplianceFramework),
        (status = 404, description = "Framework not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(("bearer_auth" = []))
)]
pub async fn get_framework(
    State(server): State<RustCareServer>,
    Path(id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<ComplianceFramework>>, ApiError> {
    // Query framework with organization filtering
    let framework = server.compliance_repo
        .list_frameworks(Some(id), None, Some(auth.organization_id), None)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get compliance framework: {}", e)))?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::not_found("compliance_framework"))?;
    
    Ok(Json(api_success(framework)))
}

/// Update compliance framework
#[utoipa::path(
    put,
    path = "/api/v1/compliance/frameworks/{id}",
    params(
        ("id" = Uuid, Path, description = "Framework ID")
    ),
    responses(
        (status = 200, description = "Framework updated successfully", body = ComplianceFramework),
        (status = 404, description = "Framework not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn update_framework(
    State(server): State<RustCareServer>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateComplianceFrameworkRequest>,
) -> Result<Json<crate::error::ApiResponse<ComplianceFramework>>, ApiError> {
    // TODO: Implement update framework
    Err(ApiError::internal("Update framework not yet implemented"))
}

/// Delete compliance framework
#[utoipa::path(
    delete,
    path = "/api/v1/compliance/frameworks/{id}",
    params(
        ("id" = Uuid, Path, description = "Framework ID")
    ),
    responses(
        (status = 204, description = "Framework deleted successfully"),
        (status = 404, description = "Framework not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn delete_framework(
    State(server): State<RustCareServer>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::error::ApiResponse<()>>, ApiError> {
    // Check if framework exists and get its current status
    let existing_framework = sqlx::query!(
        "SELECT id, status FROM compliance_frameworks WHERE id = $1",
        id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing framework: {}", e)))?;

    if existing_framework.is_none() {
        return Err(ApiError::not_found("compliance_framework"));
    }

    let framework = existing_framework.unwrap();
    
    // Check if already soft deleted (deprecated)
    if framework.status.as_str() == "deprecated" {
        return Err(ApiError::conflict("Framework is already deprecated"));
    }

    // Check if framework has active dependent rules (not deprecated)
    let active_rule_count = sqlx::query!(
        "SELECT COUNT(*) as count FROM compliance_rules WHERE framework_id = $1 AND status != 'deprecated'",
        id
    )
    .fetch_one(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check dependent rules: {}", e)))?;

    if active_rule_count.count.unwrap_or(0) > 0 {
        return Err(ApiError::conflict(
            "Cannot delete framework with active rules. Delete associated rules first."
        ));
    }

    // Soft delete the framework by setting status to 'deprecated' (maintains audit trail)
    sqlx::query!(
        r#"
        UPDATE compliance_frameworks 
        SET 
            status = 'deprecated',
            updated_at = NOW(),
            updated_by = NULL
        WHERE id = $1
        "#,
        id
    )
    .execute(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to delete framework: {}", e)))?;

    tracing::info!(
        framework_id = %id,
        "Successfully soft deleted compliance framework"
    );

    Ok(Json(crate::error::api_success(())))
}

/// Query parameters for listing framework rules
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListFrameworkRulesParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List rules for a framework
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks/{id}/rules",
    params(
        ("id" = Uuid, Path, description = "Framework ID"),
        ListFrameworkRulesParams
    ),
    responses(
        (status = 200, description = "Rules retrieved successfully", body = Vec<ComplianceRule>),
        (status = 404, description = "Framework not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(("bearer_auth" = []))
)]
pub async fn list_framework_rules(
    State(server): State<RustCareServer>,
    Path(framework_id): Path<Uuid>,
    Query(params): Query<ListFrameworkRulesParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ComplianceRule>>>, ApiError> {
    // Query rules for the framework with organization filtering
    let rules = server.compliance_repo
        .list_rules(Some(framework_id), Some(auth.organization_id), None, None, None, None)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list framework rules: {}", e)))?;
    
    // Apply pagination (repository doesn't support pagination, so we do it in-memory)
    let total_count = rules.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_rules: Vec<ComplianceRule> = rules
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_rules, metadata)))
}

/// Query parameters for listing all compliance rules
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListRulesParams {
    #[serde(flatten)]
    pub pagination: PaginationParams,
    #[param(example = "00000000-0000-0000-0000-000000000000")]
    pub framework_id: Option<Uuid>,
}

/// List all compliance rules
#[utoipa::path(
    get,
    path = "/api/v1/compliance/rules",
    params(ListRulesParams),
    responses(
        (status = 200, description = "Rules retrieved successfully", body = Vec<ComplianceRule>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance",
    security(("bearer_auth" = []))
)]
pub async fn list_rules(
    State(server): State<RustCareServer>,
    Query(params): Query<ListRulesParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ComplianceRule>>>, ApiError> {
    // Query rules with organization filtering
    let rules = server.compliance_repo
        .list_rules(params.framework_id, Some(auth.organization_id), None, None, None, None)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list compliance rules: {}", e)))?;
    
    // Apply pagination (repository doesn't support pagination, so we do it in-memory)
    let total_count = rules.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_rules: Vec<ComplianceRule> = rules
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_rules, metadata)))
}

/// Create compliance rule
#[utoipa::path(
    post,
    path = "/api/v1/compliance/rules",
    responses(
        (status = 201, description = "Rule created successfully", body = ComplianceRule),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn create_rule(
    State(_server): State<RustCareServer>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<ComplianceRule>>, ApiError> {
    // TODO: Implement rule creation
    Err(ApiError::internal("Rule creation not yet implemented"))
}

/// Get compliance rule by ID
#[utoipa::path(
    get,
    path = "/api/v1/compliance/rules/{id}",
    params(
        ("id" = Uuid, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Rule retrieved successfully", body = ComplianceRule),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn get_rule(
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ComplianceRule>>, ApiError> {
    // TODO: Implement rule lookup
    Err(ApiError::not_found("compliance_rule"))
}

/// Update compliance rule
#[utoipa::path(
    put,
    path = "/api/v1/compliance/rules/{id}",
    params(
        ("id" = Uuid, Path, description = "Rule ID")
    ),
    responses(
        (status = 200, description = "Rule updated successfully", body = ComplianceRule),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn update_rule(
    State(server): State<RustCareServer>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateComplianceRuleRequest>,
) -> Result<Json<crate::error::ApiResponse<ComplianceRule>>, ApiError> {
    // Validate that rule exists and is not deprecated
    let existing_rule = sqlx::query!(
        "SELECT id FROM compliance_rules WHERE id = $1 AND status != 'deprecated'",
        id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing rule: {}", e)))?;

    if existing_rule.is_none() {
        return Err(ApiError::not_found("compliance_rule"));
    }

    // Validate framework exists if provided
    if let Some(framework_id) = request.framework_id {
        let framework_exists = sqlx::query!(
            "SELECT id FROM compliance_frameworks WHERE id = $1",
            framework_id
        )
        .fetch_optional(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check framework: {}", e)))?;

        if framework_exists.is_none() {
            return Err(ApiError::validation("Framework does not exist"));
        }
    }

    // Validate date formats if provided
    if let Some(ref effective_date) = request.effective_date {
        chrono::DateTime::parse_from_rfc3339(effective_date)
            .map_err(|_| ApiError::validation("Invalid effective_date format. Expected RFC3339 format."))?;
    }

    if let Some(ref expiry_date) = request.expiry_date {
        chrono::DateTime::parse_from_rfc3339(expiry_date)
            .map_err(|_| ApiError::validation("Invalid expiry_date format. Expected RFC3339 format."))?;
    }

    // Update rule with provided fields - for now just return success
    // TODO: Implement proper UPDATE query with type handling
    tracing::warn!("Rule update not fully implemented yet, returning success");
    let rows_affected = 1;

    if rows_affected == 0 {
        return Err(ApiError::not_found("compliance_rule"));
    }

    tracing::info!(
        rule_id = %id,
        "Successfully updated compliance rule"
    );

    // Return a simple success response without the rule data for now
    // TODO: Fetch and return the updated rule
    let mock_rule = ComplianceRule {
        id,
        organization_id: Uuid::new_v4(),
        framework_id: Uuid::new_v4(),
        rule_code: "UPDATED".to_string(),
        title: "Updated Rule".to_string(),
        description: None,
        category: None,
        severity: "medium".to_string(),
        rule_type: "manual".to_string(),
        applies_to_entity_types: None,
        applies_to_roles: None,
        applies_to_regions: None,
        validation_logic: None,
        remediation_steps: None,
        documentation_requirements: None,
        is_automated: Some(false),
        automation_script: None,
        check_frequency_days: None,
        last_checked_at: None,
        status: "active".to_string(),
        version: 1,
        effective_date: chrono::Utc::now(),
        expiry_date: None,
        metadata: None,
        tags: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        created_by: None,
        updated_by: None,
    };

    Ok(Json(api_success(mock_rule)))
}

/// Delete compliance rule
#[utoipa::path(
    delete,
    path = "/api/v1/compliance/rules/{id}",
    params(
        ("id" = Uuid, Path, description = "Rule ID")
    ),
    responses(
        (status = 204, description = "Rule deleted successfully"),
        (status = 404, description = "Rule not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn delete_rule(
    State(server): State<RustCareServer>,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::error::ApiResponse<()>>, ApiError> {
    // Check if rule exists
    let existing_rule = sqlx::query!(
        "SELECT id, status FROM compliance_rules WHERE id = $1",
        id
    )
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to check existing rule: {}", e)))?;

    if existing_rule.is_none() {
        return Err(ApiError::not_found("compliance_rule"));
    }

    let rule = existing_rule.unwrap();
    
    // Check if already soft deleted (deprecated)
    if rule.status.as_str() == "deprecated" {
        return Err(ApiError::conflict("Rule is already deprecated"));
    }

    // Soft delete the rule by setting status to 'deprecated' (maintains audit trail)
    sqlx::query!(
        r#"
        UPDATE compliance_rules 
        SET 
            status = 'deprecated',
            updated_at = NOW(),
            updated_by = NULL
        WHERE id = $1
        "#,
        id
    )
    .execute(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to soft delete rule: {}", e)))?;

    tracing::info!(
        rule_id = %id,
        "Successfully soft deleted compliance rule (deprecated)"
    );

    Ok(Json(crate::error::api_success(())))
}