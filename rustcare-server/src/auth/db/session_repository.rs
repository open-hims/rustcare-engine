/// Session repository for managing user sessions
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use super::super::models::{Session, ActiveSessionWithUser};
use super::{RlsContext};
use database_layer::AuditLogger;
use sqlx::PgPool;
use ipnetwork::IpNetwork;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::Arc;

pub type DbResult<T> = Result<T, sqlx::Error>;

#[derive(Clone)]
pub struct SessionRepository {
    pool: PgPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl SessionRepository {
    pub fn new(pool: PgPool) -> Self {
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
                ctx.user_id, &ctx.tenant_id, operation, "sessions", record_id, metadata
            ).await;
        }
    }
    
    /// Create a new session
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        user_id: Uuid,
        session_token: &str,
        device_fingerprint: Option<&str>,
        user_agent: Option<&str>,
        ip_address: Option<IpNetwork>,
        device_name: Option<&str>,
        device_type: Option<&str>,
        expires_at: DateTime<Utc>,
        auth_method: &str,
        cert_serial: Option<&str>,
        oauth_provider: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> DbResult<Session> {
        let session = sqlx::query_as!(
            Session,
            r#"
            INSERT INTO sessions (
                user_id, session_token, device_fingerprint, user_agent,
                ip_address, device_name, device_type, expires_at,
                auth_method, cert_serial, oauth_provider, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
            user_id,
            session_token,
            device_fingerprint,
            user_agent,
            ip_address,
            device_name,
            device_type,
            expires_at,
            auth_method,
            cert_serial,
            oauth_provider,
            metadata
        )
        .fetch_one(&self.pool)
        .await?;

        self.log_audit("create_session", Some(&user_id.to_string()), serde_json::json!({
            "auth_method": auth_method,
            "device_name": device_name,
            "oauth_provider": oauth_provider
        })).await;

        Ok(session)
    }
    
    /// Find session by token
    pub async fn find_by_token(&self, session_token: &str) -> DbResult<Option<Session>> {
        sqlx::query_as!(
            Session,
            r#"
            SELECT *
            FROM sessions
            WHERE session_token = $1
            "#,
            session_token
        )
        .fetch_optional(&self.pool)
        .await
    }
    
    /// Find session by ID
    pub async fn find_by_id(&self, session_id: Uuid) -> DbResult<Option<Session>> {
        sqlx::query_as!(
            Session,
            r#"
            SELECT *
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_optional(&self.pool)
        .await
    }
    
    /// Update last activity
    pub async fn update_activity(&self, session_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE sessions
            SET last_activity_at = NOW()
            WHERE id = $1 AND active = true
            "#,
            session_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    /// Terminate session
    pub async fn terminate(
        &self,
        session_id: Uuid,
        reason: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE sessions
            SET 
                active = false,
                terminated_at = NOW(),
                termination_reason = $2
            WHERE id = $1
            "#,
            session_id,
            reason
        )
        .execute(&self.pool)
        .await?;

        self.log_audit("terminate_session", Some(&session_id.to_string()), serde_json::json!({
            "reason": reason.unwrap_or("user_logout")
        })).await;

        Ok(())
    }
    
    /// Terminate all user sessions
    pub async fn terminate_all_user_sessions(
        &self,
        user_id: Uuid,
        reason: Option<&str>,
    ) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE sessions
            SET 
                active = false,
                terminated_at = NOW(),
                termination_reason = $2
            WHERE user_id = $1 AND active = true
            "#,
            user_id,
            reason
        )
        .execute(&self.pool)
        .await?;
        
        let rows_affected = result.rows_affected();

        self.log_audit("terminate_all_user_sessions", Some(&user_id.to_string()), serde_json::json!({
            "reason": reason.unwrap_or("logout_all_devices"),
            "sessions_terminated": rows_affected
        })).await;
        
        Ok(rows_affected)
    }
    
    /// Get active sessions for user
    pub async fn get_user_active_sessions(&self, user_id: Uuid) -> DbResult<Vec<Session>> {
        sqlx::query_as!(
            Session,
            r#"
            SELECT *
            FROM sessions
            WHERE user_id = $1 
                AND active = true 
                AND expires_at > NOW()
            ORDER BY last_activity_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }
    
    /// Get all active sessions with user info
    pub async fn get_all_active_with_users(&self, limit: i64, offset: i64) -> DbResult<Vec<ActiveSessionWithUser>> {
        sqlx::query_as!(
            ActiveSessionWithUser,
            r#"
            SELECT 
                s.id,
                s.user_id,
                u.email,
                u.full_name,
                u.status as "user_status!",
                s.session_token,
                s.device_name,
                s.last_activity_at,
                s.expires_at,
                s.auth_method
            FROM active_sessions s
            JOIN users u ON s.user_id = u.id
            ORDER BY s.last_activity_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_expired(&self) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE sessions
            SET 
                active = false,
                terminated_at = NOW(),
                termination_reason = 'expired'
            WHERE expires_at < NOW() AND active = true
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Clean up idle sessions
    pub async fn cleanup_idle(&self, idle_timeout_minutes: i32) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE sessions
            SET 
                active = false,
                terminated_at = NOW(),
                termination_reason = 'idle_timeout'
            WHERE last_activity_at < NOW() - ($1 || ' minutes')::interval
                AND active = true
            "#,
            idle_timeout_minutes.to_string()
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Count active sessions for user
    pub async fn count_user_sessions(&self, user_id: Uuid) -> DbResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM sessions
            WHERE user_id = $1 
                AND active = true 
                AND expires_at > NOW()
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.count)
    }
    
    /// Validate session matches device fingerprint
    pub async fn validate_fingerprint(
        &self,
        session_id: Uuid,
        fingerprint: &str,
    ) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT device_fingerprint
            FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result
            .and_then(|r| r.device_fingerprint)
            .map(|fp| fp == fingerprint)
            .unwrap_or(false))
    }
    
    /// Delete session permanently
    pub async fn delete(&self, session_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM sessions
            WHERE id = $1
            "#,
            session_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
