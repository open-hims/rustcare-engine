use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use crate::server::RustCareServer;
use crate::error::{ApiError, ApiResponse, api_success};
use crate::utils::query_builder::PaginatedQuery;
use crate::types::pagination::PaginationParams;
use crate::middleware::AuthContext;
use crate::validation::RequestValidation;
use crate::services::AuditService;
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use std::collections::HashMap;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Vendor Type structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct VendorType {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub is_active: bool,
    pub metadata: serde_json::Value,
}

/// Vendor structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct Vendor {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub vendor_type_id: Uuid,
    pub name: String,
    pub code: String,
    pub tax_id: Option<String>,
    pub vat_number: Option<String>,
    pub address: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub legal_entity_type: Option<String>,
    pub payment_terms: Option<String>,
    pub credit_limit: Option<String>,
    pub quality_rating: Option<String>,
    pub is_preferred_vendor: bool,
    pub is_active: bool,
    pub contract_start_date: Option<String>,
    pub contract_end_date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Vendor Inventory structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct VendorInventory {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub item_code: String,
    pub item_name: String,
    pub description: Option<String>,
    pub item_category: String,
    pub unit_of_measure: String,
    pub unit_price: String,
    pub bulk_price: Option<String>,
    pub minimum_order_quantity: i32,
    pub in_stock: bool,
    pub lead_time_days: Option<i32>,
    pub stock_quantity: Option<i32>,
    pub manufacturer: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub specifications: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Vendor Service structure
#[derive(Debug, Serialize, Deserialize, ToSchema, FromRow, Clone)]
pub struct VendorService {
    pub id: Uuid,
    pub vendor_id: Uuid,
    pub service_code: String,
    pub service_name: String,
    pub description: Option<String>,
    pub service_category: String,
    pub service_type: String,
    pub duration_hours: Option<String>,
    pub service_location: String,
    pub pricing_model: String,
    pub base_price: Option<String>,
    pub hourly_rate: Option<String>,
    pub is_available: bool,
    pub requires_appointment: bool,
    pub turnaround_time: Option<String>,
    pub is_active: bool,
    pub tags: Option<Vec<String>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// QUERY PARAMETERS
// ============================================================================

/// List Vendor Types Query Parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListVendorTypesParams {
    pub category: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// List Vendors Query Parameters
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListVendorsParams {
    pub vendor_type_id: Option<Uuid>,
    pub is_preferred_vendor: Option<bool>,
    pub is_active: Option<bool>,
    pub city: Option<String>,
    pub state: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

// ============================================================================
// API HANDLERS (Refactored to use new utilities)
// ============================================================================

/// List vendor types
#[utoipa::path(
    get,
    path = "/api/v1/vendors/types",
    responses(
        (status = 200, description = "Vendor types retrieved successfully", body = Vec<VendorType>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(ListVendorTypesParams),
    tag = "vendors",
    security(("bearer_auth" = []))
)]
pub async fn list_vendor_types(
    State(server): State<RustCareServer>,
    Query(params): Query<ListVendorTypesParams>,
    _auth: AuthContext, // Using AuthContext for consistency, even though vendor types are global
) -> Result<Json<ApiResponse<Vec<VendorType>>>, ApiError> {
    // Use PaginatedQuery utility
    let mut query_builder = PaginatedQuery::new(
        "SELECT * FROM vendor_types WHERE is_active = true"
    );
    
    query_builder
        .filter_eq("category", params.category.as_ref().map(|s| s.as_str()))
        .order_by("name", "ASC")
        .paginate(
            params.pagination.page,
            params.pagination.page_size
        );
    
    let vendor_types: Vec<VendorType> = query_builder.build_query_as().fetch_all(&server.db_pool).await?;
    
    // Get total count for pagination metadata
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM vendor_types
        WHERE is_active = true
          AND ($1::text IS NULL OR category = $1)
        "#
    )
    .bind(params.category.as_deref())
    .fetch_one(&server.db_pool)
    .await?;
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(vendor_types, metadata)))
}

/// List vendors
#[utoipa::path(
    get,
    path = "/api/v1/vendors",
    responses(
        (status = 200, description = "Vendors retrieved successfully", body = Vec<Vendor>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    params(ListVendorsParams),
    tag = "vendors",
    security(("bearer_auth" = []))
)]
pub async fn list_vendors(
    State(server): State<RustCareServer>,
    Query(params): Query<ListVendorsParams>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<Vendor>>>, ApiError> {
    // Use PaginatedQuery utility
    let mut query_builder = PaginatedQuery::new(
        "SELECT * FROM vendors WHERE is_deleted = false"
    );
    
    query_builder
        .filter_organization(Some(auth.organization_id)) // Use actual auth context
        .filter_eq("vendor_type_id", params.vendor_type_id)
        .filter_eq("is_preferred_vendor", params.is_preferred_vendor)
        .filter_eq("is_active", params.is_active)
        .filter_eq("city", params.city.as_ref().map(|s| s.as_str()))
        .filter_eq("state", params.state.as_ref().map(|s| s.as_str()))
        .order_by_created_desc()
        .paginate(
            params.pagination.page,
            params.pagination.page_size
        );
    
    let vendors: Vec<Vendor> = query_builder.build_query_as().fetch_all(&server.db_pool).await?;
    
    // Get total count for pagination metadata
    let total_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM vendors
        WHERE organization_id = $1
          AND (is_deleted = false OR is_deleted IS NULL)
          AND ($2::uuid IS NULL OR vendor_type_id = $2)
          AND ($3::bool IS NULL OR is_preferred_vendor = $3)
          AND ($4::bool IS NULL OR is_active = $4)
          AND ($5::text IS NULL OR city = $5)
          AND ($6::text IS NULL OR state = $6)
        "#
    )
    .bind(auth.organization_id)
    .bind(params.vendor_type_id)
    .bind(params.is_preferred_vendor)
    .bind(params.is_active)
    .bind(params.city.as_deref())
    .bind(params.state.as_deref())
    .fetch_one(&server.db_pool)
    .await?;
    
    let metadata = params.pagination.to_metadata(total_count);
    Ok(Json(crate::error::api_success_with_meta(vendors, metadata)))
}

