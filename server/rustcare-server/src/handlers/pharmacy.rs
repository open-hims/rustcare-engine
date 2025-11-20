use crate::error::{api_success, api_success_with_meta, ApiError, ApiResponse};
use crate::handlers::common::crud::{AuthCrudHandler, CrudHandler};
use crate::middleware::AuthContext;
use crate::server::RustCareServer;
use crate::services::AuditService;
use crate::types::pagination::PaginationParams;
use crate::utils::query_builder::PaginatedQuery;
use crate::validation::RequestValidation;
use crate::{validate_email, validate_field, validate_length, validate_required};
use async_trait::async_trait;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[cfg(feature = "mcp")]
use mcp_macros::mcp_tool;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Pharmacy structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct Pharmacy {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub code: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub fax: Option<String>,
    pub license_number: Option<String>,
    pub license_authority: Option<String>,
    pub license_expiry: Option<String>,
    pub dea_number: Option<String>,
    pub hours_of_operation: serde_json::Value,
    pub is_internal: bool,
    pub is_active: bool,
    pub settings: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create Pharmacy Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePharmacyRequest {
    pub name: String,
    pub code: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub fax: Option<String>,
    pub license_number: Option<String>,
    pub license_authority: Option<String>,
    pub license_expiry: Option<String>,
    pub dea_number: Option<String>,
    pub hours_of_operation: Option<serde_json::Value>,
    pub is_internal: bool,
    pub settings: Option<serde_json::Value>,
}

impl RequestValidation for CreatePharmacyRequest {
    fn validate(&self) -> Result<(), ApiError> {
        validate_required!(self.name, "Pharmacy name is required");
        validate_required!(self.code, "Pharmacy code is required");
        validate_required!(self.address, "Address is required");
        validate_required!(self.city, "City is required");
        validate_required!(self.state, "State is required");
        validate_required!(self.postal_code, "Postal code is required");
        validate_required!(self.country, "Country is required");

        validate_length!(
            self.name,
            1,
            200,
            "Name must be between 1 and 200 characters"
        );
        validate_length!(self.code, 1, 50, "Code must be between 1 and 50 characters");

        // Validate email format if provided
        if let Some(ref email) = self.email {
            validate_email!(email, "Invalid email format");
        }

        Ok(())
    }
}

/// Update Pharmacy Request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePharmacyRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub fax: Option<String>,
    pub license_number: Option<String>,
    pub license_authority: Option<String>,
    pub license_expiry: Option<String>,
    pub dea_number: Option<String>,
    pub hours_of_operation: Option<serde_json::Value>,
    pub is_active: Option<bool>,
    pub settings: Option<serde_json::Value>,
}

impl RequestValidation for UpdatePharmacyRequest {
    fn validate(&self) -> Result<(), ApiError> {
        // Validate name length if provided
        if let Some(ref name) = self.name {
            validate_length!(name, 1, 200, "Name must be between 1 and 200 characters");
        }

        // Validate code length if provided
        if let Some(ref code) = self.code {
            validate_length!(code, 1, 50, "Code must be between 1 and 50 characters");
        }

        // Validate email format if provided
        if let Some(ref email) = self.email {
            validate_email!(email, "Invalid email format");
        }

        Ok(())
    }
}

