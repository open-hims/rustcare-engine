/// Certificate repository for mTLS authentication
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::ClientCertificate;
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use std::sync::Arc;

pub struct CertificateRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
    encryption: Option<Arc<database_layer::encryption::DatabaseEncryption>>,
}

impl CertificateRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { 
            pool,
            rls_context: None,
            audit_logger: None,
            encryption: None,
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

    pub fn with_encryption(mut self, enc: Arc<database_layer::encryption::DatabaseEncryption>) -> Self {
        self.encryption = Some(enc);
        self
    }

    pub fn rls_context(&self) -> Option<&RlsContext> {
        self.rls_context.as_ref()
    }

    async fn log_audit(&self, operation: &str, record_id: Option<&str>, metadata: serde_json::Value) {
        if let (Some(logger), Some(ctx)) = (&self.audit_logger, &self.rls_context) {
            let _ = logger.log_operation(
                ctx.user_id, &ctx.tenant_id, operation, "client_certificates", record_id, metadata
            ).await;
        }
    }
    
    /// Register a new certificate
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        serial_number: &str,
        fingerprint_sha256: &str,
        subject_dn: &str,
        issuer_dn: &str,
        common_name: Option<&str>,
        email_address: Option<&str>,
        organization: Option<&str>,
        organizational_unit: Option<&str>,
        not_before: DateTime<Utc>,
        not_after: DateTime<Utc>,
        certificate_pem: &str,
        public_key_pem: Option<&str>,
    ) -> DbResult<ClientCertificate> {
        let cert = sqlx::query_as!(
            ClientCertificate,
            r#"
            INSERT INTO client_certificates (
                user_id, organization_id, serial_number, fingerprint_sha256,
                subject_dn, issuer_dn, common_name, email_address,
                organization, organizational_unit,
                not_before, not_after, certificate_pem, public_key_pem,
                status, first_login_at, last_login_at, login_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'active', NOW(), NOW(), 0)
            RETURNING 
                id, organization_id, user_id, serial_number, fingerprint_sha256,
                subject_dn, issuer_dn, common_name, email_address,
                organization, organizational_unit,
                not_before, not_after,
                status as "status: _",
                revoked_at, revocation_reason,
                certificate_pem, public_key_pem,
                first_login_at, last_login_at, login_count,
                created_at, updated_at
            "#,
            user_id,
            organization_id,
            serial_number,
            fingerprint_sha256,
            subject_dn,
            issuer_dn,
            common_name,
            email_address,
            organization,
            organizational_unit,
            not_before,
            not_after,
            certificate_pem,
            public_key_pem
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("register_certificate", Some(&user_id.to_string()), serde_json::json!({
            "serial_number": serial_number,
            "common_name": common_name,
            "issuer_dn": issuer_dn
        })).await;

        Ok(cert)
    }
    
    /// Find certificate by serial number
    pub async fn find_by_serial(&self, serial_number: &str) -> DbResult<Option<ClientCertificate>> {
        sqlx::query_as!(
            ClientCertificate,
            r#"
            SELECT 
                id, organization_id, user_id, serial_number, fingerprint_sha256,
                subject_dn, issuer_dn, common_name, email_address,
                organization, organizational_unit,
                not_before, not_after,
                status as "status: _",
                revoked_at, revocation_reason,
                certificate_pem, public_key_pem,
                first_login_at, last_login_at, login_count,
                created_at, updated_at
            FROM client_certificates
            WHERE serial_number = $1
            "#,
            serial_number
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Find certificate by fingerprint
    pub async fn find_by_fingerprint(&self, fingerprint: &str) -> DbResult<Option<ClientCertificate>> {
        sqlx::query_as!(
            ClientCertificate,
            r#"
            SELECT 
                id, organization_id, user_id, serial_number, fingerprint_sha256,
                subject_dn, issuer_dn, common_name, email_address,
                organization, organizational_unit,
                not_before, not_after,
                status as "status: _",
                revoked_at, revocation_reason,
                certificate_pem, public_key_pem,
                first_login_at, last_login_at, login_count,
                created_at, updated_at
            FROM client_certificates
            WHERE fingerprint_sha256 = $1
            "#,
            fingerprint
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Get all certificates for user
    pub async fn find_by_user_id(&self, user_id: Uuid) -> DbResult<Vec<ClientCertificate>> {
        sqlx::query_as!(
            ClientCertificate,
            r#"
            SELECT 
                id, organization_id, user_id, serial_number, fingerprint_sha256,
                subject_dn, issuer_dn, common_name, email_address,
                organization, organizational_unit,
                not_before, not_after,
                status as "status: _",
                revoked_at, revocation_reason,
                certificate_pem, public_key_pem,
                first_login_at, last_login_at, login_count,
                created_at, updated_at
            FROM client_certificates
            WHERE user_id = $1
            ORDER BY last_login_at DESC
            "#,
            user_id
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Update last login
    pub async fn update_last_login(&self, id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE client_certificates
            SET 
                last_login_at = NOW(),
                login_count = login_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Revoke certificate
    pub async fn revoke(
        &self,
        id: Uuid,
        reason: &str,
    ) -> DbResult<ClientCertificate> {
        let cert = sqlx::query_as!(
            ClientCertificate,
            r#"
            UPDATE client_certificates
            SET 
                status = 'revoked',
                revoked_at = NOW(),
                revocation_reason = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, organization_id, user_id, serial_number, fingerprint_sha256,
                subject_dn, issuer_dn, common_name, email_address,
                organization, organizational_unit,
                not_before, not_after,
                status as "status: _",
                revoked_at, revocation_reason,
                certificate_pem, public_key_pem,
                first_login_at, last_login_at, login_count,
                created_at, updated_at
            "#,
            id,
            reason
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("revoke_certificate", Some(&id.to_string()), serde_json::json!({
            "reason": reason,
            "serial_number": &cert.serial_number
        })).await;

        Ok(cert)
    }
    
    /// Check if certificate is revoked
    pub async fn is_revoked(&self, serial_number: &str) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT status
            FROM client_certificates
            WHERE serial_number = $1
            "#,
            serial_number
        )
        .fetch_optional(self.pool.get())
        .await?;
        
        Ok(result.map(|r| r.status == "revoked").unwrap_or(false))
    }
    
    /// Mark expired certificates
    pub async fn mark_expired(&self) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            UPDATE client_certificates
            SET 
                status = 'expired',
                updated_at = NOW()
            WHERE not_after < NOW() 
                AND status = 'active'
            "#
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Get active certificates count for user
    pub async fn count_active_for_user(&self, user_id: Uuid) -> DbResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM client_certificates
            WHERE user_id = $1 
                AND status = 'active'
                AND not_before <= NOW()
                AND not_after > NOW()
            "#,
            user_id
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.count)
    }
    
    /// Delete certificate
    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM client_certificates
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
}
