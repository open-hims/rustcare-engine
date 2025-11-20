// Query builder and executor with RLS support
use crate::connection::DatabasePool;
use crate::encryption::DatabaseEncryption;
use crate::error::{DatabaseError, DatabaseResult};
use crate::rls::RlsContext;
use serde_json::Value as JsonValue;
use sqlx::{FromRow, Row};
use std::sync::Arc;
use tracing::{debug, error};

/// Query executor with automatic RLS context application
pub struct QueryExecutor {
    pool: DatabasePool,
    rls_context: Option<RlsContext>,
    encryption: Option<Arc<DatabaseEncryption>>,
}

impl QueryExecutor {
    pub fn new(pool: DatabasePool) -> Self {
        Self {
            pool,
            rls_context: None,
            encryption: None,
        }
    }

    /// Attach an encryption engine to the executor
    pub fn with_encryption(mut self, encryption: DatabaseEncryption) -> Self {
        self.encryption = Some(Arc::new(encryption));
        self
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
        self.fetch_one_with(sql, |q| q).await
    }

    /// Execute a parameterized query and return a single row
    /// Uses a closure to bind parameters (since sqlx requires method chaining)
    pub async fn fetch_one_with<F, T>(&self, sql: &str, bind_fn: F) -> DatabaseResult<T>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        F: FnOnce(
            sqlx::query::QueryAs<'_, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
        )
            -> sqlx::query::QueryAs<'_, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query: {}", sql);

        let query = sqlx::query_as::<_, T>(sql);
        let query = bind_fn(query);

