use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::server::RustCareServer;
use crate::middleware::AuthContext;
use crate::error::{ApiError, ApiResponse, api_success};
use crate::types::pagination::PaginationParams;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Workflow definition
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowDefinition {
    /// Workflow unique identifier
    #[schema(example = "patient-admission")]
    pub id: String,
    /// Workflow display name
    #[schema(example = "Patient Admission Workflow")]
    pub name: String,
    /// Workflow description
    #[schema(example = "Complete patient admission process")]
    pub description: String,
    /// Workflow version
    #[schema(example = "1.0.0")]
    pub version: String,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
    pub triggers: Vec<WorkflowTrigger>,
    pub metadata: HashMap<String, String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Workflow step
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: String,
    pub action: String,
    pub conditions: Vec<String>,
    pub next_steps: Vec<String>,
    pub timeout_seconds: Option<u64>,
    pub retry_config: Option<RetryConfig>,
}

/// Workflow trigger
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct WorkflowTrigger {
    pub id: String,
    pub trigger_type: String,
    pub event: String,
    pub conditions: Vec<String>,
    pub enabled: bool,
}

/// Retry configuration
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay_seconds: u64,
    pub backoff_multiplier: f64,
}

/// Workflow execution request
#[derive(Debug, Deserialize, ToSchema)]
pub struct WorkflowExecutionRequest {
    /// ID of the workflow to execute
    #[schema(example = "patient-admission")]
    pub workflow_id: String,
    /// Input data for the workflow
    pub input_data: HashMap<String, serde_json::Value>,
    /// Execution context
    pub execution_context: Option<HashMap<String, String>>,
    /// Execution priority
    #[schema(example = "high")]
    pub priority: Option<String>,
}

// Additional response schemas for OpenAPI
/// Workflow list response
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowListResponse {
    /// Available workflows
    pub workflows: Vec<WorkflowSummary>,
    /// Total count
    pub total: usize,
}

/// Workflow summary
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowSummary {
    /// Workflow ID
    #[schema(example = "patient-admission")]
    pub id: String,
    /// Workflow name
    #[schema(example = "Patient Admission")]
    pub name: String,
    /// Brief description
    pub description: String,
    /// Current version
    pub version: String,
    /// Whether active
    pub active: bool,
}

/// Workflow response
pub type WorkflowResponse = WorkflowDefinition;

/// Execution status response
#[derive(Debug, Serialize, ToSchema)]
pub struct ExecutionStatusResponse {
    /// Execution ID
    #[schema(example = "exec_123456")]
    pub execution_id: String,
    /// Current status
    #[schema(example = "running")]
    pub status: String,
    /// Progress percentage
    #[schema(example = 75)]
    pub progress: u8,
    /// Current step
    pub current_step: Option<String>,
    /// Result if completed
    pub result: Option<serde_json::Value>,
    /// Error if failed
    pub error: Option<String>,
}

/// Workflow execution response
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct WorkflowExecutionResponse {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub started_at: String,
    pub current_step: Option<String>,
    pub progress_percent: f64,
}

/// Workflow execution status
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct WorkflowExecutionStatus {
    pub execution_id: String,
    pub workflow_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub current_step: Option<String>,
    pub completed_steps: Vec<String>,
    pub failed_steps: Vec<String>,
    pub progress_percent: f64,
    pub output_data: Option<HashMap<String, serde_json::Value>>,
    pub error_message: Option<String>,
}

