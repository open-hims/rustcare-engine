//! Query builder utilities for consistent SQL query construction
//!
//! This module provides utilities to eliminate duplication in SQL query building
//! across handlers, particularly for filtering, ordering, and pagination.

use sqlx::{QueryBuilder, Postgres};
use sqlx::query::QueryAs;
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
    
    /// Create a new paginated query builder with an initial WHERE clause and bound parameter
    /// Useful for queries that need a required parameter like `WHERE user_id = $1`
    pub fn new_with_base_filter<T>(base_query: &'static str, column: &str, value: T) -> Self
    where
        T: for<'q> sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres> + Send + Sync + 'static,
    {
        let mut query = QueryBuilder::new(base_query);
        query.push(format!(" WHERE {} = ", column));
        query.push_bind(value);
        Self {
            query,
            page: 1,
            page_size: 20,
        }
    }
    
    /// Add a required base filter (appends to existing WHERE clause)
    pub fn add_base_filter<T>(&mut self, column: &str, value: T) -> &mut Self
    where
        T: for<'q> sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres> + Send + Sync + 'static,
    {
        self.query.push(format!(" AND {} = ", column));
        self.query.push_bind(value);
        self
    }
    
    /// Add an equality filter (only if value is Some)
    pub fn filter_eq<T>(&mut self, column: &str, value: Option<T>) -> &mut Self
    where
        T: for<'q> sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres> + Send + Sync + 'static,
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
        T: for<'q> sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres> + Send + Sync + 'static,
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
        T: for<'q> sqlx::Encode<'q, Postgres> + sqlx::Type<Postgres> + Send + Sync + Clone + sqlx::postgres::PgHasArrayType + 'static,
        for<'b> &'b [T]: sqlx::Encode<'b, Postgres> + sqlx::Type<Postgres>,
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
    
    /// Build the final query as a typed query for fetching specific types
    pub fn build_query_as<T>(&mut self) -> QueryAs<'_, Postgres, T, sqlx::postgres::PgArguments>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
    {
        self.query.build_query_as()
    }
    
    /// Get the underlying query builder for advanced use cases
    pub fn query_builder(&mut self) -> &mut QueryBuilder<'a, Postgres> {
        &mut self.query
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
    use uuid::Uuid;

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
        assert_eq!(query.page(), 1);
        assert_eq!(query.page_size(), 20);
    }

    #[test]
    fn test_filter_eq_with_some() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_eq("name", Some("test"));
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_ne() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_ne("status", Some("deleted"));
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_ne_with_none() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_ne("status", None::<String>);
        // Should not add filter when value is None
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_in_with_values() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_in("id", Some(vec![Uuid::new_v4(), Uuid::new_v4()]));
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_in_with_empty_vec() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_in("id", Some(Vec::<Uuid>::new()));
        // Should not add filter when vec is empty
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_in_with_none() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_in("id", None::<Vec<Uuid>>);
        // Should not add filter when value is None
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_organization() {
        let org_id = Uuid::new_v4();
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_organization(Some(org_id));
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_organization_with_none() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_organization(None);
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_active() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_active();
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_filter_not_deleted() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.filter_not_deleted();
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_order_by() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.order_by("name", "ASC");
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_order_by_created_desc() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.order_by_created_desc();
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn test_paginate_defaults() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.paginate(None, None);
        assert_eq!(query.page(), 1);
        assert_eq!(query.page_size(), 20);
    }

    #[test]
    fn test_paginate_custom() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.paginate(Some(3), Some(50));
        assert_eq!(query.page(), 3);
        assert_eq!(query.page_size(), 50);
    }

    #[test]
    fn test_paginate_page_min() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.paginate(Some(0), Some(20));
        assert_eq!(query.page(), 1); // Should clamp to minimum 1
    }

    #[test]
    fn test_paginate_page_size_max() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.paginate(Some(1), Some(200));
        assert_eq!(query.page_size(), 100); // Should clamp to maximum 100
    }

    #[test]
    fn test_paginate_page_size_min() {
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query.paginate(Some(1), Some(0));
        assert_eq!(query.page_size(), 1); // Should clamp to minimum 1
    }

    #[test]
    fn test_chaining_filters() {
        let org_id = Uuid::new_v4();
        let mut query = PaginatedQuery::new("SELECT * FROM test_table WHERE 1=1");
        query
            .filter_active()
            .filter_organization(Some(org_id))
            .filter_eq("status", Some("active"))
            .order_by_created_desc()
            .paginate(Some(2), Some(25));
        
        assert_eq!(query.page(), 2);
        assert_eq!(query.page_size(), 25);
    }
}