/// Get vendor inventory
#[utoipa::path(
    get,
    path = "/api/v1/vendors/{vendor_id}/inventory",
    params(
        ("vendor_id" = Uuid, Path, description = "Vendor ID"),
        ("is_active" = Option<bool>, Query, description = "Filter by active status"),
        ("in_stock" = Option<bool>, Query, description = "Filter by stock availability")
    ),
    responses(
        (status = 200, description = "Vendor inventory retrieved successfully", body = Vec<VendorInventory>),
        (status = 404, description = "Vendor not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "vendors",
    security(("bearer_auth" = []))
)]
pub async fn get_vendor_inventory(
    State(server): State<RustCareServer>,
    Path(vendor_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<VendorInventory>>>, ApiError> {
    // Verify vendor exists and belongs to organization
    let vendor_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM vendors 
            WHERE id = $1 
              AND organization_id = $2 
              AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(vendor_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !vendor_exists {
        return Err(ApiError::not_found("vendor"));
    }
    
    // Parse query parameters
    let is_active = params.get("is_active").and_then(|v| v.parse().ok());
    let in_stock = params.get("in_stock").and_then(|v| v.parse().ok());
    
    // Use PaginatedQuery utility
    let mut query_builder = PaginatedQuery::new(
        "SELECT * FROM vendor_inventory WHERE vendor_id = $1"
    );
    
    // Add vendor_id as a bound parameter (PaginatedQuery needs to support this better)
    // For now, use direct query with proper filtering
    let inventory = sqlx::query_as::<_, VendorInventory>(
        r#"
        SELECT vi.* 
        FROM vendor_inventory vi
        JOIN vendors v ON vi.vendor_id = v.id
        WHERE vi.vendor_id = $1
          AND v.organization_id = $2
          AND ($3::bool IS NULL OR vi.is_active = $3)
          AND ($4::bool IS NULL OR vi.in_stock = $4)
          AND (v.is_deleted = false OR v.is_deleted IS NULL)
        ORDER BY vi.created_at DESC
        "#
    )
    .bind(vendor_id)
    .bind(auth.organization_id)
    .bind(is_active)
    .bind(in_stock)
    .fetch_all(&server.db_pool)
    .await?;
    
    Ok(Json(api_success(inventory)))
}

/// Get vendor services
#[utoipa::path(
    get,
    path = "/api/v1/vendors/{vendor_id}/services",
    params(
        ("vendor_id" = Uuid, Path, description = "Vendor ID"),
        ("is_available" = Option<bool>, Query, description = "Filter by availability"),
        ("is_active" = Option<bool>, Query, description = "Filter by active status")
    ),
    responses(
        (status = 200, description = "Vendor services retrieved successfully", body = Vec<VendorService>),
        (status = 404, description = "Vendor not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "vendors",
    security(("bearer_auth" = []))
)]
pub async fn get_vendor_services(
    State(server): State<RustCareServer>,
    Path(vendor_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<VendorService>>>, ApiError> {
    // Verify vendor exists and belongs to organization
    let vendor_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM vendors 
            WHERE id = $1 
              AND organization_id = $2 
              AND (is_deleted = false OR is_deleted IS NULL)
        )
        "#
    )
    .bind(vendor_id)
    .bind(auth.organization_id)
    .fetch_one(&server.db_pool)
    .await?;
    
    if !vendor_exists {
        return Err(ApiError::not_found("vendor"));
    }
    
    // Parse query parameters
    let is_available = params.get("is_available").and_then(|v| v.parse().ok());
    let is_active = params.get("is_active").and_then(|v| v.parse().ok());
    
    // Query vendor services
    let services = sqlx::query_as::<_, VendorService>(
        r#"
        SELECT vs.* 
        FROM vendor_services vs
        JOIN vendors v ON vs.vendor_id = v.id
        WHERE vs.vendor_id = $1
          AND v.organization_id = $2
          AND ($3::bool IS NULL OR vs.is_available = $3)
          AND ($4::bool IS NULL OR vs.is_active = $4)
          AND (v.is_deleted = false OR v.is_deleted IS NULL)
        ORDER BY vs.created_at DESC
        "#
    )
    .bind(vendor_id)
    .bind(auth.organization_id)
    .bind(is_available)
    .bind(is_active)
    .fetch_all(&server.db_pool)
    .await?;
    
    Ok(Json(api_success(services)))
}