        query.fetch_one(self.pool.pool()).await.map_err(|e| {
            error!("Query failed: {}", e);
            DatabaseError::QueryFailed(e.to_string())
        })
    }

    /// Execute a query and return all rows
    pub async fn fetch_all<T>(&self, sql: &str) -> DatabaseResult<Vec<T>>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        self.fetch_all_with(sql, |q| q).await
    }

    /// Execute a parameterized query and return all rows
    /// Uses a closure to bind parameters
    pub async fn fetch_all_with<F, T>(&self, sql: &str, bind_fn: F) -> DatabaseResult<Vec<T>>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        F: FnOnce(
            sqlx::query::QueryAs<'_, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
        )
            -> sqlx::query::QueryAs<'_, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query: {}", sql);

        let query = sqlx::query_as::<_, T>(sql);
        let query = bind_fn(query);

        query.fetch_all(self.pool.pool()).await.map_err(|e| {
            error!("Query failed: {}", e);
            DatabaseError::QueryFailed(e.to_string())
        })
    }

    /// Execute a query and return optional row
    pub async fn fetch_optional<T>(&self, sql: &str) -> DatabaseResult<Option<T>>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        self.fetch_optional_with(sql, |q| q).await
    }

    /// Execute a parameterized query and return optional row
    /// Uses a closure to bind parameters
    pub async fn fetch_optional_with<F, T>(
        &self,
        sql: &str,
        bind_fn: F,
    ) -> DatabaseResult<Option<T>>
    where
        T: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        F: FnOnce(
            sqlx::query::QueryAs<'_, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
        )
            -> sqlx::query::QueryAs<'_, sqlx::Postgres, T, sqlx::postgres::PgArguments>,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query: {}", sql);

        let query = sqlx::query_as::<_, T>(sql);
        let query = bind_fn(query);

        query.fetch_optional(self.pool.pool()).await.map_err(|e| {
            error!("Query failed: {}", e);
            DatabaseError::QueryFailed(e.to_string())
        })
    }

    /// Execute a command (INSERT, UPDATE, DELETE)
    pub async fn execute(&self, sql: &str) -> DatabaseResult<u64> {
        self.execute_with(sql, |q| q).await
    }

    /// Execute a parameterized command (INSERT, UPDATE, DELETE)
    /// Uses a closure to bind parameters
    pub async fn execute_with<F>(&self, sql: &str, bind_fn: F) -> DatabaseResult<u64>
    where
        F: FnOnce(
            sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments>,
        ) -> sqlx::query::Query<'_, sqlx::Postgres, sqlx::postgres::PgArguments>,
    {
        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing command: {}", sql);

        let query = sqlx::query(sql);
        let query = bind_fn(query);

        let result = query.execute(self.pool.pool()).await.map_err(|e| {
            error!("Command failed: {}", e);
            DatabaseError::QueryFailed(e.to_string())
        })?;

        Ok(result.rows_affected())
    }

    /// Execute a command with ordered parameters where each param can optionally
    /// specify a `table.column` key to indicate the value should be encrypted
    /// before binding. `params` is a slice of (value, Option<table_column>).
    pub async fn execute_with_params(
        &self,
        sql: &str,
        params: &[(&str, Option<&str>)],
    ) -> DatabaseResult<u64> {
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing command with params: {}", sql);

        let mut query = sqlx::query(sql);

        for (val, tablecol) in params {
            let mut to_bind = (*val).to_string();
            if let (Some(enc), Some(tc)) = (&self.encryption, tablecol) {
                // tablecol is expected as "table.column"
                let parts: Vec<&str> = tc.split('.').collect();
                if parts.len() == 2 {
                    let table = parts[0];
                    let column = parts[1];
                    if enc.should_encrypt(table, column) {
                        // encrypt and bind ciphertext
                        if let Ok(ct) = enc.encrypt_value(&to_bind) {
                            to_bind = ct;
                        }
                    }
                }
            }

            query = query.bind(to_bind);
        }

        let result = query.execute(self.pool.pool()).await.map_err(|e| {
            error!("Command failed: {}", e);
            DatabaseError::QueryFailed(e.to_string())
        })?;

        Ok(result.rows_affected())
    }

    /// Fetch a single JSON row and attempt to decrypt any encrypted string fields
    pub async fn fetch_one_json_with_decrypt(
        &self,
        sql: &str,
    ) -> DatabaseResult<serde_json::Value> {
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query (json decrypt): {}", sql);

        let row = sqlx::query(sql)
            .fetch_one(self.pool.pool())
            .await
            .map_err(|e| {
                error!("Query failed: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })?;

        // Extract first column as JSON value
        let json: JsonValue = row.try_get(0).map_err(|e| {
            error!("Failed to extract JSON column: {}", e);
            DatabaseError::QueryFailed(e.to_string())
        })?;

        Ok(self.try_decrypt_json(json))
    }

    /// Fetch multiple JSON rows and attempt to decrypt any encrypted string fields
    pub async fn fetch_all_json_with_decrypt(
        &self,
        sql: &str,
    ) -> DatabaseResult<Vec<serde_json::Value>> {
        if let Some(context) = &self.rls_context {
            self.pool.apply_rls_context(context).await?;
        }

        debug!("Executing query (json decrypt): {}", sql);

        let rows = sqlx::query(sql)
            .fetch_all(self.pool.pool())
            .await
            .map_err(|e| {
                error!("Query failed: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows.into_iter() {
            let json: JsonValue = row.try_get(0).map_err(|e| {
                error!("Failed to extract JSON column: {}", e);
                DatabaseError::QueryFailed(e.to_string())
            })?;
            out.push(self.try_decrypt_json(json));
        }

        Ok(out)
    }

    /// Recursively walk JSON and attempt to decrypt string fields using attached encryption engine
    fn try_decrypt_json(&self, mut v: serde_json::Value) -> serde_json::Value {
        if self.encryption.is_none() {
            return v;
        }

        let enc = self
            .encryption
            .as_ref()
            .expect("encryption must be Some (checked above)")
            .clone();

        fn walk(value: &mut serde_json::Value, enc: &DatabaseEncryption) {
            match value {
                serde_json::Value::String(s) => {
                    if let Ok(decrypted) = enc.decrypt_value(s) {
                        // Only replace if decrypted differs (meaning it was encrypted)
                        if decrypted != *s {
                            *s = decrypted;
                        }
                    }
                }
                serde_json::Value::Array(arr) => {
                    for item in arr.iter_mut() {
                        walk(item, enc);
                    }
                }
                serde_json::Value::Object(map) => {
                    for (_k, v) in map.iter_mut() {
                        walk(v, enc);
                    }
                }
                _ => {}
            }
        }

        walk(&mut v, enc.as_ref());
        v
    }
}

/// Macro to simplify parameterized queries with QueryExecutor
///
/// Example:
/// ```rust
/// executor.fetch_one_with!(sql, |q| q.bind(param1).bind(param2))
/// ```
#[macro_export]
macro_rules! query_exec {
    ($executor:expr, $method:ident, $sql:expr) => {
        $executor.$method($sql).await
    };
    ($executor:expr, $method:ident, $sql:expr, |$q:ident| $bind:expr) => {
        $executor.$method($sql, |$q| $bind).await
    };
}

pub struct QueryBuilder;

impl QueryBuilder {
    pub fn new() -> Self {
        Self
    }
}
