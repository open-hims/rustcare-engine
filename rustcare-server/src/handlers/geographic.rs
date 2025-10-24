use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    response::{IntoResponse, Json as ResponseJson},
};
use database_layer::GeographicRepository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use utoipa::{ToSchema, IntoParams};
use crate::server::RustCareServer;

/// Geographic region with hierarchical structure
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GeographicRegion {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub region_type: String, // country, state, city, etc.
    pub parent_region_id: Option<Uuid>,
    pub path: Option<String>, // materialized path
    pub level: i32,
    pub iso_country_code: Option<String>,
    pub iso_subdivision_code: Option<String>,
    pub timezone: Option<String>,
    pub population: Option<i64>,
    pub area_sq_km: Option<f64>,
    pub is_active: bool,
    pub metadata: serde_json::Value,
}

/// Request to create/update geographic region
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGeographicRegionRequest {
    pub code: String,
    pub name: String,
    pub region_type: String,
    pub parent_region_id: Option<Uuid>,
    pub iso_country_code: Option<String>,
    pub iso_subdivision_code: Option<String>,
    pub timezone: Option<String>,
    pub population: Option<i64>,
    pub area_sq_km: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

/// Alias for create request (used in routes)
pub type CreateRegionRequest = CreateGeographicRegionRequest;

/// Alias for update request (used in routes) 
pub type UpdateRegionRequest = CreateGeographicRegionRequest;

/// Query parameters for geographic region search
#[derive(Debug, Deserialize, IntoParams)]
pub struct GeographicQuery {
    #[param(style = Form, example = "country")]
    pub region_type: Option<String>,
    #[param(style = Form, example = "United States")]
    pub search: Option<String>,
    #[param(style = Form, example = 10)]
    pub limit: Option<i32>,
    #[param(style = Form, example = 0)]
    pub offset: Option<i32>,
}

/// Postal code mapping for compliance auto-assignment
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PostalCodeMapping {
    pub id: Uuid,
    pub postal_code: String,
    pub region_id: Uuid,
    pub country_code: String,
    pub compliance_frameworks: Vec<String>,
    pub regulatory_authorities: Vec<String>,
}

/// Get all geographic regions with optional filtering
#[utoipa::path(
    get,
    path = "/api/v1/geographic/regions",
    params(GeographicQuery),
    responses(
        (status = 200, description = "Geographic regions retrieved successfully", body = Vec<GeographicRegion>),
        (status = 400, description = "Invalid query parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_geographic_regions(
    State(server): State<RustCareServer>,
    Query(query): Query<GeographicQuery>,
) -> Result<ResponseJson<Vec<GeographicRegion>>, StatusCode> {
    // Use the database repository to fetch regions
    let db_regions = server.geographic_repo
        .list_regions(
            None, // parent_id
            query.region_type.as_deref(),
            query.search.as_deref(),
            query.limit,
            query.offset,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert database models to API models
    let regions: Vec<GeographicRegion> = db_regions
        .into_iter()
        .map(|db_region| GeographicRegion {
            id: db_region.id,
            code: db_region.code,
            name: db_region.name,
            region_type: db_region.region_type,
            parent_region_id: db_region.parent_region_id,
            path: db_region.path,
            level: 0, // TODO: Calculate from path
            iso_country_code: db_region.iso_country_code,
            iso_subdivision_code: db_region.iso_subdivision_code,
            timezone: db_region.timezone,
            population: None, // TODO: Add to database model
            area_sq_km: None, // TODO: Add to database model
            is_active: db_region.is_active,
            metadata: db_region.metadata.unwrap_or(serde_json::json!({})),
        })
        .collect();

    Ok(Json(regions))
}

/// Create a new geographic region
#[utoipa::path(
    post,
    path = "/api/v1/geographic/regions",
    request_body = CreateGeographicRegionRequest,
    responses(
        (status = 201, description = "Geographic region created successfully", body = GeographicRegion),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Region code already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_geographic_region(
    State(server): State<RustCareServer>,
    Json(payload): Json<CreateGeographicRegionRequest>,
) -> impl IntoResponse {
    match server.geographic_repo
        .create_region(
            &payload.name,
            &payload.code,
            &payload.region_type,
            payload.parent_region_id,
            payload.iso_country_code.as_deref(),
            payload.iso_subdivision_code.as_deref(),
            payload.timezone.as_deref(),
            None, // population
            None, // area_sq_km
            payload.metadata,
        )
        .await
    {
        Ok(region) => {
            tracing::info!("Geographic region created: {} - {}", region.code, region.name);
            (StatusCode::CREATED, Json(region)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create geographic region: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(format!("Failed to create region: {}", e)),
            )
                .into_response()
        }
    }
}/// Get geographic region by ID
#[utoipa::path(
    get,
    path = "/api/v1/geographic/regions/{id}",
    params(
        ("id" = Uuid, Path, description = "Geographic region ID")
    ),
    responses(
        (status = 200, description = "Geographic region retrieved successfully", body = GeographicRegion),
        (status = 404, description = "Geographic region not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_geographic_region(
    State(_server): State<RustCareServer>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<GeographicRegion>, StatusCode> {
    // TODO: Implement database query with RLS
    // Sample data for development
    let region = GeographicRegion {
        id,
        code: "US-CA".to_string(),
        name: "California".to_string(),
        region_type: "state".to_string(),
        parent_region_id: Some(Uuid::new_v4()),
        path: Some("US.US-CA".to_string()),
        level: 1,
        iso_country_code: Some("US".to_string()),
        iso_subdivision_code: Some("US-CA".to_string()),
        timezone: Some("America/Los_Angeles".to_string()),
        population: Some(39500000),
        area_sq_km: Some(423970.0),
        is_active: true,
        metadata: serde_json::json!({}),
    };

    Ok(Json(region))
}

/// Update geographic region
#[utoipa::path(
    put,
    path = "/api/v1/geographic/regions/{id}",
    params(
        ("id" = Uuid, Path, description = "Geographic region ID")
    ),
    request_body = CreateGeographicRegionRequest,
    responses(
        (status = 200, description = "Geographic region updated successfully", body = GeographicRegion),
        (status = 404, description = "Geographic region not found"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_geographic_region(
    State(_server): State<RustCareServer>,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateGeographicRegionRequest>,
) -> Result<ResponseJson<GeographicRegion>, StatusCode> {
    // TODO: Implement database update with RLS
    let region = GeographicRegion {
        id,
        code: request.code,
        name: request.name,
        region_type: request.region_type,
        parent_region_id: request.parent_region_id,
        path: None, // TODO: Recalculate from parent
        level: 0, // TODO: Recalculate from parent
        iso_country_code: request.iso_country_code,
        iso_subdivision_code: request.iso_subdivision_code,
        timezone: request.timezone,
        population: request.population,
        area_sq_km: request.area_sq_km,
        is_active: true,
        metadata: request.metadata.unwrap_or_else(|| serde_json::json!({})),
    };

    Ok(Json(region))
}

/// Delete geographic region
#[utoipa::path(
    delete,
    path = "/api/v1/geographic/regions/{id}",
    params(
        ("id" = Uuid, Path, description = "Geographic region ID")
    ),
    responses(
        (status = 204, description = "Geographic region deleted successfully"),
        (status = 404, description = "Geographic region not found"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Cannot delete region with children"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_geographic_region(
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Implement database delete with RLS and dependency checks
    Ok(StatusCode::NO_CONTENT)
}

/// Get postal code compliance mapping
#[utoipa::path(
    get,
    path = "/api/v1/geographic/postal-codes/{postal_code}/compliance",
    params(
        ("postal_code" = String, Path, description = "Postal code")
    ),
    responses(
        (status = 200, description = "Postal code compliance mapping retrieved", body = PostalCodeMapping),
        (status = 404, description = "Postal code not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic"
)]
pub async fn get_postal_code_compliance(
    State(_server): State<RustCareServer>,
    Path(postal_code): Path<String>,
) -> Result<ResponseJson<PostalCodeMapping>, StatusCode> {
    // TODO: Implement postal code lookup with compliance auto-assignment
    let mapping = PostalCodeMapping {
        id: Uuid::new_v4(),
        postal_code: postal_code.clone(),
        region_id: Uuid::new_v4(),
        country_code: "US".to_string(),
        compliance_frameworks: vec!["HIPAA".to_string(), "HITECH".to_string()],
        regulatory_authorities: vec!["HHS".to_string(), "OCR".to_string()],
    };

    Ok(Json(mapping))
}

/// Get geographic hierarchy for a region
#[utoipa::path(
    get,
    path = "/api/v1/geographic/regions/{id}/hierarchy",
    params(
        ("id" = Uuid, Path, description = "Geographic region ID")
    ),
    responses(
        (status = 200, description = "Geographic hierarchy retrieved", body = Vec<GeographicRegion>),
        (status = 404, description = "Geographic region not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "geographic"
)]
pub async fn get_geographic_hierarchy(
    State(_server): State<RustCareServer>,
    Path(_id): Path<Uuid>,
) -> Result<ResponseJson<Vec<GeographicRegion>>, StatusCode> {
    // TODO: Implement hierarchical query using ltree to get full path
    let hierarchy = Vec::new();
    Ok(Json(hierarchy))
}