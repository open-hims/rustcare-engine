/// Credential repository for email/password authentication
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::UserCredential;
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use std::sync::Arc;

pub struct CredentialRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
    encryption: Option<Arc<database_layer::encryption::DatabaseEncryption>>,
}

impl CredentialRepository {
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
                ctx.user_id, &ctx.tenant_id, operation, "user_credentials", record_id, metadata
            ).await;
        }
    }
    
    /// Create user credentials
    pub async fn create(
        &self,
        user_id: Uuid,
        password_hash: &str,
        algorithm: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> DbResult<UserCredential> {
        let credential = sqlx::query_as!(
            UserCredential,
            r#"
            INSERT INTO user_credentials (
                user_id, password_hash, password_algorithm,
                password_changed_at, password_expires_at
            )
            VALUES ($1, $2, $3, NOW(), $4)
            RETURNING *
            "#,
            user_id,
            password_hash,
            algorithm,
            expires_at
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("create_credential", Some(&user_id.to_string()), serde_json::json!({
            "algorithm": algorithm,
            "has_expiry": expires_at.is_some()
        })).await;

        Ok(credential)
    }
    
    /// Find credentials by user ID
    pub async fn find_by_user_id(&self, user_id: Uuid) -> DbResult<Option<UserCredential>> {
        sqlx::query_as!(
            UserCredential,
            r#"
            SELECT *
            FROM user_credentials
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Update password hash
    pub async fn update_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
        algorithm: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> DbResult<UserCredential> {
        let credential = sqlx::query_as!(
            UserCredential,
            r#"
            UPDATE user_credentials
            SET 
                password_hash = $2,
                password_algorithm = $3,
                password_changed_at = NOW(),
                password_expires_at = $4,
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
            user_id,
            password_hash,
            algorithm,
            expires_at
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("update_password", Some(&user_id.to_string()), serde_json::json!({
            "algorithm": algorithm,
            "has_expiry": expires_at.is_some()
        })).await;

        Ok(credential)
    }
    
    /// Enable MFA
    pub async fn enable_mfa(
        &self,
        user_id: Uuid,
        mfa_secret: &str,
        backup_codes: &[String],
    ) -> DbResult<UserCredential> {
        // Encrypt mfa_secret if configured
        let mut enc_secret = mfa_secret.to_string();
        if let Some(enc) = &self.encryption {
            if let Ok(ct) = enc.encrypt_value(mfa_secret) { enc_secret = ct; }
        }

        let credential = sqlx::query_as!(
            UserCredential,
            r#"
            UPDATE user_credentials
            SET 
                mfa_enabled = true,
                mfa_secret = $2,
                mfa_backup_codes = $3,
                mfa_enabled_at = NOW(),
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
            user_id,
            enc_secret,
            backup_codes
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("enable_mfa", Some(&user_id.to_string()), serde_json::json!({
            "backup_codes_count": backup_codes.len()
        })).await;

        Ok(credential)
    }
    
    /// Disable MFA
    pub async fn disable_mfa(&self, user_id: Uuid) -> DbResult<UserCredential> {
        let credential = sqlx::query_as!(
            UserCredential,
            r#"
            UPDATE user_credentials
            SET 
                mfa_enabled = false,
                mfa_secret = NULL,
                mfa_backup_codes = NULL,
                mfa_enabled_at = NULL,
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
            user_id
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("disable_mfa", Some(&user_id.to_string()), serde_json::json!({
            "action": "mfa_disabled"
        })).await;

        Ok(credential)
    }
    
    /// Use MFA backup code
    pub async fn use_backup_code(
        &self,
        user_id: Uuid,
        used_code: &str,
    ) -> DbResult<Option<UserCredential>> {
        // First, get current backup codes
        let cred = self.find_by_user_id(user_id).await?;
        
        if let Some(mut credential) = cred {
            if let Some(codes) = credential.mfa_backup_codes {
                // Check if code exists
                if !codes.contains(&used_code.to_string()) {
                    return Ok(None);
                }
                
                // Remove the used code
                let remaining_codes: Vec<String> = codes
                    .into_iter()
                    .filter(|c| c != used_code)
                    .collect();
                
                // Update database
                let updated = sqlx::query_as!(
                    UserCredential,
                    r#"
                    UPDATE user_credentials
                    SET 
                        mfa_backup_codes = $2,
                        updated_at = NOW()
                    WHERE user_id = $1
                    RETURNING *
                    "#,
                    user_id,
                    &remaining_codes
                )
                .fetch_one(self.pool.get())
                .await?;
                
                return Ok(Some(updated));
            }
        }
        
        Ok(None)
    }
    
    /// Update security questions
    pub async fn update_security_questions(
        &self,
        user_id: Uuid,
        questions: serde_json::Value,
    ) -> DbResult<UserCredential> {
        sqlx::query_as!(
            UserCredential,
            r#"
            UPDATE user_credentials
            SET 
                security_questions = $2,
                updated_at = NOW()
            WHERE user_id = $1
            RETURNING *
            "#,
            user_id,
            questions
        )
        .fetch_one(self.pool.get())
        .await
    }
    
    /// Check if password is expired
    pub async fn is_password_expired(&self, user_id: Uuid) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT 
                password_expires_at IS NOT NULL 
                AND password_expires_at < NOW() as "expired!"
            FROM user_credentials
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(self.pool.get())
        .await?;
        
        Ok(result.map(|r| r.expired).unwrap_or(false))
    }
    
    /// Delete credentials (cascade when user is deleted)
    pub async fn delete(&self, user_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM user_credentials
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
}
