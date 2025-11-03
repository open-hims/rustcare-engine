//! Query builder utilities for consistent SQL query construction
//!
//! This module provides utilities to eliminate duplication in SQL query building
//! across handlers, particularly for filtering, ordering, and pagination.

use sqlx::{QueryBuilder, Postgres};
use uuid::Uuid;

/// Paginated query builder for consistent query construction
///
/// Example usage:
/// ```rust
/// let mut query = PaginatedQuery::new("SELECT * FROM medical_records WHERE is_deleted = false");
/// query
///     .filter_active()
///     .filter_organization(auth.organization_id)
///     .filter_eq("patient_id", params.patient_id)
///     .order_by("visit_date", "DESC")
///     .paginate(params.page, params.page_size);
///
/// let records: Vec<MedicalRecord> = query.build().fetch_all(&pool).await?;
/// ```
pub struct PaginatedQuery<'a> {
    query: QueryBuilder<'a, Postgres>,
    page: u32,
    page_size: u32,
}

impl<'a> PaginatedQuery<'a> {
    /// Create a new paginated query builder
    pub fn new(base_query: &'static str) -> Self {
        Self {
            query: QueryBuilder::new(base_query),
            page: 1,
            page_size: 20,
        }
    }
    
    /// Add an equality filter (only if value is Some)
    pub fn filter_eq<T>(&mut self, column: &str, value: Option<T>) -> &mut Self
    where
        T: sqlx::Encode<'_, Postgres> + Send + Sync,
    {
        if let Some(val) = value {
            self.query.push(format!(" AND {} = ", column));
            self.query.push_bind(val);
        }
        self
    }
    
    /// Add a not-equal filter (only if value is Some)
    pub fn filter_ne<T>(&mut self, column: &str, value: Option<T>) -> &mut Self
    where
        T: sqlx::Encode<'_, Postgres> + Send + Sync,
    {
        if let Some(val) = value {
            self.query.push(format!(" AND {} != ", column));
            self.query.push_bind(val);
        }
        self
    }
    
    /// Add an IN filter (only if values is Some and non-empty)
    pub fn filter_in<T>(&mut self, column: &str, values: Option<Vec<T>>) -> &mut Self
    where
        T: sqlx::Encode<'_, Postgres> + Send + Sync + Clone,
    {
        if let Some(vals) = values {
            if !vals.is_empty() {
                self.query.push(format!(" AND {} = ANY(", column));
                self.query.push_bind(vals);
                self.query.push(")");
            }
        }
        self
    }
    
    /// Filter by organization_id (common pattern)
    pub fn filter_organization(&mut self, org_id: Option<Uuid>) -> &mut Self {
        self.filter_eq("organization_id", org_id)
    }
    
    /// Filter for active, non-deleted records (common pattern)
    pub fn filter_active(&mut self) -> &mut Self {
        self.query.push(" AND is_active = true AND (is_deleted = false OR is_deleted IS NULL)");
        self
    }
    
    /// Filter for non-deleted records only
    pub fn filter_not_deleted(&mut self) -> &mut Self {
        self.query.push(" AND (is_deleted = false OR is_deleted IS NULL)");
        self
    }
    
    /// Add ORDER BY clause
    pub fn order_by(&mut self, column: &str, direction: &str) -> &mut Self {
        self.query.push(format!(" ORDER BY {} {}", column, direction));
        self
    }
    
    /// Add ORDER BY created_at DESC (common pattern)
    pub fn order_by_created_desc(&mut self) -> &mut Self {
        self.order_by("created_at", "DESC")
    }
    
    /// Apply pagination
    pub fn paginate(&mut self, page: Option<u32>, page_size: Option<u32>) -> &mut Self {
        self.page = page.unwrap_or(1).max(1);
        self.page_size = page_size.unwrap_or(20).min(100).max(1);
        let offset = (self.page - 1) * self.page_size;
        self.query.push(" LIMIT ");
        self.query.push_bind(self.page_size as i64);
        self.query.push(" OFFSET ");
        self.query.push_bind(offset as i64);
        self
    }
    
    /// Build the final query
    pub fn build<T>(&mut self) -> sqlx::QueryAs<'_, Postgres, T, sqlx::postgres::PgArguments>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
    {
        self.query.build_query_as()
    }
    
    /// Get current page
    pub fn page(&self) -> u32 {
        self.page
    }
    
    /// Get current page size
    pub fn page_size(&self) -> u32 {
        self.page_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginated_query_builder() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query
            .filter_eq("status", Some("active"))
            .filter_active()
            .order_by("created_at", "DESC")
            .paginate(Some(2), Some(10));
        
        assert_eq!(query.page(), 2);
        assert_eq!(query.page_size(), 10);
    }

    #[test]
    fn test_filter_eq_with_none() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_eq("status", None::<String>);
        // Should not add filter when value is None
        // This is verified by the query builder not panicking
    }
}