/// List all workflows
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListWorkflowsParams {
    pub search: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::WORKFLOWS,
    params(ListWorkflowsParams),
    responses(
        (status = 200, description = "Workflows retrieved successfully", body = Vec<WorkflowSummary>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "workflow",
    security(("bearer_auth" = []))
)]
pub async fn list_workflows(
    State(server): State<RustCareServer>,
    Query(params): Query<ListWorkflowsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<WorkflowSummary>>>, ApiError> {
    // TODO: Integrate with workflow-engine module
    // This is a placeholder implementation
    
    let sample_workflows = vec![
        WorkflowDefinition {
            id: "wf_patient_registration".to_string(),
            name: "Patient Registration".to_string(),
            description: "Complete patient registration workflow with HIPAA compliance".to_string(),
            version: "1.0.0".to_string(),
            steps: vec![
                WorkflowStep {
                    id: "step_verify_identity".to_string(),
                    name: "Verify Identity".to_string(),
                    step_type: "validation".to_string(),
                    action: "verify_patient_identity".to_string(),
                    conditions: vec!["has_valid_id".to_string()],
                    next_steps: vec!["step_create_record".to_string()],
                    timeout_seconds: Some(300),
                    retry_config: None,
                }
            ],
            triggers: vec![
                WorkflowTrigger {
                    id: "trigger_new_patient".to_string(),
                    trigger_type: "event".to_string(),
                    event: "patient.registration.started".to_string(),
                    conditions: vec![],
                    enabled: true,
                }
            ],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    ];

    // Convert to summaries
    let mut workflows: Vec<WorkflowSummary> = sample_workflows
        .into_iter()
        .map(|wf| WorkflowSummary {
            id: wf.id,
            name: wf.name,
            description: wf.description,
            version: wf.version,
            active: true,
        })
        .collect();

    // Apply pagination
    let total_count = workflows.len() as i64;
    let offset = params.pagination.offset() as usize;
    let limit = params.pagination.limit() as usize;
    let paginated_workflows: Vec<WorkflowSummary> = workflows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(paginated_workflows, metadata)))
}

/// Get workflow by ID
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::WORKFLOW_BY_ID,
    params(("workflow_id" = String, Path, description = "Workflow ID")),
    responses(
        (status = 200, description = "Workflow retrieved successfully", body = WorkflowDefinition),
        (status = 404, description = "Workflow not found"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "workflow",
    security(("bearer_auth" = []))
)]
pub async fn get_workflow(
    State(server): State<RustCareServer>,
    Path(workflow_id): Path<String>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<WorkflowDefinition>>, ApiError> {
    // TODO: Integrate with workflow-engine module
    // This is a placeholder implementation
    
    if workflow_id == "wf_patient_registration" {
        let workflow = WorkflowDefinition {
            id: workflow_id,
            name: "Patient Registration".to_string(),
            description: "Complete patient registration workflow with HIPAA compliance".to_string(),
            version: "1.0.0".to_string(),
            steps: vec![],
            triggers: vec![],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        Ok(Json(api_success(workflow)))
    } else {
        Err(ApiError::not_found("workflow"))
    }
}

/// Execute workflow
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::WORKFLOW_EXECUTE,
    request_body = WorkflowExecutionRequest,
    responses(
        (status = 200, description = "Workflow execution started", body = WorkflowExecutionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "workflow",
    security(("bearer_auth" = []))
)]
pub async fn execute_workflow(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(execution_request): Json<WorkflowExecutionRequest>,
) -> Result<Json<ApiResponse<WorkflowExecutionResponse>>, ApiError> {
    // TODO: Integrate with workflow-engine module
    // This is a placeholder implementation
    
    let execution_id = Uuid::new_v4().to_string();
    
    let response = WorkflowExecutionResponse {
        execution_id,
        workflow_id: execution_request.workflow_id,
        status: "running".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        current_step: Some("step_verify_identity".to_string()),
        progress_percent: 0.0,
    };

    Ok(Json(api_success(response)))
}

/// Get workflow execution status
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::WORKFLOW_EXECUTION_BY_ID,
    params(("execution_id" = String, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Execution status", body = WorkflowExecutionStatus),
        (status = 404, description = "Execution not found"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "workflow",
    security(("bearer_auth" = []))
)]
pub async fn get_execution_status(
    State(server): State<RustCareServer>,
    Path(execution_id): Path<String>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<WorkflowExecutionStatus>>, ApiError> {
    // TODO: Integrate with workflow-engine module
    // This is a placeholder implementation
    
    let status = WorkflowExecutionStatus {
        execution_id,
        workflow_id: "wf_patient_registration".to_string(),
        status: "completed".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
        current_step: None,
        completed_steps: vec!["step_verify_identity".to_string()],
        failed_steps: vec![],
        progress_percent: 100.0,
        output_data: Some(HashMap::new()),
        error_message: None,
    };

    Ok(Json(api_success(status)))
}

/// Cancel workflow execution
#[utoipa::path(
    delete,
    path = crate::routes::paths::api_v1::WORKFLOW_EXECUTION_BY_ID,
    params(("execution_id" = String, Path, description = "Execution ID")),
    responses(
        (status = 204, description = "Execution cancelled"),
        (status = 404, description = "Execution not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "workflow",
    security(("bearer_auth" = []))
)]
pub async fn cancel_execution(
    State(server): State<RustCareServer>,
    Path(execution_id): Path<String>,
    auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    // TODO: Integrate with workflow-engine module
    // This is a placeholder implementation
    
    // Simulate cancellation logic
    Ok(StatusCode::NO_CONTENT)
}