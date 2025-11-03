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

/// Trait for CRUD operations on database entities
///
/// Implement this trait to get standard list/get/delete operations.
/// Override create/update methods for custom logic.
#[async_trait]
pub trait CrudHandler<T, CreateReq, UpdateReq, ListParams>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Sync,
    CreateReq: Send + Sync,
    UpdateReq: Send + Sync,
    ListParams: Send + Sync,
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
        let mut query = PaginatedQuery::new(&format!(
            "SELECT * FROM {} WHERE is_deleted = false",
            Self::table_name()
        ));
        
        // Apply custom filters
        Self::apply_filters(&mut query, &params)?;
        
        // Apply default ordering and pagination
        query
            .order_by_created_desc()
            .paginate(
                Self::extract_page(&params),
                Self::extract_page_size(&params)
            );
        
        let results = query.build::<T>().fetch_all(&server.db_pool).await?;
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

