//! Generic CRUD handler traits and implementations
//!
//! This module provides reusable CRUD operations to eliminate code duplication
//! across handlers.

use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::{http::StatusCode, Json};
use uuid::Uuid;

use crate::error::{ApiError, ApiResponse, api_success};
use crate::server::RustCareServer;
use crate::utils::query_builder::PaginatedQuery;
use crate::middleware::AuthContext;

/// Trait for CRUD operations on database entities
///
/// Implement this trait to get standard list/get/delete operations.
/// Override create/update methods for custom logic.
#[async_trait]
pub trait CrudHandler<T, CreateReq, UpdateReq, ListParams>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Sync + Unpin + 'static,
    CreateReq: Send + Sync + 'static,
    UpdateReq: Send + Sync + 'static,
    ListParams: Send + Sync + 'static,
{
    /// The database table name
    fn table_name() -> &'static str;
    
    /// Default page size for pagination
    fn default_page_size() -> u32 { 20 }
    
    /// List all resources with optional filtering
    async fn list(
        State(server): State<RustCareServer>,
        Query(params): Query<ListParams>,
    ) -> Result<Json<ApiResponse<Vec<T>>>, ApiError> {
        // Build query string to avoid temporary value issue
        const SELECT_QUERY: &str = "SELECT * FROM ";
        const WHERE_NOT_DELETED: &str = " WHERE (is_deleted = false OR is_deleted IS NULL)";
        let table_name = Self::table_name();
        let query_str = format!("{}{}{}", SELECT_QUERY, table_name, WHERE_NOT_DELETED);
        let query_str_static: &'static str = Box::leak(query_str.into_boxed_str());
        let mut query = PaginatedQuery::new(query_str_static);
        
        // Apply custom filters
        Self::apply_filters(&mut query, &params)?;
        
        // Apply default ordering and pagination
        query
            .order_by_created_desc()
            .paginate(
                Self::extract_page(&params),
                Self::extract_page_size(&params)
            );
        
        let results = query.build_query_as::<T>().fetch_all(&server.db_pool).await
            .map_err(|e| ApiError::internal(format!("Failed to list {}: {}", Self::table_name(), e)))?;
        Ok(Json(api_success(results)))
    }
    
    /// Get a single resource by ID
    async fn get(
        State(server): State<RustCareServer>,
        Path(id): Path<Uuid>,
    ) -> Result<Json<ApiResponse<T>>, ApiError> {
        let result = sqlx::query_as::<_, T>(
            &format!(
                "SELECT * FROM {} WHERE id = $1 AND (is_deleted = false OR is_deleted IS NULL)",
                Self::table_name()
            )
        )
        .bind(id)
        .fetch_optional(&server.db_pool)
        .await?;
        
        match result {
            Some(item) => Ok(Json(api_success(item))),
            None => Err(ApiError::not_found(Self::table_name())),
        }
    }
    
    /// Create a new resource
    async fn create(
        State(_server): State<RustCareServer>,
        Json(_req): Json<CreateReq>,
    ) -> Result<Json<ApiResponse<T>>, ApiError> {
        // Default implementation - should be overridden
        Err(ApiError::internal(format!(
            "Create operation not implemented for {}",
            Self::table_name()
        )))
    }
    
    /// Update an existing resource
    async fn update(
        State(_server): State<RustCareServer>,
        Path(_id): Path<Uuid>,
        Json(_req): Json<UpdateReq>,
    ) -> Result<Json<ApiResponse<T>>, ApiError> {
        // Default implementation - should be overridden
        Err(ApiError::internal(format!(
            "Update operation not implemented for {}",
            Self::table_name()
        )))
    }
    
    /// Delete a resource (soft delete by default)
    async fn delete(
        State(server): State<RustCareServer>,
        Path(id): Path<Uuid>,
    ) -> Result<StatusCode, ApiError> {
        // Default soft delete implementation
        let rows_affected = sqlx::query(&format!(
            "UPDATE {} SET is_deleted = true, updated_at = NOW() WHERE id = $1 AND (is_deleted = false OR is_deleted IS NULL)",
            Self::table_name()
        ))
        .bind(id)
        .execute(&server.db_pool)
        .await?
        .rows_affected();
        
        if rows_affected == 0 {
            Err(ApiError::not_found(Self::table_name()))
        } else {
            Ok(StatusCode::NO_CONTENT)
        }
    }
    
    /// Apply custom filters to the query
    ///
    /// Override this method to add custom filtering logic
    fn apply_filters(_query: &mut PaginatedQuery, _params: &ListParams) -> Result<(), ApiError> {
        Ok(()) // Default: no additional filters
    }
    
    /// Extract page number from params
    ///
    /// Override this if your ListParams uses different field names
    fn extract_page(_params: &ListParams) -> Option<u32> {
        None // Default: use pagination defaults
    }
    
    /// Extract page size from params
    ///
    /// Override this if your ListParams uses different field names
    fn extract_page_size(_params: &ListParams) -> Option<u32> {
        None // Default: use pagination defaults
    }
}

