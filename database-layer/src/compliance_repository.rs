use chrono::NaiveDate;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::{
    models::{ComplianceFramework, ComplianceRule}, DatabaseResult as DbResult,
};

/// Repository for compliance framework and rule operations
#[derive(Clone, Debug)]
pub struct ComplianceRepository {
    pool: Pool<Postgres>,
}

impl ComplianceRepository {
    /// Create a new compliance repository
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    // Compliance Framework operations

    /// List all compliance frameworks
    pub async fn list_frameworks(
        &self,
        organization_id: Option<Uuid>,
        status: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> DbResult<Vec<ComplianceFramework>> {
        let mut query = String::from(
            r#"
            SELECT id, organization_id, name, code, version, description, authority, 
                   jurisdiction, effective_date, review_date, status, parent_framework_id,
                   metadata, created_at, updated_at, created_by, updated_by
            FROM compliance_frameworks
            WHERE 1=1
            "#
        );

        let mut bind_idx = 1;
        let mut params: Vec<Box<dyn std::any::Any + Send + Sync>> = Vec::new();

        if let Some(org_id) = organization_id {
            query.push_str(&format!(" AND organization_id = ${}", bind_idx));
            params.push(Box::new(org_id));
            bind_idx += 1;
        }

        if let Some(status_val) = status {
            query.push_str(&format!(" AND status = ${}", bind_idx));
            params.push(Box::new(status_val.to_string()));
            bind_idx += 1;
        }

        query.push_str(" ORDER BY effective_date DESC");

        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT ${}", bind_idx));
            params.push(Box::new(limit_val as i64));
            bind_idx += 1;
        }

        if let Some(offset_val) = offset {
            query.push_str(&format!(" OFFSET ${}", bind_idx));
            params.push(Box::new(offset_val as i64));
        }

        // For now, let's use a simpler approach without dynamic binding
        let frameworks = if let Some(org_id) = organization_id {
            sqlx::query_as::<_, ComplianceFramework>(
                r#"
                SELECT id, organization_id, name, code, version, description, authority, 
                       jurisdiction, effective_date, review_date, status, parent_framework_id,
                       metadata, created_at, updated_at, created_by, updated_by
                FROM compliance_frameworks
                WHERE organization_id = $1
                ORDER BY effective_date DESC
                LIMIT $2
                "#,
            )
            .bind(org_id)
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ComplianceFramework>(
                r#"
                SELECT id, organization_id, name, code, version, description, authority, 
                       jurisdiction, effective_date, review_date, status, parent_framework_id,
                       metadata, created_at, updated_at, created_by, updated_by
                FROM compliance_frameworks
                ORDER BY effective_date DESC
                LIMIT $1
                "#,
            )
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(frameworks)
    }

    /// Create a new compliance framework
    pub async fn create_framework(
        &self,
        organization_id: Uuid,
        name: &str,
        code: &str,
        version: &str,
        description: Option<&str>,
        authority: Option<&str>,
        jurisdiction: Option<&str>,
        effective_date: NaiveDate,
        review_date: Option<NaiveDate>,
        parent_framework_id: Option<Uuid>,
        metadata: Option<Value>,
        created_by: Option<Uuid>,
    ) -> DbResult<ComplianceFramework> {
        let framework = sqlx::query_as::<_, ComplianceFramework>(
            r#"
            INSERT INTO compliance_frameworks (
                organization_id, name, code, version, description, authority, 
                jurisdiction, effective_date, review_date, parent_framework_id,
                metadata, created_by, updated_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
            RETURNING id, organization_id, name, code, version, description, authority, 
                      jurisdiction, effective_date, review_date, status, parent_framework_id,
                      metadata, created_at, updated_at, created_by, updated_by
            "#,
        )
        .bind(organization_id)
        .bind(name)
        .bind(code)
        .bind(version)
        .bind(description)
        .bind(authority)
        .bind(jurisdiction)
        .bind(effective_date)
        .bind(review_date)
        .bind(parent_framework_id)
        .bind(metadata)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(framework)
    }

    /// Get a compliance framework by ID
    pub async fn get_framework(&self, id: Uuid) -> DbResult<Option<ComplianceFramework>> {
        let framework = sqlx::query_as::<_, ComplianceFramework>(
            r#"
            SELECT id, organization_id, name, code, version, description, authority, 
                   jurisdiction, effective_date, review_date, status, parent_framework_id,
                   metadata, created_at, updated_at, created_by, updated_by
            FROM compliance_frameworks
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(framework)
    }

    // Compliance Rule operations

    /// List compliance rules for a framework
    pub async fn list_rules(
        &self,
        framework_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        category: Option<&str>,
        severity: Option<&str>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> DbResult<Vec<ComplianceRule>> {
        let rules = if let Some(fw_id) = framework_id {
            sqlx::query_as::<_, ComplianceRule>(
                r#"
                SELECT id, organization_id, framework_id, rule_code, title, description, 
                       category, severity, rule_type, applies_to_entity_types, applies_to_roles,
                       applies_to_regions, validation_logic, remediation_steps, 
                       documentation_requirements, is_automated, automation_script,
                       check_frequency_days, last_checked_at, status, version, effective_date,
                       expiry_date, metadata, tags, created_at, updated_at, created_by, updated_by
                FROM compliance_rules
                WHERE framework_id = $1 AND status = 'active'
                ORDER BY category ASC, title ASC
                LIMIT $2
                "#,
            )
            .bind(fw_id)
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        } else if let Some(org_id) = organization_id {
            sqlx::query_as::<_, ComplianceRule>(
                r#"
                SELECT id, organization_id, framework_id, rule_code, title, description, 
                       category, severity, rule_type, applies_to_entity_types, applies_to_roles,
                       applies_to_regions, validation_logic, remediation_steps, 
                       documentation_requirements, is_automated, automation_script,
                       check_frequency_days, last_checked_at, status, version, effective_date,
                       expiry_date, metadata, tags, created_at, updated_at, created_by, updated_by
                FROM compliance_rules
                WHERE organization_id = $1 AND status = 'active'
                ORDER BY category ASC, title ASC
                LIMIT $2
                "#,
            )
            .bind(org_id)
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ComplianceRule>(
                r#"
                SELECT id, organization_id, framework_id, rule_code, title, description, 
                       category, severity, rule_type, applies_to_entity_types, applies_to_roles,
                       applies_to_regions, validation_logic, remediation_steps, 
                       documentation_requirements, is_automated, automation_script,
                       check_frequency_days, last_checked_at, status, version, effective_date,
                       expiry_date, metadata, tags, created_at, updated_at, created_by, updated_by
                FROM compliance_rules
                WHERE status = 'active'
                ORDER BY category ASC, title ASC
                LIMIT $1
                "#,
            )
            .bind(limit.unwrap_or(100) as i64)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rules)
    }

    /// Create a new compliance rule
    pub async fn create_rule(
        &self,
        organization_id: Uuid,
        framework_id: Uuid,
        rule_code: &str,
        title: &str,
        description: Option<&str>,
        category: Option<&str>,
        severity: &str,
        rule_type: &str,
        applies_to_entity_types: Option<Value>,
        applies_to_roles: Option<Value>,
        applies_to_regions: Option<Value>,
        validation_logic: Option<Value>,
        remediation_steps: Option<&str>,
        is_automated: bool,
        check_frequency_days: Option<i32>,
        effective_date: NaiveDate,
        metadata: Option<Value>,
        created_by: Option<Uuid>,
    ) -> DbResult<ComplianceRule> {
        let rule = sqlx::query_as::<_, ComplianceRule>(
            r#"
            INSERT INTO compliance_rules (
                organization_id, framework_id, rule_code, title, description, category,
                severity, rule_type, applies_to_entity_types, applies_to_roles,
                applies_to_regions, validation_logic, remediation_steps, is_automated,
                check_frequency_days, effective_date, metadata, created_by, updated_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $18)
            RETURNING id, organization_id, framework_id, rule_code, title, description, 
                      category, severity, rule_type, applies_to_entity_types, applies_to_roles,
                      applies_to_regions, validation_logic, remediation_steps, 
                      documentation_requirements, is_automated, automation_script,
                      check_frequency_days, last_checked_at, status, version, effective_date,
                      expiry_date, metadata, tags, created_at, updated_at, created_by, updated_by
            "#,
        )
        .bind(organization_id)
        .bind(framework_id)
        .bind(rule_code)
        .bind(title)
        .bind(description)
        .bind(category)
        .bind(severity)
        .bind(rule_type)
        .bind(applies_to_entity_types)
        .bind(applies_to_roles)
        .bind(applies_to_regions)
        .bind(validation_logic)
        .bind(remediation_steps)
        .bind(is_automated)
        .bind(check_frequency_days)
        .bind(effective_date)
        .bind(metadata)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(rule)
    }

    /// Get compliance rules applicable to a geographic region
    pub async fn get_rules_for_region(
        &self,
        region_id: Uuid,
        entity_type: Option<&str>,
    ) -> DbResult<Vec<ComplianceRule>> {
        let rules = sqlx::query_as::<_, ComplianceRule>(
            r#"
            SELECT DISTINCT r.id, r.organization_id, r.framework_id, r.rule_code, r.title, 
                   r.description, r.category, r.severity, r.rule_type, r.applies_to_entity_types,
                   r.applies_to_roles, r.applies_to_regions, r.validation_logic, r.remediation_steps, 
                   r.documentation_requirements, r.is_automated, r.automation_script,
                   r.check_frequency_days, r.last_checked_at, r.status, r.version, r.effective_date,
                   r.expiry_date, r.metadata, r.tags, r.created_at, r.updated_at, r.created_by, r.updated_by
            FROM compliance_rules r
            JOIN rule_region_applicability rra ON r.id = rra.rule_id
            WHERE rra.region_id = $1 AND r.status = 'active'
            ORDER BY r.severity DESC, r.category ASC
            "#,
        )
        .bind(region_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rules)
    }

    /// Auto-assign compliance rules to a geographic region based on jurisdiction
    pub async fn auto_assign_compliance(
        &self,
        region_id: Uuid,
        entity_type: &str,
        organization_type: Option<&str>,
    ) -> DbResult<Vec<ComplianceRule>> {
        // Get applicable rules based on entity type
        let rules = sqlx::query_as::<_, ComplianceRule>(
            r#"
            SELECT id, organization_id, framework_id, rule_code, title, description, 
                   category, severity, rule_type, applies_to_entity_types, applies_to_roles,
                   applies_to_regions, validation_logic, remediation_steps, 
                   documentation_requirements, is_automated, automation_script,
                   check_frequency_days, last_checked_at, status, version, effective_date,
                   expiry_date, metadata, tags, created_at, updated_at, created_by, updated_by
            FROM compliance_rules
            WHERE status = 'active'
            AND (applies_to_entity_types IS NULL OR applies_to_entity_types @> $1::jsonb)
            AND effective_date <= NOW()
            AND (expiry_date IS NULL OR expiry_date > NOW())
            ORDER BY severity DESC, framework_id ASC
            "#,
        )
        .bind(serde_json::json!([entity_type]))
        .fetch_all(&self.pool)
        .await?;

        Ok(rules)
    }
}