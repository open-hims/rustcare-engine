use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    response::{IntoResponse, Json as ResponseJson},
};
use chrono::{DateTime, NaiveDate, Utc};
use database_layer::models::{ComplianceFramework, ComplianceRule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use utoipa::ToSchema;
use crate::{
    error::{ApiError, ApiResult, api_success, api_paginated},
    server::RustCareServer,
};

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
) -> Result<Json<crate::error::ApiResponse<Vec<ComplianceFramework>>>, ApiError> {
    // For now, we'll get frameworks without organization filtering
    // TODO: Add proper organization context from authentication
    let frameworks = server.compliance_repo
        .list_frameworks(
            None, // organization_id - TODO: get from auth context
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
) -> Result<Json<crate::error::ApiResponse<ComplianceFramework>>, ApiError> {
    // TODO: Get organization ID from authentication context
    let organization_id = Uuid::new_v4(); // Placeholder
    
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
            None, // created_by - TODO: get from auth context
        )
        .await
        .map_err(ApiError::from)?;

    tracing::info!("Compliance framework created: {} - {}", framework.code, framework.name);
    Ok(Json(api_success(framework)))
}

/// List compliance rules for a framework
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks/{framework_id}/rules",
    params(
        ("framework_id" = Uuid, Path, description = "Compliance framework ID")
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
) -> Result<ResponseJson<Vec<ComplianceRule>>, StatusCode> {
    let rules = server.compliance_repo
        .list_rules(Some(framework_id), None, None, None, None, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rules))
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
) -> Result<Json<crate::error::ApiResponse<ComplianceRule>>, ApiError> {
    // TODO: Get actual organization_id from auth context
    let organization_id = Uuid::new_v4();
    let created_by = Uuid::new_v4(); // TODO: Get from auth context
    
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
            None, // metadata
            Some(created_by),
        )
        .await
        .map_err(ApiError::from)?;

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
) -> Result<ResponseJson<ComplianceAssignmentResponse>, StatusCode> {
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

    Ok(Json(response))
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
) -> Result<ResponseJson<Vec<EntityCompliance>>, StatusCode> {
    // TODO: Implement database query with RLS filtering
    let compliance_records = Vec::new();
    Ok(Json(compliance_records))
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
) -> Result<ResponseJson<Vec<EntityCompliance>>, StatusCode> {
    // TODO: Implement compliance assessment logic with Zanzibar authorization
    let compliance_records = Vec::new();
    Ok(Json(compliance_records))
}

/// List all compliance frameworks
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks",
    responses(
        (status = 200, description = "Frameworks retrieved successfully", body = Vec<ComplianceFramework>),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn list_frameworks(
    State(server): State<RustCareServer>,
) -> Result<ResponseJson<Vec<ComplianceFramework>>, StatusCode> {
    match server.compliance_repo.list_frameworks(None, None, None, None).await {
        Ok(frameworks) => {
            tracing::info!("Successfully retrieved {} compliance frameworks", frameworks.len());
            Ok(Json(frameworks))
        },
        Err(e) => {
            tracing::error!("Failed to list compliance frameworks: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
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
    
    // For now, use a default organization ID (will be replaced with proper auth later)
    let org_id = Uuid::nil();
    
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
        None, // created_by - will be set from auth context later
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
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn get_framework(
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<ResponseJson<ComplianceFramework>, StatusCode> {
    // TODO: Implement framework lookup
    Err(StatusCode::NOT_FOUND)
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
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<serde_json::Value>,
) -> Result<ResponseJson<ComplianceFramework>, StatusCode> {
    // TODO: Implement framework update
    Err(StatusCode::NOT_FOUND)
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
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Implement framework deletion
    Err(StatusCode::NOT_FOUND)
}

/// List rules for a framework
#[utoipa::path(
    get,
    path = "/api/v1/compliance/frameworks/{id}/rules",
    params(
        ("id" = Uuid, Path, description = "Framework ID")
    ),
    responses(
        (status = 200, description = "Rules retrieved successfully", body = Vec<ComplianceRule>),
        (status = 404, description = "Framework not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn list_framework_rules(
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<ResponseJson<Vec<ComplianceRule>>, StatusCode> {
    // TODO: Implement framework rules lookup
    let rules = Vec::new();
    Ok(Json(rules))
}

/// List all compliance rules
#[utoipa::path(
    get,
    path = "/api/v1/compliance/rules",
    responses(
        (status = 200, description = "Rules retrieved successfully", body = Vec<ComplianceRule>),
        (status = 500, description = "Internal server error")
    ),
    tag = "compliance"
)]
pub async fn list_rules(
    State(_server): State<RustCareServer>,
) -> Result<ResponseJson<Vec<ComplianceRule>>, StatusCode> {
    // TODO: Implement database query
    let rules = Vec::new();
    Ok(Json(rules))
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
) -> Result<ResponseJson<ComplianceRule>, StatusCode> {
    // TODO: Implement rule creation
    Err(StatusCode::NOT_IMPLEMENTED)
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
) -> Result<ResponseJson<ComplianceRule>, StatusCode> {
    // TODO: Implement rule lookup
    Err(StatusCode::NOT_FOUND)
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
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<serde_json::Value>,
) -> Result<ResponseJson<ComplianceRule>, StatusCode> {
    // TODO: Implement rule update
    Err(StatusCode::NOT_FOUND)
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
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Implement rule deletion
    Err(StatusCode::NOT_FOUND)
}