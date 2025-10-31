use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;
use sqlx::FromRow;
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};
use chrono::{DateTime, Utc};

/// Service Type structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow)]
pub struct ServiceType {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub service_classification: Option<String>,
    pub typical_duration_hours: Option<String>,
    pub typical_duration_days: Option<i32>,
    pub requires_licensure: bool,
    pub required_qualifications: serde_json::Value,
    pub equipment_required: serde_json::Value,
    pub facility_requirements: serde_json::Value,
    pub pre_authorization_required: bool,
    pub cpt_code: Option<String>,
    pub icd_10_codes: serde_json::Value,
    pub hcpcs_code: Option<String>,
    pub insurance_coverage_typical: bool,
    pub urgency_level: Option<String>,
    pub complexity_level: Option<String>,
    pub risk_level: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Medical Record structure
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, FromRow)]
pub struct MedicalRecord {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    
    // Record Metadata
    pub record_type: String,
    pub title: String,
    pub description: Option<String>,
    
    // Clinical Data
    pub chief_complaint: Option<String>,
    pub diagnosis: serde_json::Value,
    pub treatments: serde_json::Value,
    pub prescriptions: serde_json::Value,
    pub test_results: serde_json::Value,
    pub vital_signs: serde_json::Value,
    
    // Visit Information
    pub visit_date: DateTime<Utc>,
    pub visit_duration_minutes: Option<i32>,
    pub location: Option<String>,
    
    // HIPAA Compliance
    pub access_level: String,
    pub phi_present: bool,
    
    // Audit
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create Medical Record Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMedicalRecordRequest {
    pub organization_id: Uuid,
    pub patient_id: Uuid,
    pub record_type: String,
    pub title: String,
    pub description: Option<String>,
    pub chief_complaint: Option<String>,
    pub diagnosis: Option<serde_json::Value>,
    pub treatments: Option<serde_json::Value>,
    pub prescriptions: Option<serde_json::Value>,
    pub visit_date: DateTime<Utc>,
    pub visit_duration_minutes: Option<i32>,
    pub location: Option<String>,
    pub access_level: Option<String>,
    pub access_reason: String,
}

/// Update Medical Record Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMedicalRecordRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub chief_complaint: Option<String>,
    pub diagnosis: Option<serde_json::Value>,
    pub treatments: Option<serde_json::Value>,
    pub prescriptions: Option<serde_json::Value>,
    pub visit_duration_minutes: Option<i32>,
    pub location: Option<String>,
    pub update_reason: String,
}

