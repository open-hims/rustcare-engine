/// OAuth account repository
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::OAuthAccount;
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use std::sync::Arc;

pub struct OAuthRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
    encryption: Option<Arc<database_layer::encryption::DatabaseEncryption>>,
}

impl OAuthRepository {
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
                ctx.user_id, &ctx.tenant_id, operation, "oauth_accounts", record_id, metadata
            ).await;
        }
    }
    
    /// Find OAuth account by provider and subject (used by OAuth provider)
    pub async fn find_by_provider_and_subject(
        &self,
        provider: &str,
        subject: &str,
    ) -> DbResult<Option<OAuthAccount>> {
        sqlx::query_as!(
            OAuthAccount,
            r#"
            SELECT *
            FROM oauth_accounts
            WHERE provider = $1 AND provider_account_id = $2
            "#,
            provider,
            subject
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Create new OAuth account (used by OAuth provider)
    pub async fn create(
        &self,
        user_id: Uuid,
        provider: &str,
        provider_account_id: &str,
        provider_email: Option<&str>,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        id_token: Option<&str>,
        token_expires_at: Option<DateTime<Utc>>,
        provider_data: Option<serde_json::Value>,
        scopes: Option<&[String]>,
    ) -> DbResult<OAuthAccount> {
        // Prepare tokens (encrypt if configured)
        let mut enc_access: Option<String> = access_token.map(|s| s.to_string());
        let mut enc_refresh: Option<String> = refresh_token.map(|s| s.to_string());
        let mut enc_id: Option<String> = id_token.map(|s| s.to_string());
        if let Some(enc) = &self.encryption {
            if let Some(a) = access_token {
                if let Ok(ct) = enc.encrypt_value(a) { enc_access = Some(ct); }
            }
            if let Some(r) = refresh_token {
                if let Ok(ct) = enc.encrypt_value(r) { enc_refresh = Some(ct); }
            }
            if let Some(i) = id_token {
                if let Ok(ct) = enc.encrypt_value(i) { enc_id = Some(ct); }
            }
        }

        let account = sqlx::query_as!(
            OAuthAccount,
            r#"
            INSERT INTO oauth_accounts (
                user_id, provider, provider_account_id, provider_email,
                access_token, refresh_token, id_token, token_expires_at,
                provider_data, scopes, first_login_at, last_login_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            RETURNING *
            "#,
            user_id,
            provider,
            provider_account_id,
            provider_email,
            enc_access,
            enc_refresh,
            enc_id,
            token_expires_at,
            provider_data,
            scopes
        )
        .fetch_one(self.pool.get())
        .await?;

        self.log_audit("create_oauth_account", Some(&user_id.to_string()), serde_json::json!({
            "provider": provider,
            "provider_account_id": provider_account_id,
            "has_refresh_token": refresh_token.is_some()
        })).await;

        Ok(account)
    }
    
    /// Create or update OAuth account
    pub async fn upsert(
        &self,
        user_id: Uuid,
        provider: &str,
        provider_account_id: &str,
        provider_email: Option<&str>,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        id_token: Option<&str>,
        token_expires_at: Option<DateTime<Utc>>,
        provider_data: Option<serde_json::Value>,
        scopes: Option<&[String]>,
    ) -> DbResult<OAuthAccount> {
        // Encrypt tokens if configured
        let mut enc_access: Option<String> = access_token.map(|s| s.to_string());
        let mut enc_refresh: Option<String> = refresh_token.map(|s| s.to_string());
        let mut enc_id: Option<String> = id_token.map(|s| s.to_string());
        if let Some(enc) = &self.encryption {
            if let Some(a) = access_token {
                if let Ok(ct) = enc.encrypt_value(a) { enc_access = Some(ct); }
            }
            if let Some(r) = refresh_token {
                if let Ok(ct) = enc.encrypt_value(r) { enc_refresh = Some(ct); }
            }
            if let Some(i) = id_token {
                if let Ok(ct) = enc.encrypt_value(i) { enc_id = Some(ct); }
            }
        }

        sqlx::query_as!(
            OAuthAccount,
            r#"
            INSERT INTO oauth_accounts (
                user_id, provider, provider_account_id, provider_email,
                access_token, refresh_token, id_token, token_expires_at,
                provider_data, scopes, first_login_at, last_login_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            ON CONFLICT (provider, provider_account_id)
            DO UPDATE SET
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                id_token = EXCLUDED.id_token,
                token_expires_at = EXCLUDED.token_expires_at,
                provider_data = EXCLUDED.provider_data,
                scopes = EXCLUDED.scopes,
                last_login_at = NOW(),
                updated_at = NOW()
            RETURNING *
            "#,
            user_id,
            provider,
            provider_account_id,
            provider_email,
            enc_access,
            enc_refresh,
            enc_id,
            token_expires_at,
            provider_data,
            scopes
        )
        .fetch_one(self.pool.get())
        .await
    }
    
    /// Find OAuth account by provider and ID
    pub async fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> DbResult<Option<OAuthAccount>> {
        sqlx::query_as!(
            OAuthAccount,
            r#"
            SELECT *
            FROM oauth_accounts
            WHERE provider = $1 AND provider_account_id = $2
            "#,
            provider,
            provider_account_id
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Find all OAuth accounts for user
    pub async fn find_by_user_id(&self, user_id: Uuid) -> DbResult<Vec<OAuthAccount>> {
        sqlx::query_as!(
            OAuthAccount,
            r#"
            SELECT *
            FROM oauth_accounts
            WHERE user_id = $1
            ORDER BY last_login_at DESC
            "#,
            user_id
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Update OAuth tokens
    pub async fn update_tokens(
        &self,
        id: Uuid,
        access_token: Option<&str>,
        refresh_token: Option<&str>,
        id_token: Option<&str>,
        token_expires_at: Option<DateTime<Utc>>,
    ) -> DbResult<OAuthAccount> {
        // Encrypt if needed
        let mut enc_access: Option<String> = access_token.map(|s| s.to_string());
        let mut enc_refresh: Option<String> = refresh_token.map(|s| s.to_string());
        let mut enc_id: Option<String> = id_token.map(|s| s.to_string());
        if let Some(enc) = &self.encryption {
            if let Some(a) = access_token { if let Ok(ct) = enc.encrypt_value(a) { enc_access = Some(ct); } }
            if let Some(r) = refresh_token { if let Ok(ct) = enc.encrypt_value(r) { enc_refresh = Some(ct); } }
            if let Some(i) = id_token { if let Ok(ct) = enc.encrypt_value(i) { enc_id = Some(ct); } }
        }

        sqlx::query_as!(
            OAuthAccount,
            r#"
            UPDATE oauth_accounts
            SET 
                access_token = $2,
                refresh_token = $3,
                id_token = $4,
                token_expires_at = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
            id,
            enc_access,
            enc_refresh,
            enc_id,
            token_expires_at
        )
        .fetch_one(self.pool.get())
        .await
    }
    
    /// Update last login time
    pub async fn update_last_login(&self, id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE oauth_accounts
            SET 
                last_login_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool.get())
        .await?;

        self.log_audit("oauth_login", Some(&id.to_string()), serde_json::json!({
            "action": "oauth_login_recorded"
        })).await;

        Ok(())
    }
    
    /// Check if OAuth account exists for user and provider
    pub async fn exists_for_user(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 
                FROM oauth_accounts 
                WHERE user_id = $1 AND provider = $2
            ) as "exists!"
            "#,
            user_id,
            provider
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.exists)
    }
    
    /// Unlink OAuth account
    pub async fn unlink(&self, id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM oauth_accounts
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Get OAuth providers for user
    pub async fn get_user_providers(&self, user_id: Uuid) -> DbResult<Vec<String>> {
        let rows = sqlx::query!(
            r#"
            SELECT provider
            FROM oauth_accounts
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_all(self.pool.get())
        .await?;
        
        Ok(rows.into_iter().map(|r| r.provider).collect())
    }
}
