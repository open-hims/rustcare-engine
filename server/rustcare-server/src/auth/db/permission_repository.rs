/// Permission repository
///
/// Now supports RLS context for multi-tenant isolation and audit logging

use crate::auth::models::UserPermission;
use super::RlsContext;
use database_layer::AuditLogger;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use std::sync::Arc;

pub type DbResult<T> = Result<T, sqlx::Error>;

#[derive(Clone)]
pub struct PermissionRepository {
    pool: PgPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl PermissionRepository {
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
                ctx.user_id, &ctx.tenant_id, operation, "user_permissions", record_id, metadata
            ).await;
        }
    }
    
    /// Grant permission to user
    pub async fn grant(
        &self,
        user_id: Uuid,
        permission: &str,
        resource_type: Option<&str>,
        resource_id: Option<Uuid>,
        granted_by: Option<Uuid>,
        expires_at: Option<DateTime<Utc>>,
    ) -> DbResult<UserPermission> {
        let perm = sqlx::query_as!(
            UserPermission,
            r#"
            INSERT INTO user_permissions (
                user_id, permission, resource_type, resource_id,
                granted_by, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            user_id,
            permission,
            resource_type,
            resource_id,
            granted_by,
            expires_at
        )
        .fetch_one(&self.pool)
        .await?;

        self.log_audit("grant_permission", Some(&user_id.to_string()), serde_json::json!({
            "permission": permission,
            "resource_type": resource_type,
            "granted_by": granted_by
        })).await;

        Ok(perm)
    }
    
    /// Revoke permission
    pub async fn revoke(&self, permission_id: Uuid) -> DbResult<()> {
        sqlx::query!(
            r#"
            DELETE FROM user_permissions
            WHERE id = $1
            "#,
            permission_id
        )
        .execute(&self.pool)
        .await?;

        self.log_audit("revoke_permission", Some(&permission_id.to_string()), serde_json::json!({
            "action": "revoke_by_id"
        })).await;

        Ok(())
    }
    
    /// Revoke specific permission from user
    pub async fn revoke_by_name(
        &self,
        user_id: Uuid,
        permission: &str,
        resource_type: Option<&str>,
        resource_id: Option<Uuid>,
    ) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_permissions
            WHERE user_id = $1 
                AND permission = $2
                AND ($3::text IS NULL OR resource_type = $3)
                AND ($4::uuid IS NULL OR resource_id = $4)
            "#,
            user_id,
            permission,
            resource_type,
            resource_id
        )
        .execute(&self.pool)
        .await?;
        
        let rows_affected = result.rows_affected();

        if rows_affected > 0 {
            self.log_audit("revoke_permission_by_name", Some(&user_id.to_string()), serde_json::json!({
                "permission": permission,
                "resource_type": resource_type,
                "permissions_revoked": rows_affected
            })).await;
        }
        
        Ok(rows_affected)
    }
    
    /// Get all permissions for user
    pub async fn get_user_permissions(&self, user_id: Uuid) -> DbResult<Vec<UserPermission>> {
        sqlx::query_as!(
            UserPermission,
            r#"
            SELECT *
            FROM user_permissions
            WHERE user_id = $1
                AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY granted_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }
    
    /// Check if user has permission
    pub async fn has_permission(
        &self,
        user_id: Uuid,
        permission: &str,
        resource_type: Option<&str>,
        resource_id: Option<Uuid>,
    ) -> DbResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 
                FROM user_permissions
                WHERE user_id = $1 
                    AND permission = $2
                    AND ($3::text IS NULL OR resource_type = $3)
                    AND ($4::uuid IS NULL OR resource_id = $4)
                    AND (expires_at IS NULL OR expires_at > NOW())
            ) as "exists!"
            "#,
            user_id,
            permission,
            resource_type,
            resource_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.exists)
    }
    
    /// Get permissions for resource
    pub async fn get_resource_permissions(
        &self,
        resource_type: &str,
        resource_id: Uuid,
    ) -> DbResult<Vec<UserPermission>> {
        sqlx::query_as!(
            UserPermission,
            r#"
            SELECT *
            FROM user_permissions
            WHERE resource_type = $1 
                AND resource_id = $2
                AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY granted_at DESC
            "#,
            resource_type,
            resource_id
        )
        .fetch_all(&self.pool)
        .await
    }
    
    /// Cleanup expired permissions
    pub async fn cleanup_expired(&self) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_permissions
            WHERE expires_at IS NOT NULL 
                AND expires_at < NOW()
            "#
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Revoke all permissions for user
    pub async fn revoke_all_user_permissions(&self, user_id: Uuid) -> DbResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_permissions
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
    
    /// Count user permissions
    pub async fn count_user_permissions(&self, user_id: Uuid) -> DbResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM user_permissions
            WHERE user_id = $1
                AND (expires_at IS NULL OR expires_at > NOW())
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.count)
    }
}
