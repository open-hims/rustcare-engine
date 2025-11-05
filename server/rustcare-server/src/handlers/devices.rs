use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use utoipa::{ToSchema, IntoParams};

use crate::{
    error::{ApiError, ApiResponse, api_success},
    server::RustCareServer,
    middleware::AuthContext,
    types::pagination::PaginationParams,
    utils::query_builder::PaginatedQuery,
    validation::RequestValidation,
    services::AuditService,
};

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterDeviceRequest {
    pub name: String,
    pub device_type: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub location: serde_json::Value,
    pub config: serde_json::Value,
}

impl RequestValidation for RegisterDeviceRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.name, "Device name is required");
        validate_required!(self.device_type, "Device type is required");
        validate_required!(self.manufacturer, "Manufacturer is required");
        validate_required!(self.model, "Model is required");
        validate_required!(self.serial_number, "Serial number is required");
        
        validate_length!(self.name, 1, 200, "Name must be between 1 and 200 characters");
        validate_length!(self.serial_number, 1, 100, "Serial number must be between 1 and 100 characters");
        
        Ok(())
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateDeviceRequest {
    pub name: Option<String>,
    pub device_type: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub location: Option<serde_json::Value>,
    pub config: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

impl RequestValidation for UpdateDeviceRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if let Some(ref name) = self.name {
            validate_length!(name, 1, 200, "Name must be between 1 and 200 characters");
        }
        if let Some(ref serial_number) = self.serial_number {
            validate_length!(serial_number, 1, 100, "Serial number must be between 1 and 100 characters");
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDevicesQuery {
    pub device_type: Option<String>,
    pub status: Option<String>,
    pub location: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendCommandRequest {
    pub command: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct GetDataQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub data_type: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceResponse {
    pub id: Uuid,
    pub name: String,
    pub device_type: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub location: serde_json::Value,
    pub status: String,
    pub last_connected: Option<DateTime<Utc>>,
    pub last_data_received: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub config: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceListResponse {
    pub devices: Vec<DeviceResponse>,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeviceDataResponse {
    pub id: Uuid,
    pub device_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub data_type: String,
    pub format: String,
    pub raw_data: serde_json::Value,
    pub parsed_data: Option<serde_json::Value>,
    pub normalized_data: Option<serde_json::Value>,
    pub patient_id: Option<Uuid>,
    pub encounter_id: Option<Uuid>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommandResponse {
    pub id: Uuid,
    pub device_id: Uuid,
    pub command: String,
    pub parameters: serde_json::Value,
    pub status: String,
    pub response: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// List all devices with optional filters
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICES,
    params(ListDevicesQuery),
    responses(
        (status = 200, description = "Devices retrieved successfully", body = Vec<DeviceResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn list_devices(
    State(server): State<RustCareServer>,
    Query(query): Query<ListDevicesQuery>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<DeviceResponse>>>, ApiError> {
    // Use PaginatedQuery utility
    let mut query_builder = PaginatedQuery::new(
        "SELECT * FROM devices WHERE organization_id = $1 AND (is_deleted = false OR is_deleted IS NULL)"
    );
    
    query_builder
        .filter_eq("device_type", query.device_type.as_ref().map(|s| s.as_str()))
        .filter_eq("status", query.status.as_ref().map(|s| s.as_str()))
        .order_by("created_at", "DESC")
        .paginate(query.pagination.page, query.pagination.page_size);
    
    // For now, return empty until device manager is implemented
    // TODO: Implement actual device query
    let devices: Vec<DeviceResponse> = vec![];
    
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM devices
        WHERE organization_id = $1
          AND (is_deleted = false OR is_deleted IS NULL)
          AND ($2::text IS NULL OR device_type = $2)
          AND ($3::text IS NULL OR status = $3)
        "#
    )
    .bind(auth.organization_id)
    .bind(query.device_type.as_deref())
    .bind(query.status.as_deref())
    .fetch_one(&server.db_pool)
    .await
    .unwrap_or(0);
    
    // Use standard pagination metadata format
    let metadata = query.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(devices, metadata)))
}

/// Register a new device
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::DEVICES,
    request_body = RegisterDeviceRequest,
    responses(
        (status = 201, description = "Device registered successfully", body = DeviceResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Device already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn register_device(
    State(server): State<RustCareServer>,
    Json(req): Json<RegisterDeviceRequest>,
    auth: AuthContext,
) -> Result<(StatusCode, Json<ApiResponse<DeviceResponse>>), ApiError> {
    // Validate request
    req.validate()?;
    
    // TODO: Implement with device manager
    // let config: DeviceConfig = serde_json::from_value(req.config)?;
    // let device = server.device_manager
    //     .register_device(
    //         req.name,
    //         req.device_type,
    //         req.manufacturer,
    //         req.model,
    //         req.serial_number,
    //         req.location,
    //         config,
    //         None, // user_id from auth
    //     )
    //     .await?;

    // TODO: Implement with device manager
    // For now, create a placeholder device record in database
    let device_id = Uuid::new_v4();
    
    let device = sqlx::query_as::<_, DeviceResponse>(
        r#"
        INSERT INTO devices (
            id, organization_id, name, device_type, manufacturer, model,
            serial_number, location, status, config, metadata,
            created_at, updated_at, created_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, 'registered', $9, $10, NOW(), NOW(), $11
        ) RETURNING
            id, name, device_type, manufacturer, model, serial_number,
            location, status, last_connected, last_data_received, last_error,
            config, metadata, created_at, updated_at
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .bind(&req.name)
    .bind(&req.device_type)
    .bind(&req.manufacturer)
    .bind(&req.model)
    .bind(&req.serial_number)
    .bind(&req.location)
    .bind(&req.config)
    .bind(serde_json::json!({}))
    .bind(auth.user_id)
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to register device: {}", e)))?;
    
    match device {
        Some(d) => {
            // Log the registration using AuditService
            let audit_service = AuditService::new(server.db_pool.clone());
            let _ = audit_service.log_general_action(
                &auth,
                "device",
                d.id,
                "registered",
                Some(serde_json::json!({"name": req.name, "type": req.device_type})),
            ).await;
            
            Ok((StatusCode::CREATED, Json(api_success(d))))
        },
        None => {
            // Fallback if table doesn't exist yet
            let response = DeviceResponse {
                id: device_id,
                name: req.name,
                device_type: req.device_type,
                manufacturer: req.manufacturer,
                model: req.model,
                serial_number: req.serial_number,
                location: req.location,
                status: "registered".to_string(),
                last_connected: None,
                last_data_received: None,
                last_error: None,
                config: req.config,
                metadata: serde_json::json!({}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            
            // Log the registration using AuditService
            let audit_service = AuditService::new(server.db_pool.clone());
            let _ = audit_service.log_general_action(
                &auth,
                "device",
                device_id,
                "registered",
                Some(serde_json::json!({"name": req.name, "type": req.device_type})),
            ).await;
            
            Ok((StatusCode::CREATED, Json(api_success(response))))
        }
    }
}

/// Get device by ID
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_BY_ID,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    responses(
        (status = 200, description = "Device retrieved successfully", body = DeviceResponse),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn get_device(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<DeviceResponse>>, ApiError> {
    let device = sqlx::query_as::<_, DeviceResponse>(
        r#"
        SELECT * FROM devices
        WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_optional(&server.db_pool)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch device: {}", e)))?;
    
    match device {
        Some(d) => Ok(Json(api_success(d))),
        None => Err(ApiError::not_found("device")),
    }
}

/// Update device
#[utoipa::path(
    put,
    path = crate::routes::paths::api_v1::DEVICE_BY_ID,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    request_body = UpdateDeviceRequest,
    responses(
        (status = 200, description = "Device updated successfully", body = DeviceResponse),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn update_device(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    Json(req): Json<UpdateDeviceRequest>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<DeviceResponse>>, ApiError> {
    // Validate request
    req.validate()?;
    
    // Check if device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement actual update query
    // For now, return error indicating not implemented
    Err(ApiError::internal("Device update not yet fully implemented"))
}

/// Delete device
#[utoipa::path(
    delete,
    path = crate::routes::paths::api_v1::DEVICE_BY_ID,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    responses(
        (status = 204, description = "Device deleted successfully"),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn delete_device(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE devices
        SET is_deleted = true, updated_at = NOW()
        WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .execute(&server.db_pool)
    .await?
    .rows_affected();
    
    if rows_affected == 0 {
        Err(ApiError::not_found("device"))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

/// Connect to device
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::DEVICE_CONNECT,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    responses(
        (status = 200, description = "Device connection initiated", body = serde_json::Value),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn connect_device(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Verify device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement with device manager
    Ok(Json(api_success(serde_json::json!({
        "success": true,
        "message": format!("Connection initiated for device {}", device_id)
    }))))
}

/// Disconnect from device
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::DEVICE_DISCONNECT,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    responses(
        (status = 200, description = "Device disconnection initiated", body = serde_json::Value),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn disconnect_device(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // Verify device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement with device manager
    Ok(Json(api_success(serde_json::json!({
        "success": true,
        "message": format!("Disconnection initiated for device {}", device_id)
    }))))
}

/// Read data from device
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_DATA,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    responses(
        (status = 200, description = "Device data retrieved successfully", body = DeviceDataResponse),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn read_device_data(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<DeviceDataResponse>>, ApiError> {
    // Verify device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement with device manager
    Err(ApiError::internal("Device data reading not yet implemented"))
}

/// Get device data history
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_DATA,
    params(
        ("device_id" = Uuid, Path, description = "Device ID"),
        GetDataQuery
    ),
    responses(
        (status = 200, description = "Device data history retrieved successfully", body = Vec<DeviceDataResponse>),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn get_device_data_history(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    Query(query): Query<GetDataQuery>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<DeviceDataResponse>>>, ApiError> {
    // Verify device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement with device manager
    Ok(Json(api_success(vec![])))
}

/// Send command to device
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::DEVICE_COMMANDS,
    params(
        ("device_id" = Uuid, Path, description = "Device ID")
    ),
    request_body = SendCommandRequest,
    responses(
        (status = 201, description = "Command sent successfully", body = CommandResponse),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn send_device_command(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    Json(req): Json<SendCommandRequest>,
    auth: AuthContext,
) -> Result<(StatusCode, Json<ApiResponse<CommandResponse>>), ApiError> {
    // Verify device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement with device manager
    let response = CommandResponse {
        id: Uuid::new_v4(),
        device_id,
        command: req.command,
        parameters: req.parameters,
        status: "pending".to_string(),
        response: None,
        error: None,
        created_at: Utc::now(),
        executed_at: None,
        completed_at: None,
    };

    Ok((StatusCode::CREATED, Json(api_success(response))))
}

/// Get device commands
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_COMMANDS,
    params(
        ("device_id" = Uuid, Path, description = "Device ID"),
        GetDataQuery
    ),
    responses(
        (status = 200, description = "Device commands retrieved successfully", body = Vec<CommandResponse>),
        (status = 404, description = "Device not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn get_device_commands(
    State(server): State<RustCareServer>,
    Path(device_id): Path<Uuid>,
    Query(_query): Query<GetDataQuery>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<CommandResponse>>>, ApiError> {
    // Verify device exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM devices 
            WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(device_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !exists {
        return Err(ApiError::not_found("device"));
    }
    
    // TODO: Implement with device manager
    Ok(Json(api_success(vec![])))
}

/// Get device types (configuration)
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_TYPES,
    responses(
        (status = 200, description = "Device types retrieved successfully", body = Vec<serde_json::Value>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn list_device_types(
    State(_server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    // TODO: Fetch from database configuration
    Ok(Json(api_success(vec![
        serde_json::json!({
            "code": "vitals_monitor",
            "name": "Vitals Monitor",
            "category": "monitoring",
            "description": "Patient vital signs monitoring device"
        }),
        serde_json::json!({
            "code": "lab_analyzer",
            "name": "Laboratory Analyzer",
            "category": "laboratory",
            "description": "Clinical laboratory analysis device"
        }),
    ])))
}

/// Get connection types (configuration)
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_CONNECTION_TYPES,
    responses(
        (status = 200, description = "Connection types retrieved successfully", body = Vec<serde_json::Value>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn list_connection_types(
    State(_server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    // TODO: Fetch from database configuration
    Ok(Json(api_success(vec![
        serde_json::json!({
            "code": "serial",
            "name": "Serial Port",
            "protocol": "RS232",
            "requires_auth": false
        }),
        serde_json::json!({
            "code": "network",
            "name": "Network (TCP/IP)",
            "protocol": "TCP",
            "default_port": 8080,
            "requires_auth": true
        }),
    ])))
}

/// Get data formats (configuration)
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::DEVICE_DATA_FORMATS,
    responses(
        (status = 200, description = "Data formats retrieved successfully", body = Vec<serde_json::Value>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "devices",
    security(("bearer_auth" = []))
)]
pub async fn list_data_formats(
    State(_server): State<RustCareServer>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    // TODO: Fetch from database configuration
    Ok(Json(api_success(vec![
        serde_json::json!({
            "code": "hl7_v2",
            "name": "HL7 v2",
            "version": "2.5.1",
            "mime_type": "x-application/hl7-v2+er7",
            "parser_plugin": "hl7v2_parser"
        }),
        serde_json::json!({
            "code": "fhir_r4",
            "name": "FHIR R4",
            "version": "4.0.1",
            "mime_type": "application/fhir+json",
            "parser_plugin": "fhir_r4_parser"
        }),
    ])))
}
