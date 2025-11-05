/// Audit log repository for HIPAA compliance
///
/// Now supports RLS context for multi-tenant isolation

use crate::auth::models::AuthAuditLog;
use super::{DbPool, DbResult, RlsContext};
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use ipnetwork::IpNetwork;

pub struct AuditRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
}

impl AuditRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { 
            pool,
            rls_context: None,
        }
    }

    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    pub fn rls_context(&self) -> Option<&RlsContext> {
        self.rls_context.as_ref()
    }
    
    /// Log an authentication event
    #[allow(clippy::too_many_arguments)]
    pub async fn log_event(
        &self,
        user_id: Option<Uuid>,
        email: Option<&str>,
        event_type: &str,
        event_status: &str,
        auth_method: Option<&str>,
        oauth_provider: Option<&str>,
        cert_serial: Option<&str>,
        ip_address: Option<IpNetwork>,
        user_agent: Option<&str>,
        device_fingerprint: Option<&str>,
        geolocation: Option<serde_json::Value>,
        session_id: Option<Uuid>,
        request_id: Option<&str>,
        endpoint: Option<&str>,
        error_message: Option<&str>,
        metadata: Option<serde_json::Value>,
        anomaly_detected: Option<bool>,
        risk_score: Option<i32>,
        blocked_reason: Option<&str>,
    ) -> DbResult<AuthAuditLog> {
        sqlx::query_as!(
            AuthAuditLog,
            r#"
            INSERT INTO auth_audit_log (
                user_id, email, event_type, event_status,
                auth_method, oauth_provider, cert_serial,
                ip_address, user_agent, device_fingerprint, geolocation,
                session_id, request_id, endpoint, error_message, metadata,
                anomaly_detected, risk_score, blocked_reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            RETURNING *
            "#,
            user_id,
            email,
            event_type,
            event_status,
            auth_method,
            oauth_provider,
            cert_serial,
            ip_address,
            user_agent,
            device_fingerprint,
            geolocation,
            session_id,
            request_id,
            endpoint,
            error_message,
            metadata,
            anomaly_detected,
            risk_score,
            blocked_reason
        )
        .fetch_one(self.pool.get())
        .await
    }
    
    /// Get audit logs for user
    pub async fn get_user_logs(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuthAuditLog>> {
        sqlx::query_as!(
            AuthAuditLog,
            r#"
            SELECT *
            FROM auth_audit_log
            WHERE user_id = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Get logs by event type
    pub async fn get_by_event_type(
        &self,
        event_type: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuthAuditLog>> {
        sqlx::query_as!(
            AuthAuditLog,
            r#"
            SELECT *
            FROM auth_audit_log
            WHERE event_type = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#,
            event_type,
            limit,
            offset
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Get failed login attempts
    pub async fn get_failed_logins(
        &self,
        email: Option<&str>,
        ip_address: Option<IpNetwork>,
        since: DateTime<Utc>,
    ) -> DbResult<Vec<AuthAuditLog>> {
        if let Some(email_val) = email {
            sqlx::query_as!(
                AuthAuditLog,
                r#"
                SELECT *
                FROM auth_audit_log
                WHERE event_type = 'login' 
                    AND event_status = 'failure'
                    AND email = $1
                    AND timestamp >= $2
                ORDER BY timestamp DESC
                "#,
                email_val,
                since
            )
            .fetch_all(self.pool.get())
            .await
        } else if let Some(ip) = ip_address {
            sqlx::query_as!(
                AuthAuditLog,
                r#"
                SELECT *
                FROM auth_audit_log
                WHERE event_type = 'login' 
                    AND event_status = 'failure'
                    AND ip_address = $1
                    AND timestamp >= $2
                ORDER BY timestamp DESC
                "#,
                ip,
                since
            )
            .fetch_all(self.pool.get())
            .await
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Get anomalies
    pub async fn get_anomalies(
        &self,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuthAuditLog>> {
        sqlx::query_as!(
            AuthAuditLog,
            r#"
            SELECT *
            FROM auth_audit_log
            WHERE anomaly_detected = true
            ORDER BY timestamp DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Get logs by time range
    pub async fn get_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<AuthAuditLog>> {
        sqlx::query_as!(
            AuthAuditLog,
            r#"
            SELECT *
            FROM auth_audit_log
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp DESC
            LIMIT $3 OFFSET $4
            "#,
            start,
            end,
            limit,
            offset
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Count logs by criteria
    pub async fn count_events(
        &self,
        user_id: Option<Uuid>,
        event_type: Option<&str>,
        event_status: Option<&str>,
        since: Option<DateTime<Utc>>,
    ) -> DbResult<i64> {
        let since_val = since.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
        
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM auth_audit_log
            WHERE ($1::uuid IS NULL OR user_id = $1)
                AND ($2::text IS NULL OR event_type = $2)
                AND ($3::text IS NULL OR event_status = $3)
                AND timestamp >= $4
            "#,
            user_id,
            event_type,
            event_status,
            since_val
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.count)
    }
    
    /// Delete old logs (retention policy)
    pub async fn delete_old_logs(&self, days: i32) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM auth_audit_log
            WHERE timestamp < NOW() - ($1 || ' days')::interval
            "#,
            days.to_string()
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
}
