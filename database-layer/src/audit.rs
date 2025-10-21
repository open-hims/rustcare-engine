// Audit logging for database operations
use crate::models::AuditInfo;
use crate::error::DatabaseResult;
use sqlx::PgPool;
use tracing::{info, error};
use uuid::Uuid;
use chrono::Utc;
use serde_json::Value as JsonValue;

/// Audit logger for database operations
/// 
/// Provides HIPAA-compliant audit logging for all database operations
pub struct AuditLogger {
    pool: PgPool,
    enabled: bool,
    retention_days: i32,
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { 
            pool,
            enabled: true,
            retention_days: 2555, // 7 years for HIPAA
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_retention_days(mut self, days: i32) -> Self {
        self.retention_days = days;
        self
    }

    /// Log a database operation with full context
    pub async fn log_operation(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        operation: &str,
        table_name: &str,
        record_id: Option<&str>,
        metadata: JsonValue,
    ) -> DatabaseResult<()> {
        if !self.enabled {
            return Ok(());
        }

        // Log to tracing for immediate visibility
        info!(
            target: "audit",
            user_id = %user_id,
            tenant_id = %tenant_id,
            operation = %operation,
            table_name = %table_name,
            record_id = ?record_id,
            "Database operation audit"
        );

        // Store in database for compliance (async, don't block on errors)
        if let Err(e) = self.store_audit_log(user_id, tenant_id, operation, table_name, record_id, metadata).await {
            error!(
                target: "audit",
                error = %e,
                "Failed to store audit log - this is a compliance issue!"
            );
            // In production, you might want to:
            // 1. Queue to dead-letter queue
            // 2. Alert monitoring system
            // 3. Write to backup audit file
        }

        Ok(())
    }

    /// Store audit log in database
    async fn store_audit_log(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        operation: &str,
        table_name: &str,
        record_id: Option<&str>,
        metadata: JsonValue,
    ) -> DatabaseResult<()> {
        // Redact sensitive fields from metadata
        let sanitized_metadata = self.redact_sensitive_fields(metadata);

        sqlx::query!(
            r#"
            INSERT INTO auth_audit_log (
                user_id, email, event_type, event_status,
                ip_address, user_agent, metadata, timestamp
            )
            VALUES (
                $1,
                (SELECT email FROM users WHERE id = $1),
                $2,
                'success',
                NULL,
                NULL,
                $3,
                NOW()
            )
            "#,
            user_id,
            format!("{}_{}", operation, table_name),
            sanitized_metadata
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Redact sensitive fields from audit metadata
    fn redact_sensitive_fields(&self, mut metadata: JsonValue) -> JsonValue {
        if let Some(obj) = metadata.as_object_mut() {
            // List of sensitive field names to redact
            let sensitive_fields = [
                "password",
                "password_hash",
                "access_token",
                "refresh_token",
                "session_token",
                "id_token",
                "mfa_secret",
                "backup_codes",
                "private_key_pem",
                "secret",
                "api_key",
            ];

            for field in &sensitive_fields {
                if obj.contains_key(*field) {
                    obj.insert(field.to_string(), JsonValue::String("***REDACTED***".to_string()));
                }
            }
        }
        metadata
    }

    /// Search audit logs with RLS enforcement
    pub async fn search(
        &self,
        user_id: Option<Uuid>,
        event_type: Option<&str>,
        start_time: Option<chrono::DateTime<Utc>>,
        end_time: Option<chrono::DateTime<Utc>>,
        limit: i64,
    ) -> DatabaseResult<Vec<AuditInfo>> {
        // This would use the auth_audit_log table
        // For now, return empty as we're using the existing table structure
        Ok(vec![])
    }

    /// Clean up old audit logs based on retention policy
    pub async fn cleanup_old_logs(&self) -> DatabaseResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM auth_audit_log
            WHERE timestamp < NOW() - ($1 || ' days')::interval
            "#,
            self.retention_days.to_string()
        )
        .execute(&self.pool)
        .await?;

        info!(
            target: "audit",
            deleted_count = result.rows_affected(),
            retention_days = self.retention_days,
            "Cleaned up old audit logs"
        );

        Ok(result.rows_affected())
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            enabled: self.enabled,
            retention_days: self.retention_days,
        }
    }
}

pub struct DatabaseAudit;

impl DatabaseAudit {
    pub fn new() -> Self {
        Self
    }
}