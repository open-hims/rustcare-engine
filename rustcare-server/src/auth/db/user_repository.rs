/// User repository for database operations
/// 
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::{User, UserStatus, UserWithAuthMethods};
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use sqlx::types::{chrono::{DateTime, Utc}, Uuid};
use ipnetwork::IpNetwork;
use std::sync::Arc;

pub struct UserRepository {
    pool: DbPool,
    /// Optional RLS context for automatic tenant isolation
    rls_context: Option<RlsContext>,
    /// Optional audit logger for HIPAA compliance
    audit_logger: Option<Arc<AuditLogger>>,
}

impl UserRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { 
            pool,
            rls_context: None,
            audit_logger: None,
        }
    }

    /// Create with RLS context for multi-tenant operations
    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    /// Add audit logging
    pub fn with_audit_logger(mut self, logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// Get current RLS context
    pub fn rls_context(&self) -> Option<&RlsContext> {
        self.rls_context.as_ref()
    }

    /// Log audit event if logger is configured
    async fn log_audit(
        &self,
        operation: &str,
        record_id: Option<&str>,
        metadata: serde_json::Value,
    ) {
        if let (Some(logger), Some(ctx)) = (&self.audit_logger, &self.rls_context) {
            let _ = logger.log_operation(
                ctx.user_id,
                &ctx.tenant_id,
                operation,
                "users",
                record_id,
                metadata,
            ).await;
        }
    }
    
    /// Create a new user
    pub async fn create(
        &self,
        email: &str,
        full_name: Option<&str>,
        display_name: Option<&str>,
    ) -> DbResult<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (email, full_name, display_name, status)
            VALUES ($1, $2, $3, 'active')
            RETURNING 
                id, email, email_verified, email_verified_at, 
                full_name, display_name, avatar_url,
                status as "status: _",
                locale, timezone, 
                last_login_at, last_login_ip, last_login_method,
                failed_login_attempts, locked_until,
                created_at, updated_at, deleted_at
            "#,
            email,
            full_name,
            display_name
        )
        .fetch_one(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "CREATE",
            Some(&user.id.to_string()),
            serde_json::json!({
                "email": email,
                "full_name": full_name,
                "display_name": display_name,
            })
        ).await;

        Ok(user)
    }
    
    /// Find user by ID
    pub async fn find_by_id(&self, user_id: Uuid) -> DbResult<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT 
                id, email, email_verified, email_verified_at, 
                full_name, display_name, avatar_url,
                status as "status: _",
                locale, timezone, 
                last_login_at, last_login_ip, last_login_method,
                failed_login_attempts, locked_until,
                created_at, updated_at, deleted_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
            user_id
        )
        .fetch_optional(self.pool.get())
        .await?;

        // Audit log for sensitive data access
        if user.is_some() {
            self.log_audit(
                "SELECT",
                Some(&user_id.to_string()),
                serde_json::json!({
                    "query_type": "find_by_id",
                })
            ).await;
        }

        Ok(user)
    }
    
    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> DbResult<Option<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT 
                id, email, email_verified, email_verified_at, 
                full_name, display_name, avatar_url,
                status as "status: _",
                locale, timezone, 
                last_login_at, last_login_ip, last_login_method,
                failed_login_attempts, locked_until,
                created_at, updated_at, deleted_at
            FROM users
            WHERE email = $1 AND deleted_at IS NULL
            "#,
            email
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Update user's last login information
    pub async fn update_last_login(
        &self,
        user_id: Uuid,
        ip_address: Option<IpNetwork>,
        auth_method: &str,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET 
                last_login_at = NOW(),
                last_login_ip = $2,
                last_login_method = $3,
                failed_login_attempts = 0,
                locked_until = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            ip_address,
            auth_method
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Increment failed login attempts
    pub async fn increment_failed_login(&self, user_id: Uuid) -> DbResult<i32> {
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET 
                failed_login_attempts = failed_login_attempts + 1,
                updated_at = NOW()
            WHERE id = $1
            RETURNING failed_login_attempts
            "#,
            user_id
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.failed_login_attempts)
    }
    
    /// Lock user account
    pub async fn lock_account(
        &self,
        user_id: Uuid,
        lock_duration_minutes: i32,
    ) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET 
                status = 'locked',
                locked_until = NOW() + ($2 || ' minutes')::interval,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            lock_duration_minutes.to_string()
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Unlock user account if lock has expired
    pub async fn check_and_unlock(&self, user_id: Uuid) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            UPDATE users
            SET 
                status = 'active',
                locked_until = NULL,
                failed_login_attempts = 0,
                updated_at = NOW()
            WHERE id = $1 
                AND status = 'locked' 
                AND locked_until IS NOT NULL 
                AND locked_until < NOW()
            "#,
            user_id
        )
        .execute(self.pool.get())
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    /// Update user status
    pub async fn update_status(&self, user_id: Uuid, status: UserStatus) -> DbResult<()> {
        let status_str = status.to_string();
        sqlx::query!(
            r#"
            UPDATE users
            SET 
                status = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id,
            status_str
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Mark email as verified
    pub async fn verify_email(&self, user_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET 
                email_verified = true,
                email_verified_at = NOW(),
                status = CASE 
                    WHEN status = 'pending_verification' THEN 'active'
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Update user profile
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        full_name: Option<&str>,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
        locale: Option<&str>,
        timezone: Option<&str>,
    ) -> DbResult<User> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET 
                full_name = COALESCE($2, full_name),
                display_name = COALESCE($3, display_name),
                avatar_url = COALESCE($4, avatar_url),
                locale = COALESCE($5, locale),
                timezone = COALESCE($6, timezone),
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, email, email_verified, email_verified_at, 
                full_name, display_name, avatar_url,
                status as "status: _",
                locale, timezone, 
                last_login_at, last_login_ip, last_login_method,
                failed_login_attempts, locked_until,
                created_at, updated_at, deleted_at
            "#,
            user_id,
            full_name,
            display_name,
            avatar_url,
            locale,
            timezone
        )
        .fetch_one(self.pool.get())
        .await
    }
    
    /// Soft delete user
    pub async fn soft_delete(&self, user_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            UPDATE users
            SET 
                deleted_at = NOW(),
                status = 'inactive',
                updated_at = NOW()
            WHERE id = $1
            "#,
            user_id
        )
        .execute(self.pool.get())
        .await?;
        Ok(())
    }
    
    /// Get user with authentication methods
    pub async fn get_with_auth_methods(&self, user_id: Uuid) -> DbResult<Option<UserWithAuthMethods>> {
        sqlx::query_as!(
            UserWithAuthMethods,
            r#"
            SELECT 
                user_id,
                email,
                has_password,
                oauth_providers,
                active_certificates
            FROM user_auth_methods
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(self.pool.get())
        .await
    }
    
    /// Get all active users
    pub async fn list_active(&self, limit: i64, offset: i64) -> DbResult<Vec<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT 
                id, email, email_verified, email_verified_at, 
                full_name, display_name, avatar_url,
                status as "status: _",
                locale, timezone, 
                last_login_at, last_login_ip, last_login_method,
                failed_login_attempts, locked_until,
                created_at, updated_at, deleted_at
            FROM active_users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit,
            offset
        )
        .fetch_all(self.pool.get())
        .await
    }
    
    /// Count all active users
    pub async fn count_active(&self) -> DbResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM active_users
            "#
        )
        .fetch_one(self.pool.get())
        .await?;
        
        Ok(result.count)
    }
}
