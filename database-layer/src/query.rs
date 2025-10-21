// Query builder and executor with RLS support
use crate::error::{DatabaseError, DatabaseResult};
use crate::rls::RlsContext;
use crate::connection::DatabasePool;
use sqlx::{Row, FromRow};
use tracing::{debug, error};

/// Query executor with automatic RLS context application
pub struct QueryExecutor {
    pool: DatabasePool,
    rls_context: Option<RlsContext>,
}

impl QueryExecutor {
    pub fn new(pool: DatabasePool) -> Self {
        Self {
            pool,
            rls_context: None,
        }
    }

    /// Set RLS context for this query
    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    /// Execute a query and return a single row
    pub async fn fetch_one<T>(&self, sql: &str) -> DatabaseResult<T>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query: {}", sql);

        sqlx::query_as::<_, T>(sql)
            .fetch_one(self.pool.pool())
            .await
            .map_err(|e| {
                error!("Query failed: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })
    }

    /// Execute a query and return all rows
    pub async fn fetch_all<T>(&self, sql: &str) -> DatabaseResult<Vec<T>>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query: {}", sql);

        sqlx::query_as::<_, T>(sql)
            .fetch_all(self.pool.pool())
            .await
            .map_err(|e| {
                error!("Query failed: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })
    }

    /// Execute a query and return optional row
    pub async fn fetch_optional<T>(&self, sql: &str) -> DatabaseResult<Option<T>>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query: {}", sql);

        sqlx::query_as::<_, T>(sql)
            .fetch_optional(self.pool.pool())
            .await
            .map_err(|e| {
                error!("Query failed: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })
    }

    /// Execute a command (INSERT, UPDATE, DELETE)
    pub async fn execute(&self, sql: &str) -> DatabaseResult<u64> {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing command: {}", sql);

        let result = sqlx::query(sql)
            .execute(self.pool.pool())
            .await
            .map_err(|e| {
                error!("Command failed: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })?;

        Ok(result.rows_affected())
    }
}

pub struct QueryBuilder;

impl QueryBuilder {
    pub fn new() -> Self {
        Self
    }
}