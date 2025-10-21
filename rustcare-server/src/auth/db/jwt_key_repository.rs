/// JWT signing key repository
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::{JwtSigningKey, KeyStatus};
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use std::sync::Arc;

pub struct JwtKeyRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl JwtKeyRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { 
            pool,
            rls_context: None,
            audit_logger: None,
        }
    }

    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    pub fn with_audit_logger(mut self, logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    pub fn rls_context(&self) -> Option<&RlsContext> {
        self.rls_context.as_ref()
    }

    async fn log_audit(&self, operation: &str, record_id: Option<&str>, metadata: serde_json::Value) {
        if let (Some(logger), Some(ctx)) = (&self.audit_logger, &self.rls_context) {
            let _ = logger.log_operation(
                ctx.user_id, &ctx.tenant_id, operation, "jwt_signing_keys", record_id, metadata
            ).await;
        }
    }
    
    /// Create a new signing key
    pub async fn create(
        &self,
        kid: &str,
        algorithm: &str,
        private_key_pem: &str,
        public_key_pem: &str,
        key_size: Option<i32>,
        is_primary: bool,
    ) -> DbResult<JwtSigningKey> {
        let key = sqlx::query_as!(
            JwtSigningKey,
            r#"
            INSERT INTO jwt_signing_keys (
                kid, algorithm, private_key_pem, public_key_pem,
                status, is_primary, key_size, activated_at
            )
            VALUES ($1, $2, $3, $4, 'active', $5, $6, CASE WHEN $5 THEN NOW() ELSE NULL END)
            RETURNING 
                id, kid, algorithm, private_key_pem, public_key_pem,
                status as "status: _",
                is_primary, created_at, activated_at, rotated_at,
                retired_at, expires_at, tokens_signed, last_used_at,
                key_size, rotation_reason
            "#,
            kid,
            algorithm,
            private_key_pem,
            public_key_pem,
            is_primary,
            key_size
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("create_jwt_key", Some(kid), serde_json::json!({
            "algorithm": algorithm,
            "is_primary": is_primary,
            "key_size": key_size
        })).await;

        Ok(key)
    }
    
    /// Get current primary key
    pub async fn get_primary(&self) -> DbResult<Option<JwtSigningKey>> {
        sqlx::query_as!(
            JwtSigningKey,
            r#"
            SELECT 
                id, kid, algorithm, private_key_pem, public_key_pem,
                status as "status: _",
                is_primary, created_at, activated_at, rotated_at,
                retired_at, expires_at, tokens_signed, last_used_at,
                key_size, rotation_reason
            FROM jwt_signing_keys
            WHERE is_primary = true AND status = 'active'
            LIMIT 1
            "#
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Get key by kid
    pub async fn find_by_kid(&self, kid: &str) -> DbResult<Option<JwtSigningKey>> {
        sqlx::query_as!(
            JwtSigningKey,
            r#"
            SELECT 
                id, kid, algorithm, private_key_pem, public_key_pem,
                status as "status: _",
                is_primary, created_at, activated_at, rotated_at,
                retired_at, expires_at, tokens_signed, last_used_at,
                key_size, rotation_reason
            FROM jwt_signing_keys
            WHERE kid = $1
            "#,
            kid
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Get all active keys (for JWKS)
    pub async fn get_active_keys(&self) -> DbResult<Vec<JwtSigningKey>> {
        sqlx::query_as!(
            JwtSigningKey,
            r#"
            SELECT 
                id, kid, algorithm, private_key_pem, public_key_pem,
                status as "status: _",
                is_primary, created_at, activated_at, rotated_at,
                retired_at, expires_at, tokens_signed, last_used_at,
                key_size, rotation_reason
            FROM jwt_signing_keys
            WHERE status IN ('active', 'rotating')
            ORDER BY is_primary DESC, created_at DESC
            "#
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Set a key as primary (and demote others)
    pub async fn set_primary(&self, kid: &str) -> DbResult<JwtSigningKey> {
        // First, demote all other primary keys
        sqlx::query!(
            r#"
            UPDATE jwt_signing_keys
            SET is_primary = false
            WHERE is_primary = true AND kid != $1
            "#,
            kid
        )
        .execute(self.pool.get())
        .await?;
        
        // Then promote the new primary
        let key = sqlx::query_as!(
            JwtSigningKey,
            r#"
            UPDATE jwt_signing_keys
            SET 
                is_primary = true,
                activated_at = COALESCE(activated_at, NOW())
            WHERE kid = $1
            RETURNING 
                id, kid, algorithm, private_key_pem, public_key_pem,
                status as "status: _",
                is_primary, created_at, activated_at, rotated_at,
                retired_at, expires_at, tokens_signed, last_used_at,
                key_size, rotation_reason
            "#,
            kid
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("set_primary_jwt_key", Some(kid), serde_json::json!({
            "algorithm": &key.algorithm
        })).await;

        Ok(key)
    }
    
    /// Mark key as rotating
    pub async fn start_rotation(&self, kid: &str) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE jwt_signing_keys
            SET 
                status = 'rotating',
                rotated_at = NOW()
            WHERE kid = $1
            "#,
            kid
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Retire a key
    pub async fn retire(
        &self,
        kid: &str,
        reason: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE jwt_signing_keys
            SET 
                status = 'retired',
                is_primary = false,
                retired_at = NOW(),
                expires_at = $2,
                rotation_reason = $3
            WHERE kid = $1
            "#,
            kid,
            expires_at,
            reason
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Increment token signed counter
    pub async fn increment_tokens_signed(&self, kid: &str) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE jwt_signing_keys
            SET 
                tokens_signed = tokens_signed + 1,
                last_used_at = NOW()
            WHERE kid = $1
            "#,
            kid
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Clean up expired retired keys
    pub async fn delete_expired(&self) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM jwt_signing_keys
            WHERE status = 'retired' 
                AND expires_at IS NOT NULL
                AND expires_at < NOW()
            "#
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Get key statistics
    pub async fn get_key_stats(&self) -> DbResult<Vec<(String, String, i64, Option<DateTime<Utc>>)>> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                kid,
                status,
                tokens_signed,
                last_used_at
            FROM jwt_signing_keys
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(self.pool.get())
        .await?;
        
        Ok(rows
            .into_iter()
            .map(|r| (r.kid, r.status, r.tokens_signed, r.last_used_at))
            .collect())
    }
}