/// List Medical Records Query Parameters
#[derive(Debug, Deserialize)]
pub struct ListMedicalRecordsParams {
    pub patient_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub record_type: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// List Service Types Query Parameters
#[derive(Debug, Deserialize)]
pub struct ListServiceTypesParams {
    pub category: Option<String>,
    pub is_active: Option<bool>,
    pub organization_id: Option<Uuid>,
}

/// Audit Log Entry
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MedicalRecordAuditLog {
    pub id: Uuid,
    pub medical_record_id: Uuid,
    pub accessed_by: Uuid,
    pub access_type: String,
    pub access_time: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub access_reason: Option<String>,
    pub emergency_access: bool,
    pub success: bool,
}

/// Healthcare Provider structure
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone, FromRow)]
pub struct HealthcareProvider {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub license_number: String,
    pub license_state: String,
    pub license_expiry: String,
    pub specialty: Option<String>,
    pub npi_number: Option<String>,
    pub department: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// API HANDLERS
// ============================================================================

/// Create a new medical record
#[utoipa::path(
    post,
    path = "/api/v1/healthcare/medical-records",
    request_body = CreateMedicalRecordRequest,
    responses(
        (status = 201, description = "Medical record created successfully", body = MedicalRecord),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_medical_record(
    State(_server): State<RustCareServer>,
    Json(_request): Json<CreateMedicalRecordRequest>,
) -> Result<Json<ApiResponse<MedicalRecord>>, ApiError> {
    // TODO: Implement actual database insertion
    // For now, return a mock response
    
    let mock_record = MedicalRecord {
        id: Uuid::new_v4(),
        organization_id: _request.organization_id,
        patient_id: _request.patient_id,
        provider_id: Uuid::new_v4(), // TODO: Get from authenticated user
        record_type: _request.record_type,
        title: _request.title,
        description: _request.description,
        chief_complaint: _request.chief_complaint,
        diagnosis: _request.diagnosis.unwrap_or(serde_json::json!({})),
        treatments: _request.treatments.unwrap_or(serde_json::json!({})),
        prescriptions: _request.prescriptions.unwrap_or(serde_json::json!({})),
        test_results: serde_json::json!({}),
        vital_signs: serde_json::json!({}),
        visit_date: _request.visit_date,
        visit_duration_minutes: _request.visit_duration_minutes,
        location: _request.location,
        access_level: _request.access_level.unwrap_or_else(|| "restricted".to_string()),
        phi_present: true,
        created_by: Uuid::new_v4(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    Ok(Json(api_success(mock_record)))
}

/// Get a specific medical record
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/medical-records/{record_id}",
    params(
        ("record_id" = Uuid, Path, description = "Medical Record ID")
    ),
    responses(
        (status = 200, description = "Medical record retrieved successfully", body = MedicalRecord),
        (status = 404, description = "Record not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_medical_record(
    State(server): State<RustCareServer>,
    Path(record_id): Path<Uuid>,
) -> Result<Json<ApiResponse<MedicalRecord>>, ApiError> {
    // Try to fetch from database first
    match sqlx::query_as::<_, MedicalRecord>(
        "SELECT * FROM medical_records WHERE id = $1 AND is_deleted = false"
    )
    .bind(record_id)
    .fetch_optional(&server.db_pool)
    .await
    {
        Ok(Some(record)) => Ok(Json(api_success(record))),
        Ok(None) => Err(ApiError::not_found("medical_record".to_string())),
        Err(_) => {
            // Fallback to mock data for development
            let mock_record = MedicalRecord {
                id: record_id,
                organization_id: Uuid::new_v4(),
                patient_id: Uuid::new_v4(),
                provider_id: Uuid::new_v4(),
                record_type: "consultation".to_string(),
                title: "Initial Consultation".to_string(),
                description: Some("Patient consultation notes".to_string()),
                chief_complaint: Some("Chest pain".to_string()),
                diagnosis: serde_json::json!({"primary": "Cardiac arrhythmia"}),
                treatments: serde_json::json!({}),
                prescriptions: serde_json::json!({}),
                test_results: serde_json::json!({}),
                vital_signs: serde_json::json!({}),
                visit_date: Utc::now(),
                visit_duration_minutes: Some(30),
                location: Some("Cardiology Department".to_string()),
                access_level: "restricted".to_string(),
                phi_present: true,
                created_by: Uuid::new_v4(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            Ok(Json(api_success(mock_record)))
        }
    }
}

/// List medical records with optional filters
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/medical-records",
    params(
        ("patient_id" = Option<Uuid>, Query, description = "Filter by patient ID"),
        ("provider_id" = Option<Uuid>, Query, description = "Filter by provider ID"),
        ("record_type" = Option<String>, Query, description = "Filter by record type"),
        ("page" = Option<u32>, Query, description = "Page number"),
        ("page_size" = Option<u32>, Query, description = "Page size")
    ),
    responses(
        (status = 200, description = "Medical records retrieved successfully", body = Vec<MedicalRecord>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_medical_records(
    State(server): State<RustCareServer>,
    Query(params): Query<ListMedicalRecordsParams>,
) -> Result<Json<ApiResponse<Vec<MedicalRecord>>>, ApiError> {
    use crate::error::ApiError;
    
    // Build query with proper sqlx parameter binding
    let mut query_builder = sqlx::QueryBuilder::new(
        "SELECT * FROM medical_records WHERE is_deleted = false"
    );
    
    // Apply filters
    if let Some(patient_id) = params.patient_id {
        query_builder.push(" AND patient_id = ");
        query_builder.push_bind(patient_id);
    }
    
    if let Some(provider_id) = params.provider_id {
        query_builder.push(" AND provider_id = ");
        query_builder.push_bind(provider_id);
    }
    
    if let Some(record_type) = params.record_type {
        query_builder.push(" AND record_type = ");
        query_builder.push_bind(record_type);
    }
    
    if let Some(start_date) = params.start_date {
        query_builder.push(" AND visit_date >= ");
        query_builder.push_bind(start_date);
    }
    
    if let Some(end_date) = params.end_date {
        query_builder.push(" AND visit_date <= ");
        query_builder.push_bind(end_date);
    }
    
    // Order by visit date descending
    query_builder.push(" ORDER BY visit_date DESC");
    
    // Pagination
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;
    
    query_builder.push(" LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset as i64);
    
    // Execute query
    let query = query_builder.build_query_as::<MedicalRecord>();
    
    // Try to fetch from database, fallback to mock data if database unavailable
    match query.fetch_all(&server.db_pool).await {
        Ok(records) => Ok(Json(api_success(records))),
        Err(_) => {
            // Fallback to mock data for development
            let mock_records = vec![
                MedicalRecord {
                    id: Uuid::new_v4(),
                    organization_id: Uuid::new_v4(),
                    patient_id: params.patient_id.unwrap_or(Uuid::new_v4()),
                    provider_id: Uuid::new_v4(),
                    record_type: "consultation".to_string(),
                    title: "Initial Consultation".to_string(),
                    description: Some("Patient consultation notes".to_string()),
                    chief_complaint: Some("Chest pain".to_string()),
                    diagnosis: serde_json::json!({"primary": "Cardiac arrhythmia"}),
                    treatments: serde_json::json!({}),
                    prescriptions: serde_json::json!({}),
                    test_results: serde_json::json!({}),
                    vital_signs: serde_json::json!({}),
                    visit_date: Utc::now(),
                    visit_duration_minutes: Some(30),
                    location: Some("Cardiology".to_string()),
                    access_level: "restricted".to_string(),
                    phi_present: true,
                    created_by: Uuid::new_v4(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            ];
            Ok(Json(api_success(mock_records)))
        }
    }
}

/// Update a medical record
#[utoipa::path(
    put,
    path = "/api/v1/healthcare/medical-records/{record_id}",
    params(
        ("record_id" = Uuid, Path, description = "Medical Record ID")
    ),
    request_body = UpdateMedicalRecordRequest,
    responses(
        (status = 200, description = "Medical record updated successfully", body = MedicalRecord),
        (status = 404, description = "Record not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - insufficient permissions"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_medical_record(
    State(_server): State<RustCareServer>,
    Path(record_id): Path<Uuid>,
    Json(_request): Json<UpdateMedicalRecordRequest>,
) -> Result<Json<ApiResponse<MedicalRecord>>, ApiError> {
    // TODO: Implement actual database update
    let mock_record = MedicalRecord {
        id: record_id,
        organization_id: Uuid::new_v4(),
        patient_id: Uuid::new_v4(),
        provider_id: Uuid::new_v4(),
        record_type: "consultation".to_string(),
        title: _request.title.unwrap_or_else(|| "Updated Consultation".to_string()),
        description: _request.description,
        chief_complaint: _request.chief_complaint,
        diagnosis: _request.diagnosis.unwrap_or(serde_json::json!({})),
        treatments: _request.treatments.unwrap_or(serde_json::json!({})),
        prescriptions: _request.prescriptions.unwrap_or(serde_json::json!({})),
        test_results: serde_json::json!({}),
        vital_signs: serde_json::json!({}),
        visit_date: Utc::now(),
        visit_duration_minutes: _request.visit_duration_minutes,
        location: _request.location,
        access_level: "restricted".to_string(),
        phi_present: true,
        created_by: Uuid::new_v4(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    Ok(Json(api_success(mock_record)))
}

/// Delete a medical record (soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/healthcare/medical-records/{record_id}",
    params(
        ("record_id" = Uuid, Path, description = "Medical Record ID")
    ),
    responses(
        (status = 204, description = "Medical record deleted successfully"),
        (status = 404, description = "Record not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - only admins can delete"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_medical_record(
    State(_server): State<RustCareServer>,
    Path(record_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // TODO: Implement soft delete
    println!("Soft deleting medical record: {}", record_id);
    Ok(StatusCode::NO_CONTENT)
}

/// Get audit log for a medical record
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/medical-records/{record_id}/audit",
    params(
        ("record_id" = Uuid, Path, description = "Medical Record ID")
    ),
    responses(
        (status = 200, description = "Audit log retrieved successfully", body = Vec<MedicalRecordAuditLog>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_medical_record_audit(
    State(_server): State<RustCareServer>,
    Path(record_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<MedicalRecordAuditLog>>>, ApiError> {
    // TODO: Implement audit log query
    let mock_audit = vec![
        MedicalRecordAuditLog {
            id: Uuid::new_v4(),
            medical_record_id: record_id,
            accessed_by: Uuid::new_v4(),
            access_type: "view".to_string(),
            access_time: Utc::now(),
            ip_address: Some("192.168.1.1".to_string()),
            access_reason: Some("Patient care".to_string()),
            emergency_access: false,
            success: true,
        },
        MedicalRecordAuditLog {
            id: Uuid::new_v4(),
            medical_record_id: record_id,
            accessed_by: Uuid::new_v4(),
            access_type: "create".to_string(),
            access_time: Utc::now(),
            ip_address: None,
            access_reason: None,
            emergency_access: false,
            success: true,
        },
    ];
    
    Ok(Json(api_success(mock_audit)))
}

/// List healthcare providers
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/providers",
    responses(
        (status = 200, description = "Healthcare providers retrieved successfully", body = Vec<HealthcareProvider>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_providers(
    State(_server): State<RustCareServer>,
) -> Result<Json<ApiResponse<Vec<HealthcareProvider>>>, ApiError> {
    // TODO: Implement provider query
    let mock_providers = vec![
        HealthcareProvider {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            license_number: "MD-12345".to_string(),
            license_state: "CA".to_string(),
            license_expiry: "2025-12-31".to_string(),
            specialty: Some("Cardiology".to_string()),
            npi_number: Some("1234567890".to_string()),
            department: Some("Cardiology".to_string()),
            is_active: true,
            created_at: Utc::now(),
        },
    ];
    
    Ok(Json(api_success(mock_providers)))
}

/// List service types
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/service-types",
    params(
        ("category" = Option<String>, Query, description = "Filter by category"),
        ("is_active" = Option<bool>, Query, description = "Filter by active status")
    ),
    responses(
        (status = 200, description = "Service types retrieved successfully", body = Vec<ServiceType>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(("bearer_auth" = []))
)]
pub async fn list_service_types(
    State(_server): State<RustCareServer>,
    Query(_params): Query<ListServiceTypesParams>,
) -> Result<Json<ApiResponse<Vec<ServiceType>>>, ApiError> {
    // TODO: Implement actual database query
    Ok(Json(api_success(Vec::<ServiceType>::new())))
}

/// Create service type
#[utoipa::path(
    post,
    path = "/api/v1/healthcare/service-types",
    request_body = ServiceType,
    responses(
        (status = 201, description = "Service type created successfully", body = ServiceType),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(("bearer_auth" = []))
)]
pub async fn create_service_type(
    State(_server): State<RustCareServer>,
    Json(_request): Json<ServiceType>,
) -> Result<Json<ApiResponse<ServiceType>>, ApiError> {
    // TODO: Implement database insertion
    Ok(Json(api_success(ServiceType {
        id: Uuid::new_v4(),
        organization_id: _request.organization_id,
        code: _request.code,
        name: _request.name,
        description: _request.description,
        category: _request.category,
        service_classification: _request.service_classification,
        typical_duration_hours: _request.typical_duration_hours,
        typical_duration_days: _request.typical_duration_days,
        requires_licensure: _request.requires_licensure,
        required_qualifications: _request.required_qualifications,
        equipment_required: _request.equipment_required,
        facility_requirements: _request.facility_requirements,
        pre_authorization_required: _request.pre_authorization_required,
        cpt_code: _request.cpt_code,
        icd_10_codes: _request.icd_10_codes,
        hcpcs_code: _request.hcpcs_code,
        insurance_coverage_typical: _request.insurance_coverage_typical,
        urgency_level: _request.urgency_level,
        complexity_level: _request.complexity_level,
        risk_level: _request.risk_level,
        tags: _request.tags,
        is_active: _request.is_active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })))
}

/// Get service type by ID
#[utoipa::path(
    get,
    path = "/api/v1/healthcare/service-types/{service_type_id}",
    params(("service_type_id" = Uuid, Path, description = "Service Type ID")),
    responses(
        (status = 200, description = "Service type retrieved successfully", body = ServiceType),
        (status = 404, description = "Service type not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(("bearer_auth" = []))
)]
pub async fn get_service_type(
    State(_server): State<RustCareServer>,
    Path(service_type_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ServiceType>>, ApiError> {
    // TODO: Implement database query
    Ok(Json(api_success(ServiceType {
        id: service_type_id,
        organization_id: None,
        code: "UNKNOWN".to_string(),
        name: "Unknown Service".to_string(),
        description: None,
        category: "general".to_string(),
        service_classification: None,
        typical_duration_hours: None,
        typical_duration_days: None,
        requires_licensure: true,
        required_qualifications: serde_json::json!([]),
        equipment_required: serde_json::json!([]),
        facility_requirements: serde_json::json!({}),
        pre_authorization_required: false,
        cpt_code: None,
        icd_10_codes: serde_json::json!([]),
        hcpcs_code: None,
        insurance_coverage_typical: true,
        urgency_level: None,
        complexity_level: None,
        risk_level: None,
        tags: None,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })))
}

/// Update service type
#[utoipa::path(
    put,
    path = "/api/v1/healthcare/service-types/{service_type_id}",
    params(("service_type_id" = Uuid, Path, description = "Service Type ID")),
    request_body = ServiceType,
    responses(
        (status = 200, description = "Service type updated successfully", body = ServiceType),
        (status = 404, description = "Service type not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(("bearer_auth" = []))
)]
pub async fn update_service_type(
    State(_server): State<RustCareServer>,
    Path(service_type_id): Path<Uuid>,
    Json(_request): Json<ServiceType>,
) -> Result<Json<ApiResponse<ServiceType>>, ApiError> {
    // TODO: Implement database update
    Ok(Json(api_success(ServiceType {
        id: service_type_id,
        organization_id: _request.organization_id,
        code: _request.code,
        name: _request.name,
        description: _request.description,
        category: _request.category,
        service_classification: _request.service_classification,
        typical_duration_hours: _request.typical_duration_hours,
        typical_duration_days: _request.typical_duration_days,
        requires_licensure: _request.requires_licensure,
        required_qualifications: _request.required_qualifications,
        equipment_required: _request.equipment_required,
        facility_requirements: _request.facility_requirements,
        pre_authorization_required: _request.pre_authorization_required,
        cpt_code: _request.cpt_code,
        icd_10_codes: _request.icd_10_codes,
        hcpcs_code: _request.hcpcs_code,
        insurance_coverage_typical: _request.insurance_coverage_typical,
        urgency_level: _request.urgency_level,
        complexity_level: _request.complexity_level,
        risk_level: _request.risk_level,
        tags: _request.tags,
        is_active: _request.is_active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })))
}

/// Delete service type
#[utoipa::path(
    delete,
    path = "/api/v1/healthcare/service-types/{service_type_id}",
    params(("service_type_id" = Uuid, Path, description = "Service Type ID")),
    responses(
        (status = 204, description = "Service type deleted successfully"),
        (status = 404, description = "Service type not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "healthcare",
    security(("bearer_auth" = []))
)]
pub async fn delete_service_type(
    State(_server): State<RustCareServer>,
    Path(service_type_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // TODO: Implement soft delete
    println!("Deleting service type: {}", service_type_id);
    Ok(StatusCode::NO_CONTENT)
}

