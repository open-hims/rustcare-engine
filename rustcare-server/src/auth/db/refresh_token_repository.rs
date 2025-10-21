/// Refresh token repository
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::RefreshToken;
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use ipnetwork::IpNetwork;
use std::sync::Arc;

pub struct RefreshTokenRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl RefreshTokenRepository {
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
                ctx.user_id, &ctx.tenant_id, operation, "refresh_tokens", record_id, metadata
            ).await;
        }
    }
    
    /// Create a new refresh token
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        user_id: Uuid,
        token_hash: &str,
        token_family: Uuid,
        device_name: Option<&str>,
        device_fingerprint: Option<&str>,
        user_agent: Option<&str>,
        ip_address: Option<IpNetwork>,
        expires_at: DateTime<Utc>,
        auth_method: Option<&str>,
        cert_serial: Option<&str>,
        parent_token_id: Option<Uuid>,
    ) -> DbResult<RefreshToken> {
        let token = sqlx::query_as!(
            RefreshToken,
            r#"
            INSERT INTO refresh_tokens (
                user_id, token_hash, token_family,
                device_name, device_fingerprint, user_agent, ip_address,
                expires_at, auth_method, cert_serial, parent_token_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
            user_id,
            token_hash,
            token_family,
            device_name,
            device_fingerprint,
            user_agent,
            ip_address,
            expires_at,
            auth_method,
            cert_serial,
            parent_token_id
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("create_refresh_token", Some(&user_id.to_string()), serde_json::json!({
            "token_family": token_family,
            "device_name": device_name,
            "auth_method": auth_method
        })).await;

        Ok(token)
    }
    
    /// Find refresh token by hash
    pub async fn find_by_hash(&self, token_hash: &str) -> DbResult<Option<RefreshToken>> {
        sqlx::query_as!(
            RefreshToken,
            r#"
            SELECT *
            FROM refresh_tokens
            WHERE token_hash = $1
            "#,
            token_hash
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Find refresh token by ID
    pub async fn find_by_id(&self, token_id: Uuid) -> DbResult<Option<RefreshToken>> {
        sqlx::query_as!(
            RefreshToken,
            r#"
            SELECT *
            FROM refresh_tokens
            WHERE id = $1
            "#,
            token_id
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Get all tokens in a family
    pub async fn find_family(&self, token_family: Uuid) -> DbResult<Vec<RefreshToken>> {
        sqlx::query_as!(
            RefreshToken,
            r#"
            SELECT *
            FROM refresh_tokens
            WHERE token_family = $1
            ORDER BY issued_at DESC
            "#,
            token_family
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Update last used time
    pub async fn update_last_used(&self, token_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET last_used_at = NOW()
            WHERE id = $1
            "#,
            token_id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Mark token as replaced by new token (rotation)
    pub async fn mark_replaced(
        &self,
        token_id: Uuid,
        replaced_by_id: Uuid,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET 
                revoked = true,
                revoked_at = NOW(),
                revocation_reason = 'rotated',
                replaced_by_token_id = $2
            WHERE id = $1
            "#,
            token_id,
            replaced_by_id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Revoke token
    pub async fn revoke(
        &self,
        token_id: Uuid,
        reason: &str,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET 
                revoked = true,
                revoked_at = NOW(),
                revocation_reason = $2
            WHERE id = $1
            "#,
            token_id,
            reason
        )
        .execute(self.pool.get())
        .await?;

        self.log_audit("revoke_refresh_token", Some(&token_id.to_string()), serde_json::json!({
            "reason": reason
        })).await;

        Ok(())
    }
    
    /// Revoke entire token family (security breach detected)
    pub async fn revoke_token_family(
        &self,
        token_family: Uuid,
        reason: &str,
    ) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET 
                revoked = true,
                revoked_at = NOW(),
                revocation_reason = $2
            WHERE token_family = $1 AND revoked = false
            "#,
            token_family,
            reason
        )
        .execute(self.pool.get())
        .await?;
        
        let rows_affected = result.rows_affected();
        
        self.log_audit("revoke_token_family", Some(&token_family.to_string()), serde_json::json!({
            "reason": reason,
            "tokens_revoked": rows_affected
        })).await;
        
        Ok(rows_affected)
    }
    
    /// Revoke all tokens for user
    pub async fn revoke_all_user_tokens(
        &self,
        user_id: Uuid,
        reason: &str,
    ) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET 
                revoked = true,
                revoked_at = NOW(),
                revocation_reason = $2
            WHERE user_id = $1 AND revoked = false
            "#,
            user_id,
            reason
        )
        .execute(self.pool.get())
        .await?;
        
        let rows_affected = result.rows_affected();
        
        self.log_audit("revoke_all_user_tokens", Some(&user_id.to_string()), serde_json::json!({
            "reason": reason,
            "tokens_revoked": rows_affected
        })).await;
        
        Ok(rows_affected)
    }
    
    /// Revoke all tokens for user
    pub async fn revoke_all_user_tokens(
        &self,
        user_id: Uuid,
        reason: &str,
    ) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET 
                revoked = true,
                revoked_at = NOW(),
                revocation_reason = $2
            WHERE user_id = $1 AND revoked = false
            "#,
            user_id,
            reason
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Get active tokens for user
    pub async fn get_user_active_tokens(&self, user_id: Uuid) -> DbResult<Vec<RefreshToken>> {
        sqlx::query_as!(
            RefreshToken,
            r#"
            SELECT *
            FROM refresh_tokens
            WHERE user_id = $1 
                AND revoked = false 
                AND expires_at > NOW()
            ORDER BY issued_at DESC
            "#,
            user_id
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE refresh_tokens
            SET 
                revoked = true,
                revoked_at = NOW(),
                revocation_reason = 'expired'
            WHERE expires_at < NOW() AND revoked = false
            "#
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Delete old revoked tokens (cleanup)
    pub async fn delete_old_revoked(&self, days: i32) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE revoked = true 
                AND revoked_at < NOW() - ($1 || ' days')::interval
            "#,
            days.to_string()
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Count active tokens for user
    pub async fn count_user_tokens(&self, user_id: Uuid) -> DbResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM refresh_tokens
            WHERE user_id = $1 
                AND revoked = false 
                AND expires_at > NOW()
            "#,
            user_id
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.count)
    }
}
