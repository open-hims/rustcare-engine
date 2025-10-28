use crate::auth::models::{
    Organization, CreateOrganization, UpdateOrganization, SubscriptionTier, SYSTEM_ORGANIZATION_ID,
};
use super::{DbPool, DbResult, RlsContext};
use database_layer::AuditLogger;
use std::sync::Arc;
use uuid::Uuid;

/// Repository for managing organizations (multi-tenant entities)
pub struct OrganizationRepository {
    pool: DbPool,
    rls_context: Option<RlsContext>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl OrganizationRepository {
    /// Create a new OrganizationRepository
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            rls_context: None,
            audit_logger: None,
        }
    }

    /// Set the RLS context for tenant isolation
    pub fn with_rls_context(mut self, context: RlsContext) -> Self {
        self.rls_context = Some(context);
        self
    }

    /// Set the audit logger for HIPAA compliance
    pub fn with_audit_logger(mut self, logger: Arc<AuditLogger>) -> Self {
        self.audit_logger = Some(logger);
        self
    }

    /// Helper to log audit events
    async fn log_audit(
        &self,
        action: &str,
        organization_id: Option<Uuid>,
        details: serde_json::Value,
    ) {
        if let Some(logger) = &self.audit_logger {
            if let Some(ctx) = &self.rls_context {
                let _ = logger.log_operation(
                    ctx.user_id,
                    &ctx.tenant_id,
                    action,
                    "organizations",
                    organization_id.as_ref().map(|id| id.to_string()).as_deref(),
                    details,
                ).await;
            }
        }
    }

    /// Create a new organization
    pub async fn create(&self, org: CreateOrganization) -> DbResult<Organization> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (
                id, name, slug, domain, subscription_tier,
                max_users, max_storage_gb, is_active, is_verified,
                billing_email, contact_email, contact_phone,
                address_line1, address_line2, city, state, postal_code, country,
                settings, tax_id, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, NOW(), NOW()
            )
            RETURNING *
            "#,
        )
        .bind(org.id)
        .bind(&org.name)
        .bind(&org.slug)
        .bind(&org.domain)
        .bind(org.subscription_tier.to_string())
        .bind(org.max_users)
        .bind(org.max_storage_gb)
        .bind(org.is_active.unwrap_or(true))
        .bind(org.is_verified.unwrap_or(false))
        .bind(&org.billing_email)
        .bind(&org.contact_email)
        .bind(&org.contact_phone)
        .bind(&org.address_line1)
        .bind(&org.address_line2)
        .bind(&org.city)
        .bind(&org.state)
        .bind(&org.postal_code)
        .bind(&org.country)
        .bind(org.settings)
        .bind(&org.tax_id)
        .fetch_one(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "organization.created",
            Some(organization.id),
            serde_json::json!({
                "organization_id": organization.id,
                "name": organization.name,
                "slug": organization.slug,
                "subscription_tier": organization.subscription_tier,
            }),
        )
        .await;

        Ok(organization)
    }

    /// Find organization by ID
    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Organization>> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool.get())
        .await?;

        Ok(organization)
    }

    /// Find organization by slug
    pub async fn find_by_slug(&self, slug: &str) -> DbResult<Option<Organization>> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE slug = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(slug)
        .fetch_optional(self.pool.get())
        .await?;

        Ok(organization)
    }

    /// Find organization by domain
    pub async fn find_by_domain(&self, domain: &str) -> DbResult<Option<Organization>> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE domain = $1 AND deleted_at IS NULL AND is_verified = true
            "#,
        )
        .bind(domain)
        .fetch_optional(self.pool.get())
        .await?;

        Ok(organization)
    }

    /// List organizations with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<Organization>> {
        let organizations = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.get())
        .await?;

        Ok(organizations)
    }

    /// List active organizations (excluding soft-deleted)
    pub async fn list_active(&self, limit: i64, offset: i64) -> DbResult<Vec<Organization>> {
        let organizations = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations
            WHERE deleted_at IS NULL AND is_active = true
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool.get())
        .await?;

        Ok(organizations)
    }

    /// Update organization
    pub async fn update(&self, id: Uuid, update: UpdateOrganization) -> DbResult<Organization> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET
                name = COALESCE($2, name),
                domain = COALESCE($3, domain),
                is_active = COALESCE($4, is_active),
                is_verified = COALESCE($5, is_verified),
                billing_email = COALESCE($6, billing_email),
                contact_email = COALESCE($7, contact_email),
                contact_phone = COALESCE($8, contact_phone),
                address_line1 = COALESCE($9, address_line1),
                address_line2 = COALESCE($10, address_line2),
                city = COALESCE($11, city),
                state = COALESCE($12, state),
                postal_code = COALESCE($13, postal_code),
                country = COALESCE($14, country),
                settings = COALESCE($15, settings),
                tax_id = COALESCE($16, tax_id),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&update.name)
        .bind(&update.domain)
        .bind(&update.is_active)
        .bind(&update.is_verified)
        .bind(&update.billing_email)
        .bind(&update.contact_email)
        .bind(&update.contact_phone)
        .bind(&update.address_line1)
        .bind(&update.address_line2)
        .bind(&update.city)
        .bind(&update.state)
        .bind(&update.postal_code)
        .bind(&update.country)
        .bind(&update.settings)
        .bind(&update.tax_id)
        .fetch_one(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "organization.updated",
            Some(id),
            serde_json::json!({
                "organization_id": id,
                "changes": update,
            }),
        )
        .await;

        Ok(organization)
    }

    /// Update subscription tier
    pub async fn update_subscription(
        &self,
        id: Uuid,
        tier: SubscriptionTier,
        max_users: Option<i32>,
        max_storage_gb: Option<i32>,
    ) -> DbResult<Organization> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET
                subscription_tier = $2,
                max_users = COALESCE($3, max_users),
                max_storage_gb = COALESCE($4, max_storage_gb),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tier.to_string())
        .bind(max_users)
        .bind(max_storage_gb)
        .fetch_one(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "organization.subscription_updated",
            Some(id),
            serde_json::json!({
                "organization_id": id,
                "new_tier": tier.to_string(),
                "max_users": max_users,
                "max_storage_gb": max_storage_gb,
            }),
        )
        .await;

        Ok(organization)
    }

    /// Verify organization (e.g., after email/domain verification)
    pub async fn verify_organization(&self, id: Uuid) -> DbResult<Organization> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET is_verified = true, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "organization.verified",
            Some(id),
            serde_json::json!({
                "organization_id": id,
            }),
        )
        .await;

        Ok(organization)
    }

    /// Soft delete organization
    pub async fn soft_delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE organizations
            SET deleted_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .execute(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "organization.deleted",
            Some(id),
            serde_json::json!({
                "organization_id": id,
                "deletion_type": "soft",
            }),
        )
        .await;

        Ok(())
    }

    /// Restore soft-deleted organization
    pub async fn restore(&self, id: Uuid) -> DbResult<Organization> {
        let organization = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET deleted_at = NULL, updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(self.pool.get())
        .await?;

        // Audit log
        self.log_audit(
            "organization.restored",
            Some(id),
            serde_json::json!({
                "organization_id": id,
            }),
        )
        .await;

        Ok(organization)
    }

    /// Get user count for organization
    pub async fn get_user_count(&self, organization_id: Uuid) -> DbResult<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM users
            WHERE organization_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(organization_id)
        .fetch_one(self.pool.get())
        .await?;

        Ok(count.0)
    }

    /// Get storage usage for organization (placeholder - implement based on your storage tracking)
    pub async fn get_storage_usage(&self, _organization_id: Uuid) -> DbResult<i64> {
        // TODO: Implement actual storage calculation based on your file storage implementation
        // For now, return 0 as placeholder
        Ok(0)
    }

    /// Check if organization has reached user limit
    pub async fn has_reached_user_limit(&self, organization_id: Uuid) -> DbResult<bool> {
        let org = self
            .find_by_id(organization_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let user_count = self.get_user_count(organization_id).await?;

        Ok(user_count >= org.max_users as i64)
    }

    /// Check if organization has reached storage limit
    pub async fn has_reached_storage_limit(&self, organization_id: Uuid) -> DbResult<bool> {
        let org = self
            .find_by_id(organization_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let storage_usage = self.get_storage_usage(organization_id).await?;

        // Convert GB to bytes for comparison (assuming storage_usage is in bytes)
        let limit_bytes = org.max_storage_gb as i64 * 1024 * 1024 * 1024;

        Ok(storage_usage >= limit_bytes)
    }

    /// Get system organization (for global resources)
    pub async fn get_system_organization(&self) -> DbResult<Option<Organization>> {
        self.find_by_id(SYSTEM_ORGANIZATION_ID).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add unit tests for OrganizationRepository
    // - Test create organization
    // - Test find by id/slug/domain
    // - Test list with pagination
    // - Test update operations
    // - Test subscription tier changes
    // - Test soft delete and restore
    // - Test user/storage limit checks
}