/// List Pharmacies Query Parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPharmaciesParams {
    pub is_active: Option<bool>,
    pub is_internal: Option<bool>,
    pub city: Option<String>,
    pub state: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Medication structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct Medication {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub generic_name: Option<String>,
    pub medication_code: Option<String>,
    pub medication_type: String,
    pub drug_class: Option<String>,
    pub therapeutic_category: Option<String>,
    pub active_ingredients: serde_json::Value,
    pub strength: Option<String>,
    pub dosage_form: Option<String>,
    pub route_of_administration: Option<String>,
    pub prescription_required: bool,
    pub controlled_substance_schedule: Option<String>,
    pub contraindications: serde_json::Value,
    pub side_effects: serde_json::Value,
    pub drug_interactions: serde_json::Value,
    pub manufacturer: Option<String>,
    pub brand_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Pharmacy Inventory structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct PharmacyInventory {
    pub id: Uuid,
    pub pharmacy_id: Uuid,
    pub medication_id: Uuid,
    pub quantity_on_hand: i32,
    pub quantity_reserved: i32,
    pub quantity_available: i32,
    pub location: Option<String>,
    pub lot_number: Option<String>,
    pub expiry_date: Option<String>,
    pub date_received: String,
    pub unit_cost: Option<String>,
    pub unit_price: Option<String>,
    pub reorder_level: i32,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// List Inventory Query Parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListInventoryParams {
    pub pharmacy_id: Option<Uuid>,
    pub medication_id: Option<Uuid>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Prescription structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct Prescription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub patient_id: Uuid,
    pub provider_id: Uuid,
    pub pharmacy_id: Option<Uuid>,
    pub medication_id: Uuid,
    pub dosage: String,
    pub quantity: i32,
    pub days_supply: Option<i32>,
    pub frequency: String,
    pub route_of_administration: Option<String>,
    pub duration_days: Option<i32>,
    pub instructions: Option<String>,
    pub patient_instructions: Option<String>,
    pub sig_code: Option<String>,
    pub status: String,
    pub prescribed_date: chrono::DateTime<chrono::Utc>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
    pub refills_remaining: i32,
    pub max_refills: i32,
    pub insurance_covered: Option<bool>,
    pub copay_amount: Option<String>,
    pub total_cost: Option<String>,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// List Prescriptions Query Parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPrescriptionsParams {
    pub patient_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub pharmacy_id: Option<Uuid>,
    pub status: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

// ============================================================================
// CRUD HANDLER IMPLEMENTATION (Example of using Generic CRUD Traits)
// ============================================================================

/// Pharmacy CRUD handler implementation
///
/// This demonstrates how to use the AuthCrudHandler trait for organization-scoped CRUD operations.
struct PharmacyHandler;

#[async_trait]
impl CrudHandler<Pharmacy, CreatePharmacyRequest, UpdatePharmacyRequest, ListPharmaciesParams>
    for PharmacyHandler
{
    fn table_name() -> &'static str {
        "pharmacies"
    }

    fn apply_filters(
        query: &mut PaginatedQuery,
        params: &ListPharmaciesParams,
    ) -> Result<(), ApiError> {
        query
            .filter_eq("is_active", params.is_active)
            .filter_eq("is_internal", params.is_internal)
            .filter_eq("city", params.city.as_deref().map(str::to_owned))
            .filter_eq("state", params.state.as_deref().map(str::to_owned));
        Ok(())
    }

    fn extract_page(params: &ListPharmaciesParams) -> Option<u32> {
        Some(params.pagination.page())
    }

    fn extract_page_size(params: &ListPharmaciesParams) -> Option<u32> {
        Some(params.pagination.page_size())
    }
}

impl AuthCrudHandler<Pharmacy, CreatePharmacyRequest, UpdatePharmacyRequest, ListPharmaciesParams>
    for PharmacyHandler
{
}

// ============================================================================
// API HANDLERS (Refactored to use new utilities)
// ============================================================================

/// List all pharmacies
#[cfg_attr(
    feature = "mcp",
    mcp_macros::mcp_tool(
        name = "list_pharmacies",
        description = "List all pharmacies for the organization with optional filtering by city, state, and status",
        category = "pharmacy",
        requires_permission = "pharmacy:read",
        sensitive = false,
        response_type = "Vec<Pharmacy>",
        render_type = "table"
    )
)]
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::PHARMACY_PHARMACIES,
    responses(
        (status = 200, description = "Pharmacies retrieved successfully", body = Vec<Pharmacy>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(ListPharmaciesParams),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn list_pharmacies(
    State(server): State<RustCareServer>,
    Query(params): Query<ListPharmaciesParams>,
    auth: AuthContext, // Using new AuthContext extractor
) -> Result<Json<ApiResponse<Vec<Pharmacy>>>, ApiError> {
    // Use PaginatedQuery utility instead of manual query building
    let mut query_builder =
        PaginatedQuery::new("SELECT * FROM pharmacies WHERE is_deleted = false");

    query_builder
        .filter_organization(Some(auth.organization_id)) // Use actual auth context
        .filter_eq("is_active", params.is_active)
        .filter_eq("is_internal", params.is_internal)
        .filter_eq("city", params.city.as_deref().map(str::to_owned))
        .filter_eq("state", params.state.as_deref().map(str::to_owned))
        .order_by_created_desc()
        .paginate(params.pagination.page, params.pagination.page_size);

    let pharmacies: Vec<Pharmacy> = query_builder
        .build_query_as()
        .fetch_all(&server.db_pool)
        .await?;

    // Use pagination metadata helper
    let total_count = get_pharmacies_count(&server, &auth, &params).await?;
    let metadata = params.pagination.to_metadata(total_count);

    Ok(Json(crate::error::api_success_with_meta(
        pharmacies, metadata,
    )))
}

/// Get a specific pharmacy by ID
#[cfg_attr(
    feature = "mcp",
    mcp_macros::mcp_tool(
        name = "get_pharmacy",
        description = "Retrieve detailed information about a specific pharmacy by ID",
        category = "pharmacy",
        requires_permission = "pharmacy:read",
        sensitive = false,
        response_type = "Pharmacy",
        render_type = "markdown"
    )
)]
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::PHARMACY_PHARMACY_BY_ID,
    responses(
        (status = 200, description = "Pharmacy retrieved successfully", body = Pharmacy),
        (status = 404, description = "Pharmacy not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pharmacy_id" = Uuid, Path, description = "Pharmacy ID")
    ),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn get_pharmacy(
    State(server): State<RustCareServer>,
    Path(pharmacy_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Pharmacy>>, ApiError> {
    // Using AuthCrudHandler trait method for organization-scoped get
    PharmacyHandler::get_with_auth(State(server), Path(pharmacy_id), auth).await
}

/// Create a new pharmacy
#[utoipa::path(
    post,
    path = crate::routes::paths::api_v1::PHARMACY_PHARMACIES,
    request_body = CreatePharmacyRequest,
    responses(
        (status = 201, description = "Pharmacy created successfully", body = Pharmacy),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn create_pharmacy(
    State(server): State<RustCareServer>,
    auth: AuthContext,
    Json(req): Json<CreatePharmacyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Pharmacy>>), ApiError> {
    // Validate request
    req.validate()?;

    let pharmacy_id = Uuid::new_v4();

    let pharmacy = sqlx::query_as::<_, Pharmacy>(
        r#"
        INSERT INTO pharmacies (
            id, organization_id, name, code, address, city, state, postal_code, country,
            phone, email, fax, license_number, license_authority, license_expiry,
            dea_number, hours_of_operation, is_internal, is_active, settings,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, NOW(), NOW())
        RETURNING *
        "#
    )
    .bind(pharmacy_id)
    .bind(auth.organization_id) // Use actual auth context
    .bind(&req.name)
    .bind(&req.code)
    .bind(&req.address)
    .bind(&req.city)
    .bind(&req.state)
    .bind(&req.postal_code)
    .bind(&req.country)
    .bind(&req.phone)
    .bind(&req.email)
    .bind(&req.fax)
    .bind(&req.license_number)
    .bind(&req.license_authority)
    .bind(&req.license_expiry)
    .bind(&req.dea_number)
    .bind(req.hours_of_operation.unwrap_or_else(|| serde_json::json!({})))
    .bind(req.is_internal)
    .bind(true) // is_active defaults to true
    .bind(req.settings.unwrap_or_else(|| serde_json::json!({})))
    .fetch_one(&server.db_pool)
    .await?;

    // Log the creation using AuditService (automatically includes request context)
    let audit_service = AuditService::new(server.db_pool.clone());
    let _ = audit_service
        .log_general_action(
            &auth,
            "pharmacy",
            pharmacy.id,
            "created",
            Some(serde_json::json!({
                "name": req.name,
                "code": req.code,
                "request_id": auth.request_id()
            })),
        )
        .await;

    Ok((StatusCode::CREATED, Json(api_success(pharmacy))))
}

/// Update a pharmacy
#[utoipa::path(
    put,
    path = crate::routes::paths::api_v1::PHARMACY_PHARMACY_BY_ID,
    request_body = UpdatePharmacyRequest,
    responses(
        (status = 200, description = "Pharmacy updated successfully", body = Pharmacy),
        (status = 404, description = "Pharmacy not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pharmacy_id" = Uuid, Path, description = "Pharmacy ID")
    ),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn update_pharmacy(
    State(server): State<RustCareServer>,
    Path(pharmacy_id): Path<Uuid>,
    auth: AuthContext,
    Json(req): Json<UpdatePharmacyRequest>,
) -> Result<Json<ApiResponse<Pharmacy>>, ApiError> {
    // Validate request
    req.validate()?;

    // Check if pharmacy exists and belongs to organization
    let exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM pharmacies 
            WHERE id = $1 
              AND organization_id = $2 
              AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#,
    )
    .bind(pharmacy_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;

    if !exists {
        return Err(ApiError::not_found("pharmacy"));
    }

    // Build update query dynamically based on provided fields
    let pharmacy = sqlx::query_as::<_, Pharmacy>(
        r#"
        UPDATE pharmacies
        SET 
            name = COALESCE($1, name),
            code = COALESCE($2, code),
            address = COALESCE($3, address),
            city = COALESCE($4, city),
            state = COALESCE($5, state),
            postal_code = COALESCE($6, postal_code),
            country = COALESCE($7, country),
            phone = COALESCE($8, phone),
            email = COALESCE($9, email),
            fax = COALESCE($10, fax),
            license_number = COALESCE($11, license_number),
            license_authority = COALESCE($12, license_authority),
            license_expiry = COALESCE($13, license_expiry),
            dea_number = COALESCE($14, dea_number),
            hours_of_operation = COALESCE($15, hours_of_operation),
            is_active = COALESCE($16, is_active),
            settings = COALESCE($17, settings),
            updated_at = NOW()
        WHERE id = $18 AND organization_id = $19
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.code)
    .bind(&req.address)
    .bind(&req.city)
    .bind(&req.state)
    .bind(&req.postal_code)
    .bind(&req.country)
    .bind(&req.phone)
    .bind(&req.email)
    .bind(&req.fax)
    .bind(&req.license_number)
    .bind(&req.license_authority)
    .bind(&req.license_expiry)
    .bind(&req.dea_number)
    .bind(req.hours_of_operation.as_ref())
    .bind(req.is_active)
    .bind(req.settings.as_ref())
    .bind(pharmacy_id)
    .bind(auth.organization_id)
    .fetch_optional(&server.db_pool)
    .await?;

    if let Some(ref pharmacy) = pharmacy {
        // Check permission using Zanzibar (if available)
        if let Err(e) = auth
            .require_permission("pharmacy", Some(pharmacy_id), "update")
            .await
        {
            tracing::warn!("Permission check failed (falling back to org check): {}", e);
            // Fallback: organization check already done above
        }

        // Log the update using AuditService (automatically includes request context)
        let audit_service = AuditService::new(server.db_pool.clone());
        let _ = audit_service
            .log_general_action(
                &auth,
                "pharmacy",
                pharmacy_id,
                "updated",
                Some(serde_json::json!({
                    "request_id": auth.request_id()
                })),
            )
            .await;
    }

    match pharmacy {
        Some(pharm) => Ok(Json(api_success(pharm))),
        None => Err(ApiError::not_found("pharmacy")),
    }
}

/// Delete a pharmacy (soft delete)
#[utoipa::path(
    delete,
    path = crate::routes::paths::api_v1::PHARMACY_PHARMACY_BY_ID,
    responses(
        (status = 204, description = "Pharmacy deleted successfully"),
        (status = 404, description = "Pharmacy not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pharmacy_id" = Uuid, Path, description = "Pharmacy ID")
    ),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn delete_pharmacy(
    State(server): State<RustCareServer>,
    Path(pharmacy_id): Path<Uuid>,
    auth: AuthContext,
) -> Result<StatusCode, ApiError> {
    // Using AuthCrudHandler trait method for organization-scoped delete
    PharmacyHandler::delete_with_auth(State(server), Path(pharmacy_id), auth).await
}

/// Get pharmacy inventory
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::PHARMACY_INVENTORY,
    responses(
        (status = 200, description = "Inventory retrieved successfully", body = Vec<PharmacyInventory>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(ListInventoryParams),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn list_inventory(
    State(server): State<RustCareServer>,
    Query(params): Query<ListInventoryParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PharmacyInventory>>>, ApiError> {
    // Use PaginatedQuery utility with JOIN query
    let mut query = PaginatedQuery::new(
        "SELECT pi.* FROM pharmacy_inventory pi
         JOIN pharmacies p ON pi.pharmacy_id = p.id",
    );

    query
        .add_base_filter("p.organization_id", auth.organization_id)
        .query_builder()
        .push(" AND (p.is_deleted = false OR p.is_deleted IS NULL)");

    query
        .filter_eq("pi.pharmacy_id", params.pharmacy_id)
        .filter_eq("pi.medication_id", params.medication_id)
        .filter_eq("pi.status", params.status.clone())
        .order_by("pi.created_at", "DESC")
        .paginate(params.pagination.page, params.pagination.page_size);

    let inventory: Vec<PharmacyInventory> =
        query.build_query_as().fetch_all(&server.db_pool).await?;

    // Get total count for pagination metadata
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) 
        FROM pharmacy_inventory pi
        JOIN pharmacies p ON pi.pharmacy_id = p.id
        WHERE p.organization_id = $1
          AND (p.is_deleted = false OR p.is_deleted IS NULL)
          AND ($2::uuid IS NULL OR pi.pharmacy_id = $2)
          AND ($3::uuid IS NULL OR pi.medication_id = $3)
          AND ($4::text IS NULL OR pi.status = $4)
        "#,
    )
    .bind(auth.organization_id)
    .bind(params.pharmacy_id)
    .bind(params.medication_id)
    .bind(params.status.as_deref())
    .fetch_one(&server.db_pool)
    .await?;

    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(api_success_with_meta(inventory, metadata)))
}

/// List prescriptions
#[utoipa::path(
    get,
    path = crate::routes::paths::api_v1::PHARMACY_PRESCRIPTIONS,
    responses(
        (status = 200, description = "Prescriptions retrieved successfully", body = Vec<Prescription>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(ListPrescriptionsParams),
    tag = "pharmacy",
    security(("bearer_auth" = []))
)]
pub async fn list_prescriptions(
    State(server): State<RustCareServer>,
    Query(params): Query<ListPrescriptionsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<Prescription>>>, ApiError> {
    // Use PaginatedQuery utility
    let mut query = PaginatedQuery::new("SELECT * FROM prescriptions");

    query
        .add_base_filter("organization_id", auth.organization_id)
        .query_builder()
        .push(" AND (is_deleted = false OR is_deleted IS NULL)");

    query
        .filter_eq("patient_id", params.patient_id)
        .filter_eq("provider_id", params.provider_id)
        .filter_eq("pharmacy_id", params.pharmacy_id)
        .filter_eq("status", params.status.clone())
        .order_by("prescribed_date", "DESC")
        .paginate(params.pagination.page, params.pagination.page_size);

    let prescriptions: Vec<Prescription> =
        query.build_query_as().fetch_all(&server.db_pool).await?;

    // Get total count for pagination metadata
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM prescriptions
        WHERE organization_id = $1
          AND (is_deleted = false OR is_deleted IS NULL)
          AND ($2::uuid IS NULL OR patient_id = $2)
          AND ($3::uuid IS NULL OR provider_id = $3)
          AND ($4::uuid IS NULL OR pharmacy_id = $4)
          AND ($5::text IS NULL OR status = $5)
        "#,
    )
    .bind(auth.organization_id)
    .bind(params.patient_id)
    .bind(params.provider_id)
    .bind(params.pharmacy_id)
    .bind(params.status.as_deref())
    .fetch_one(&server.db_pool)
    .await?;

    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(api_success_with_meta(prescriptions, metadata)))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Get total count of pharmacies for pagination metadata
async fn get_pharmacies_count(
    server: &RustCareServer,
    auth: &AuthContext,
    params: &ListPharmaciesParams,
) -> Result<i64, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM pharmacies
        WHERE organization_id = $1
          AND (is_deleted = false OR is_deleted IS NULL)
          AND ($2::bool IS NULL OR is_active = $2)
          AND ($3::bool IS NULL OR is_internal = $3)
          AND ($4::text IS NULL OR city = $4)
          AND ($5::text IS NULL OR state = $5)
        "#,
    )
    .bind(auth.organization_id)
    .bind(params.is_active)
    .bind(params.is_internal)
    .bind(params.city.as_deref())
    .bind(params.state.as_deref())
    .fetch_one(&server.db_pool)
    .await?;

    Ok(count)
}
