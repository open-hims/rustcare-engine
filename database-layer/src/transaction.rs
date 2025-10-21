// Transaction management with RLS support
use crate::connection::DatabasePool;
use crate::rls::RlsContext;
use crate::error::{DatabaseError, DatabaseResult};
use sqlx::{Transaction, Postgres};
use tracing::{debug, info, error};

/// Transaction manager with automatic RLS context application
pub struct TransactionManager {
    pool: DatabasePool,
    rls_context: Option<RlsContext>,
}

impl TransactionManager {
    pub fn new(pool: DatabasePool) -> Self {
        Self {
            pool,
            rls_context: None,
        }
    }

    /// Set RLS context for this transaction
    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    /// Begin a new transaction
    pub async fn begin(&self) -> DatabaseResult<Transaction<'_, Postgres>> {
        debug!("Beginning transaction");
        
        let mut tx = self.pool.pool()
            .begin()
            .await
            .map_err(|e| DatabaseError::QueryFailed(format!("Failed to begin transaction: {}", e)))?;

        // Apply RLS context if available
        if let Some(context) = &self.rls_context {
            let sql = format!(
                "SET LOCAL app.current_user_id = '{}'; \
                 SET LOCAL app.current_tenant_id = '{}'; \
                 SET LOCAL app.user_roles = '{}'; \
                 SET LOCAL app.user_permissions = '{}';",
                context.user_id,
                context.tenant_id,
                context.roles.join(","),
                context.permissions.join(",")
            );

            sqlx::query(&sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| DatabaseError::QueryFailed(format!("Failed to apply RLS context: {}", e)))?;
        }

        Ok(tx)
    }
}