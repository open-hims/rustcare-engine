use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::models::{GeographicRegion, PostalCodeRegionMapping, ComplianceFramework, ComplianceRegionMapping};
use crate::rls::RlsContext;

pub type DbResult<T> = Result<T, DatabaseError>;

/// Repository for geographic region operations
#[derive(Debug, Clone)]
pub struct GeographicRepository {
    pool: Pool<Postgres>,
    rls_context: Option<RlsContext>,
}

impl GeographicRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            rls_context: None,
        }
    }

    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    /// List geographic regions with optional filtering
    pub async fn list_regions(
        &self,
        parent_id: Option<Uuid>,
        region_type: Option<&str>,
        search: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> DbResult<Vec<GeographicRegion>> {
        // Start with a simple query for now
        let regions = if let Some(parent) = parent_id {
            sqlx::query_as::<_, GeographicRegion>(
                r#"
                SELECT id, code, name, region_type, parent_region_id, path::text as path, level,
                       iso_country_code, iso_subdivision_code, timezone,
                       coordinates::text as coordinates, population, area_sq_km,
                       metadata, is_active, created_at, updated_at
                FROM geographic_regions
                WHERE is_active = true AND parent_region_id = $1
                ORDER BY name ASC
                LIMIT $2
                "#,
            )
            .bind(parent)
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, GeographicRegion>(
                r#"
                SELECT id, code, name, region_type, parent_region_id, path::text as path, level,
                       iso_country_code, iso_subdivision_code, timezone,
                       coordinates::text as coordinates, population, area_sq_km,
                       metadata, is_active, created_at, updated_at
                FROM geographic_regions
                WHERE is_active = true AND parent_region_id IS NULL
                ORDER BY name ASC
                LIMIT $1
                "#,
            )
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(regions)
    }

    /// Create a new geographic region
    pub async fn create_region(
        &self,
        name: &str,
        code: &str,
        region_type: &str,
        parent_region_id: Option<Uuid>,
        iso_country_code: Option<&str>,
        iso_subdivision_code: Option<&str>,
        timezone: Option<&str>,
        population: Option<i64>,
        area_sq_km: Option<f64>,
        metadata: Option<Value>,
    ) -> DbResult<GeographicRegion> {
        let region = sqlx::query_as::<_, GeographicRegion>(
            r#"
            INSERT INTO geographic_regions (
                code, name, region_type, parent_region_id,
                iso_country_code, iso_subdivision_code, timezone,
                population, area_sq_km, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, code, name, region_type, parent_region_id, path::text as path, level,
                      iso_country_code, iso_subdivision_code, timezone,
                      coordinates::text as coordinates, population, area_sq_km,
                      metadata, is_active, created_at, updated_at
            "#,
        )
        .bind(code)
        .bind(name)
        .bind(region_type)
        .bind(parent_region_id)
        .bind(iso_country_code)
        .bind(iso_subdivision_code)
        .bind(timezone)
        .bind(population)
        .bind(area_sq_km)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(region)
    }

    /// Get a region by ID
    pub async fn get_region(&self, id: Uuid) -> DbResult<Option<GeographicRegion>> {
        let region = sqlx::query_as::<_, GeographicRegion>(
            r#"
            SELECT id, code, name, region_type, parent_region_id, path::text as path, level,
                   iso_country_code, iso_subdivision_code, timezone,
                   coordinates::text as coordinates, population, area_sq_km,
                   metadata, is_active, created_at, updated_at
            FROM geographic_regions
            WHERE id = $1 AND is_active = true
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(region)
    }

    /// Update a region
    pub async fn update_region(
        &self,
        id: Uuid,
        region_name: Option<&str>,
        region_code: Option<&str>,
        iso_country_code: Option<&str>,
        postal_code_pattern: Option<&str>,
        timezone_primary: Option<&str>,
        currency_code: Option<&str>,
        metadata: Option<Value>,
    ) -> DbResult<GeographicRegion> {
        let now = Utc::now();

        let region = sqlx::query_as::<_, GeographicRegion>(
            r#"
            UPDATE geographic_regions
            SET 
                region_name = COALESCE($2, region_name),
                region_code = COALESCE($3, region_code),
                iso_country_code = COALESCE($4, iso_country_code),
                postal_code_pattern = COALESCE($5, postal_code_pattern),
                timezone_primary = COALESCE($6, timezone_primary),
                currency_code = COALESCE($7, currency_code),
                metadata = COALESCE($8, metadata),
                updated_at = $9
            WHERE id = $1 AND is_active = true
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(region_name)
        .bind(region_code)
        .bind(iso_country_code)
        .bind(postal_code_pattern)
        .bind(timezone_primary)
        .bind(currency_code)
        .bind(metadata)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(region)
    }

    /// Delete a region (soft delete)
    pub async fn delete_region(&self, id: Uuid) -> DbResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE geographic_regions
            SET is_active = false, updated_at = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get child regions of a parent region
    pub async fn get_region_children(&self, parent_id: Uuid) -> DbResult<Vec<GeographicRegion>> {
        let regions = sqlx::query_as::<_, GeographicRegion>(
            r#"
            SELECT id, code, name, region_type, parent_region_id, path::text as path, level,
                   iso_country_code, iso_subdivision_code, timezone,
                   coordinates::text as coordinates, population, area_sq_km,
                   metadata, is_active, created_at, updated_at
            FROM geographic_regions
            WHERE parent_region_id = $1 AND is_active = true
            ORDER BY name ASC
            "#,
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(regions)
    }

    /// Search regions by name
    pub async fn search_regions(
        &self,
        search_term: &str,
        limit: Option<i32>,
    ) -> DbResult<Vec<GeographicRegion>> {
        let regions = sqlx::query_as::<_, GeographicRegion>(
            r#"
            SELECT id, code, name, region_type, parent_region_id, path::text as path, level,
                   iso_country_code, iso_subdivision_code, timezone,
                   coordinates::text as coordinates, population, area_sq_km,
                   metadata, is_active, created_at, updated_at
            FROM geographic_regions
            WHERE (name ILIKE $1 OR code ILIKE $1) 
            AND is_active = true
            ORDER BY 
                CASE WHEN name ILIKE $1 THEN 1 ELSE 2 END,
                name ASC
            LIMIT $2
            "#,
        )
        .bind(format!("%{}%", search_term))
        .bind(limit.unwrap_or(50) as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(regions)
    }

    /// Get compliance frameworks by postal code
    pub async fn get_postal_code_compliance(
        &self,
        postal_code: &str,
    ) -> DbResult<Vec<ComplianceFramework>> {
        let frameworks = sqlx::query_as::<_, ComplianceFramework>(
            r#"
            SELECT DISTINCT cf.id, cf.name, cf.code, cf.version, cf.description,
                   cf.authority, cf.jurisdiction, cf.effective_date, cf.review_date,
                   cf.parent_framework_id, cf.is_active, cf.sort_order, cf.metadata,
                   cf.created_at, cf.updated_at
            FROM compliance_frameworks cf
            JOIN compliance_region_mapping crm ON cf.id = crm.framework_id
            JOIN geographic_regions gr ON crm.region_id = gr.id
            JOIN postal_code_region_mapping pcrm ON gr.id = pcrm.region_id
            WHERE pcrm.postal_code = $1 
            AND cf.is_active = true 
            AND crm.is_active = true 
            AND pcrm.is_active = true
            ORDER BY cf.sort_order ASC, cf.name ASC
            "#,
        )
        .bind(postal_code)
        .fetch_all(&self.pool)
        .await?;

        Ok(frameworks)
    }

    /// Create postal code mapping
    pub async fn create_postal_code_mapping(
        &self,
        region_id: Uuid,
        postal_code: &str,
        postal_code_prefix: Option<&str>,
        is_exact_match: bool,
        confidence_score: f64,
        validation_source: Option<&str>,
    ) -> DbResult<PostalCodeRegionMapping> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let mapping = sqlx::query_as::<_, PostalCodeRegionMapping>(
            r#"
            INSERT INTO postal_code_region_mapping (
                id, region_id, postal_code, postal_code_prefix, is_exact_match,
                confidence_score, validation_source, is_active, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(region_id)
        .bind(postal_code)
        .bind(postal_code_prefix)
        .bind(is_exact_match)
        .bind(confidence_score)
        .bind(validation_source)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(mapping)
    }
}