/// Trait for CRUD operations that require authentication context
///
/// This trait extends CrudHandler with AuthContext support for organization-scoped operations.
/// Use this for handlers that need to filter by organization_id.
#[async_trait]
pub trait AuthCrudHandler<T, CreateReq, UpdateReq, ListParams>: CrudHandler<T, CreateReq, UpdateReq, ListParams>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Sync + Unpin + 'static,
    CreateReq: Send + Sync + 'static,
    UpdateReq: Send + Sync + 'static,
    ListParams: Send + Sync + 'static,
{
    /// Get a single resource by ID with organization filtering
    async fn get_with_auth(
        State(server): State<RustCareServer>,
        Path(id): Path<Uuid>,
        auth: AuthContext,
    ) -> Result<Json<ApiResponse<T>>, ApiError> {
        let result = sqlx::query_as::<_, T>(
            &format!(
                "SELECT * FROM {} WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)",
                Self::table_name()
            )
        )
        .bind(id)
        .bind(auth.organization_id)
        .fetch_optional(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get {}: {}", Self::table_name(), e)))?;
        
        match result {
            Some(item) => Ok(Json(api_success(item))),
            None => Err(ApiError::not_found(Self::table_name())),
        }
    }
    
    /// Delete a resource with organization filtering (soft delete by default)
    async fn delete_with_auth(
        State(server): State<RustCareServer>,
        Path(id): Path<Uuid>,
        auth: AuthContext,
    ) -> Result<StatusCode, ApiError> {
        let rows_affected = sqlx::query(&format!(
            "UPDATE {} SET is_deleted = true, updated_at = NOW() WHERE id = $1 AND organization_id = $2 AND (is_deleted = false OR is_deleted IS NULL)",
            Self::table_name()
        ))
        .bind(id)
        .bind(auth.organization_id)
        .execute(&server.db_pool)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete {}: {}", Self::table_name(), e)))?
        .rows_affected();
        
        if rows_affected == 0 {
            Err(ApiError::not_found(Self::table_name()))
        } else {
            Ok(StatusCode::NO_CONTENT)
        }
    }
    
    /// Apply filters including organization_id from auth context
    fn apply_filters_with_auth(
        query: &mut PaginatedQuery,
        params: &ListParams,
        auth: &AuthContext,
    ) -> Result<(), ApiError> {
        query.filter_organization(Some(auth.organization_id));
        Self::apply_filters(query, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test trait can be implemented
    struct TestHandler;
    
    impl CrudHandler<TestEntity, (), (), ()> for TestHandler {
        fn table_name() -> &'static str {
            "test_table"
        }
    }
    
    // Placeholder for test entity
    struct TestEntity {
        id: Uuid,
    }
    
    impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for TestEntity {
        fn from_row(_row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
            Ok(TestEntity { id: Uuid::nil() })
        }
    }
}

