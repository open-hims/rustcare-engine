// Database connection management
use async_trait::async_trait;
use crate::error::{DatabaseError, DatabaseResult};
use crate::rls::RlsContext;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Database connection pool wrapper with RLS support
#[derive(Clone)]
pub struct DatabasePool {
    pool: Arc<PgPool>,
    rls_enabled: bool,
    audit_enabled: bool,
    encryption_enabled: bool,
}

impl DatabasePool {
    /// Create a new database pool from connection string
    pub async fn new(connection_string: &str) -> DatabaseResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(connection_string)
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        info!("Database connection pool created successfully");

        Ok(Self {
            pool: Arc::new(pool),
            rls_enabled: false,
            audit_enabled: false,
            encryption_enabled: false,
        })
    }

    /// Enable Row-Level Security
    pub fn with_rls(mut self, enabled: bool) -> Self {
        self.rls_enabled = enabled;
        self
    }

    /// Enable audit logging
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit_enabled = enabled;
        self
    }

    /// Enable encryption
    pub fn with_encryption(mut self, enabled: bool) -> Self {
        self.encryption_enabled = enabled;
        self
    }

    /// Get the underlying PgPool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check if the pool is healthy
    pub async fn is_healthy(&self) -> bool {
        match sqlx::query("SELECT 1")
            .fetch_one(self.pool.as_ref())
            .await
        {
            Ok(_) => true,
            Err(e) => {
                warn!("Database health check failed: {}", e);
                false
            }
        }
    }

    /// Apply RLS context to the connection
    pub async fn apply_rls_context(&self, context: &RlsContext) -> DatabaseResult<()> {
        if !self.rls_enabled {
            return Ok(());
        }

        // Build SET LOCAL commands for PostgreSQL RLS
        let mut sql = format!(
            "SET LOCAL app.current_user_id = '{}'; \
             SET LOCAL app.current_tenant_id = '{}';",
            context.user_id,
            context.tenant_id
        );

        // Set organization_id for PostgreSQL RLS policies (CRITICAL for multi-tenant isolation)
        if let Some(org_id) = context.organization_id {
            sql.push_str(&format!(" SET LOCAL app.organization_id = '{}';", org_id));
        }

        // Set user roles
        if !context.roles.is_empty() {
            sql.push_str(&format!(" SET LOCAL app.user_roles = '{}';", context.roles.join(",")));
        }

        // Set user permissions
        if !context.permissions.is_empty() {
            sql.push_str(&format!(
                " SET LOCAL app.user_permissions = '{}';",
                context.permissions.join(",")
            ));
        }

        sqlx::query(&sql)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| DatabaseError::QueryFailed(format!("Failed to apply RLS context: {}", e)))?;

        Ok(())
    }

    /// Close the pool
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection pool closed");
    }
}

#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn connect(&self) -> DatabaseResult<()>;
    async fn disconnect(&self) -> DatabaseResult<()>;
    async fn is_healthy(&self) -> bool;
}

pub struct PostgresConnection {
    connection_string: String,
    pool: Option<DatabasePool>,
}

impl PostgresConnection {
    pub fn new(connection_string: String) -> Self {
        Self {
            connection_string,
            pool: None,
        }
    }

    pub fn pool(&self) -> Option<&DatabasePool> {
        self.pool.as_ref()
    }
}

#[async_trait]
impl DatabaseConnection for PostgresConnection {
    async fn connect(&self) -> DatabaseResult<()> {
        DatabasePool::new(&self.connection_string).await?;
        info!("PostgreSQL connection established");
        Ok(())
    }
    
    async fn disconnect(&self) -> DatabaseResult<()> {
        if let Some(pool) = &self.pool {
            pool.close().await;
        }
        info!("PostgreSQL connection closed");
        Ok(())
    }
    
    async fn is_healthy(&self) -> bool {
        match &self.pool {
            Some(pool) => pool.is_healthy().await,
            None => false,
        }
    }
}