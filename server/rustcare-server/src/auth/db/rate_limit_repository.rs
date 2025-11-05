/// Rate limiting repository
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::RateLimit;
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use std::sync::Arc;

pub struct RateLimitRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl RateLimitRepository {
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
                ctx.user_id, &ctx.tenant_id, operation, "rate_limits", record_id, metadata
            ).await;
        }
    }
    
    /// Get rate limit by key (used by auth providers)
    pub async fn get_by_key(
        &self,
        key_type: &str,
        key_value: &str,
    ) -> DbResult<Option<RateLimit>> {
        sqlx::query_as!(
            RateLimit,
            r#"
            SELECT *
            FROM rate_limits
            WHERE key_type = $1 AND key_value = $2
            "#,
            key_type,
            key_value
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Increment request count (returns updated count)
    pub async fn increment(
        &self,
        key_type: &str,
        key_value: &str,
        endpoint: Option<&str>,
        window_duration_seconds: i64,
    ) -> DbResult<i32> {
        let result = sqlx::query!(
            r#"
            INSERT INTO rate_limits (key_type, key_value, endpoint, request_count, window_start, window_end)
            VALUES ($1, $2, $3, 1, NOW(), NOW() + ($4 || ' seconds')::interval)
            ON CONFLICT (key_type, key_value, endpoint, window_start)
            DO UPDATE SET
                request_count = CASE 
                    WHEN rate_limits.window_end < NOW() THEN 1
                    ELSE rate_limits.request_count + 1
                END,
                window_start = CASE 
                    WHEN rate_limits.window_end < NOW() THEN NOW()
                    ELSE rate_limits.window_start
                END,
                window_end = CASE 
                    WHEN rate_limits.window_end < NOW() THEN NOW() + ($4 || ' seconds')::interval
                    ELSE rate_limits.window_end
                END,
                updated_at = NOW()
            RETURNING request_count
            "#,
            key_type,
            key_value,
            endpoint,
            window_duration_seconds.to_string()
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.request_count)
    }
    
    /// Record a request
    pub async fn record_request(
        &self,
        key_type: &str,
        key_value: &str,
        endpoint: Option<&str>,
        window_duration_seconds: i64,
    ) -> DbResult<RateLimit> {
        sqlx::query_as!(
            RateLimit,
            r#"
            INSERT INTO rate_limits (key_type, key_value, endpoint, request_count, window_start, window_end)
            VALUES ($1, $2, $3, 1, NOW(), NOW() + ($4 || ' seconds')::interval)
            ON CONFLICT (key_type, key_value, endpoint, window_start)
            DO UPDATE SET
                request_count = CASE 
                    WHEN rate_limits.window_end < NOW() THEN 1
                    ELSE rate_limits.request_count + 1
                END,
                window_start = CASE 
                    WHEN rate_limits.window_end < NOW() THEN NOW()
                    ELSE rate_limits.window_start
                END,
                window_end = CASE 
                    WHEN rate_limits.window_end < NOW() THEN NOW() + ($4 || ' seconds')::interval
                    ELSE rate_limits.window_end
                END,
                updated_at = NOW()
            RETURNING *
            "#,
            key_type,
            key_value,
            endpoint,
            window_duration_seconds.to_string()
        )
        .fetch_one(self.pool.get())
        .await
    }
    
    /// Get current rate limit status
    pub async fn get_status(
        &self,
        key_type: &str,
        key_value: &str,
        endpoint: Option<&str>,
    ) -> DbResult<Option<RateLimit>> {
        sqlx::query_as!(
            RateLimit,
            r#"
            SELECT *
            FROM rate_limits
            WHERE key_type = $1 
                AND key_value = $2
                AND ($3::text IS NULL OR endpoint = $3)
            "#,
            key_type,
            key_value,
            endpoint
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Lock an entity (e.g., after too many failed attempts)
    pub async fn lock(
        &self,
        key_type: &str,
        key_value: &str,
        lock_duration_minutes: i32,
    ) -> DbResult<RateLimit> {
        let rate_limit = sqlx::query_as!(
            RateLimit,
            r#"
            UPDATE rate_limits
            SET 
                locked_until = NOW() + ($3 || ' minutes')::interval,
                updated_at = NOW()
            WHERE key_type = $1 AND key_value = $2
            RETURNING *
            "#,
            key_type,
            key_value,
            lock_duration_minutes.to_string()
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("rate_limit_lock", Some(key_value), serde_json::json!({
            "key_type": key_type,
            "lock_duration_minutes": lock_duration_minutes
        })).await;

        Ok(rate_limit)
    }
    
    /// Unlock an entity
    pub async fn unlock(
        &self,
        key_type: &str,
        key_value: &str,
    ) -> DbResult<RateLimit> {
        let rate_limit = sqlx::query_as!(
            RateLimit,
            r#"
            UPDATE rate_limits
            SET 
                locked_until = NULL,
                updated_at = NOW()
            WHERE key_type = $1 AND key_value = $2
            RETURNING *
            "#,
            key_type,
            key_value
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("rate_limit_unlock", Some(key_value), serde_json::json!({
            "key_type": key_type
        })).await;

        Ok(rate_limit)
    }
    
    /// Unlock an entity and reset request count
    pub async fn unlock_and_reset(
        &self,
        key_type: &str,
        key_value: &str,
    ) -> DbResult<Option<RateLimit>> {
        let rate_limit = sqlx::query_as!(
            RateLimit,
            r#"
            UPDATE rate_limits
            SET 
                locked_until = NULL,
                request_count = 0,
                updated_at = NOW()
            WHERE key_type = $1 AND key_value = $2
            RETURNING *
            "#,
            key_type,
            key_value
        )
        .fetch_optional(self.pool.get())
        .await?;

        if let Some(ref rl) = rate_limit {
            self.log_audit("rate_limit_unlock_reset", Some(key_value), serde_json::json!({
                "key_type": key_type
            })).await;
        }

        Ok(rate_limit)
    }
    
    /// Check if entity is locked
    pub async fn is_locked(
        &self,
        key_type: &str,
        key_value: &str,
    ) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT 
                locked_until IS NOT NULL 
                AND locked_until > NOW() as "locked!"
            FROM rate_limits
            WHERE key_type = $1 AND key_value = $2
            "#,
            key_type,
            key_value
        )
        .fetch_optional(self.pool.get())
        .await?;
        
        Ok(result.map(|r| r.locked).unwrap_or(false))
    }
    
    /// Reset rate limit counter
    pub async fn reset(
        &self,
        key_type: &str,
        key_value: &str,
        endpoint: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE rate_limits
            SET 
                request_count = 0,
                window_start = NOW(),
                updated_at = NOW()
            WHERE key_type = $1 
                AND key_value = $2
                AND ($3::text IS NULL OR endpoint = $3)
            "#,
            key_type,
            key_value,
            endpoint
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Cleanup expired locks
    pub async fn cleanup_expired_locks(&self) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE rate_limits
            SET 
                locked_until = NULL,
                updated_at = NOW()
            WHERE locked_until IS NOT NULL 
                AND locked_until < NOW()
            "#
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Cleanup old rate limit records
    pub async fn cleanup_old_records(&self, days: i32) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM rate_limits
            WHERE window_end < NOW() - ($1 || ' days')::interval
                AND (locked_until IS NULL OR locked_until < NOW())
            "#,
            days.to_string()
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Get all locked entities
    pub async fn get_locked_entities(&self) -> DbResult<Vec<RateLimit>> {
        sqlx::query_as!(
            RateLimit,
            r#"
            SELECT *
            FROM rate_limits
            WHERE locked_until IS NOT NULL 
                AND locked_until > NOW()
            ORDER BY locked_until DESC
            "#
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Count locked entities by type
    pub async fn count_locked_by_type(&self, key_type: &str) -> DbResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM rate_limits
            WHERE key_type = $1
                AND locked_until IS NOT NULL 
                AND locked_until > NOW()
            "#,
            key_type
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.count)
    }
}
