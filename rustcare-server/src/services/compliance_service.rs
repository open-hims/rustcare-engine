// Compliance Service: Automatic Rule Assignment Engine
// Automatically assigns compliance frameworks and rules based on organization and patient demographics

use chrono::{NaiveDate, Utc};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::error_common::AppError;

#[derive(Debug, Clone)]
pub struct ComplianceService {
    pool: PgPool,
}

// =============================================================================
// DTOs and Models
// =============================================================================

#[derive(Debug, Clone)]
pub struct OrganizationDemographics {
    pub organization_id: Uuid,
    pub facility_type: String,
    pub facility_subtype: Option<String>,
    pub primary_state: String,
    pub operating_states: Vec<String>,
    pub is_medicare_certified: bool,
    pub is_medicaid_certified: bool,
    pub handles_genetic_data: bool,
    pub handles_mental_health_data: bool,
    pub handles_substance_abuse_data: bool,
    pub handles_hiv_data: bool,
    pub serves_minors: bool,
    pub organization_type: String,
    pub is_covered_entity: bool,
    pub operates_internationally: bool,
    pub international_countries: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PatientDemographicsCompliance {
    pub patient_id: Uuid,
    pub organization_id: Uuid,
    pub resident_state: Option<String>,
    pub resident_country: String,
    pub is_minor: bool,
    pub is_veteran: bool,
    pub is_prisoner: bool,
    pub has_genetic_data: bool,
    pub has_mental_health_records: bool,
    pub has_substance_abuse_records: bool,
    pub has_hiv_records: bool,
    pub privacy_level: String,
}

#[derive(Debug, Clone)]
pub struct ComplianceFramework {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub jurisdiction_type: String,
    pub jurisdiction_code: Option<String>,
    pub applies_to_states: Vec<String>,
    pub applicability_rules: serde_json::Value,
    pub is_mandatory: bool,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct ComplianceRule {
    pub id: Uuid,
    pub framework_id: Uuid,
    pub rule_code: String,
    pub rule_name: String,
    pub category: String,
    pub severity: String,
    pub applies_to_entity: String,
    pub applicability_criteria: serde_json::Value,
    pub is_automated: bool,
}

#[derive(Debug)]
pub struct ComplianceAssignmentResult {
    pub frameworks_assigned: Vec<String>,
    pub rules_assigned: Vec<String>,
    pub audit_entries_created: usize,
}

// =============================================================================
// Implementation
// =============================================================================

impl ComplianceService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Organization Compliance Assignment
    // =========================================================================

    /// Automatically assign compliance frameworks to organization based on demographics
    pub async fn assign_organization_compliance(
        &self,
        organization_id: Uuid,
        demographics: &OrganizationDemographics,
        assigned_by: Uuid,
    ) -> Result<ComplianceAssignmentResult, AppError> {
        let mut tx = self.pool.begin().await?;

        // Get all active frameworks
        let frameworks = self.get_applicable_frameworks(&demographics).await?;

        let mut frameworks_assigned = Vec::new();
        let mut rules_assigned = Vec::new();

        for framework in frameworks {
            // Check if framework already assigned
            let existing = sqlx::query!(
                r#"
                SELECT id FROM organization_compliance
                WHERE organization_id = $1 AND framework_id = $2 AND is_active = TRUE
                "#,
                organization_id,
                framework.id
            )
            .fetch_optional(&mut *tx)
            .await?;

            if existing.is_some() {
                continue; // Already assigned
            }

            // Determine triggering criteria
            let triggering_criteria = self.build_triggering_criteria(&framework, &demographics);

            // Assign framework
            sqlx::query!(
                r#"
                INSERT INTO organization_compliance (
                    organization_id, framework_id, assignment_type, assignment_reason,
                    triggering_criteria, compliance_status, created_by, next_assessment_date
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                organization_id,
                framework.id,
                "AUTOMATIC",
                format!("Auto-assigned based on organization demographics: {} in {}", demographics.facility_type, demographics.primary_state),
                triggering_criteria,
                "PENDING_REVIEW",
                assigned_by,
                Utc::now().date_naive() + chrono::Duration::days(90)
            )
            .execute(&mut *tx)
            .await?;

            frameworks_assigned.push(framework.code.clone());

            // Log audit entry
            self.log_compliance_event(
                &mut tx,
                organization_id,
                "FRAMEWORK_ASSIGNED",
                "ASSIGNMENT",
                "INFO",
                &format!("Framework {} automatically assigned to organization", framework.code),
                json!({
                    "framework_id": framework.id,
                    "framework_code": framework.code,
                    "assignment_type": "AUTOMATIC",
                    "triggering_criteria": triggering_criteria
                }),
                Some(framework.id),
                None,
                Some(assigned_by),
            )
            .await?;

            // Assign applicable rules for this framework
            let rules = self.get_applicable_rules_for_organization(&framework.id, &demographics).await?;
            
            for rule in rules {
                self.assign_rule_to_entity(
                    &mut tx,
                    rule.id,
                    "ORGANIZATION",
                    organization_id,
                    organization_id,
                    "AUTOMATIC",
                    &format!("Auto-assigned with framework {}", framework.code),
                    json!({"framework": framework.code}),
                    assigned_by,
                )
                .await?;

                rules_assigned.push(rule.rule_code.clone());
            }
        }

        let audit_count = frameworks_assigned.len() + rules_assigned.len();

        tx.commit().await?;

        Ok(ComplianceAssignmentResult {
            frameworks_assigned,
            rules_assigned,
            audit_entries_created: audit_count,
        })
    }

    // =========================================================================
    // Patient Compliance Assignment
    // =========================================================================

    /// Automatically assign compliance rules to patient based on demographics
    pub async fn assign_patient_compliance(
        &self,
        patient_id: Uuid,
        demographics: &PatientDemographicsCompliance,
        assigned_by: Uuid,
    ) -> Result<ComplianceAssignmentResult, AppError> {
        let mut tx = self.pool.begin().await?;

        // Get organization's assigned frameworks
        let org_frameworks = sqlx::query_as!(
            ComplianceFramework,
            r#"
            SELECT 
                cf.id, cf.code, cf.name, cf.jurisdiction_type,
                cf.jurisdiction_code, cf.applies_to_states,
                cf.applicability_rules, cf.is_mandatory, cf.priority
            FROM compliance_frameworks cf
            JOIN organization_compliance oc ON cf.id = oc.framework_id
            WHERE oc.organization_id = $1 AND oc.is_active = TRUE AND cf.is_active = TRUE
            ORDER BY cf.priority ASC
            "#,
            demographics.organization_id
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut frameworks_used = Vec::new();
        let mut rules_assigned = Vec::new();

        for framework in org_frameworks {
            // Get patient-applicable rules from this framework
            let rules = self.get_applicable_rules_for_patient(&framework.id, &demographics).await?;

            for rule in rules {
                // Check if rule already assigned
                let existing = sqlx::query!(
                    r#"
                    SELECT id FROM compliance_rule_assignments
                    WHERE rule_id = $1 AND entity_type = 'PATIENT' AND entity_id = $2 AND is_active = TRUE
                    "#,
                    rule.id,
                    patient_id
                )
                .fetch_optional(&mut *tx)
                .await?;

                if existing.is_some() {
                    continue;
                }

                // Build triggering criteria
                let triggering_criteria = self.build_patient_triggering_criteria(&rule, &demographics);

                // Assign rule
                self.assign_rule_to_entity(
                    &mut tx,
                    rule.id,
                    "PATIENT",
                    patient_id,
                    demographics.organization_id,
                    "AUTOMATIC",
                    &format!("Auto-assigned based on patient demographics"),
                    triggering_criteria,
                    assigned_by,
                )
                .await?;

                rules_assigned.push(rule.rule_code.clone());
                
                if !frameworks_used.contains(&framework.code) {
                    frameworks_used.push(framework.code.clone());
                }
            }
        }

        let audit_count = rules_assigned.len();

        tx.commit().await?;

        Ok(ComplianceAssignmentResult {
            frameworks_assigned: frameworks_used,
            rules_assigned,
            audit_entries_created: audit_count,
        })
    }

    // =========================================================================
    // Helper Methods: Framework Selection
    // =========================================================================

    async fn get_applicable_frameworks(
        &self,
        demographics: &OrganizationDemographics,
    ) -> Result<Vec<ComplianceFramework>, AppError> {
        let frameworks = sqlx::query_as!(
            ComplianceFramework,
            r#"
            SELECT 
                id, code, name, jurisdiction_type, jurisdiction_code,
                applies_to_states, applicability_rules, is_mandatory, priority
            FROM compliance_frameworks
            WHERE is_active = TRUE
            ORDER BY priority ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut applicable = Vec::new();

        for framework in frameworks {
            if self.is_framework_applicable(&framework, demographics) {
                applicable.push(framework);
            }
        }

        Ok(applicable)
    }

    fn is_framework_applicable(
        &self,
        framework: &ComplianceFramework,
        demographics: &OrganizationDemographics,
    ) -> bool {
        // Federal laws apply to all US organizations
        if framework.jurisdiction_type == "FEDERAL" && demographics.primary_state.len() == 2 {
            // Check specific applicability rules
            if let Some(applies_to) = framework.applicability_rules.get("applies_to") {
                if let Some(applies_to_arr) = applies_to.as_array() {
                    let applies_to_strs: Vec<&str> = applies_to_arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect();
                    
                    if applies_to_strs.contains(&"COVERED_ENTITY") && !demographics.is_covered_entity {
                        return false;
                    }
                }
            }
            return true;
        }

        // State laws apply if organization operates in that state
        if framework.jurisdiction_type == "STATE" {
            if !framework.applies_to_states.is_empty() {
                for state in &framework.applies_to_states {
                    if state == &demographics.primary_state || demographics.operating_states.contains(state) {
                        return true;
                    }
                }
            }
            return false;
        }

        // International laws apply if operating internationally
        if framework.jurisdiction_type == "INTERNATIONAL" {
            if demographics.operates_internationally {
                if let Some(jurisdiction) = &framework.jurisdiction_code {
                    return demographics.international_countries.contains(jurisdiction);
                }
                return true;
            }
            return false;
        }

        // Industry standards are optional by default
        if framework.jurisdiction_type == "INDUSTRY" {
            return framework.is_mandatory;
        }

        false
    }

    // =========================================================================
    // Helper Methods: Rule Selection
    // =========================================================================

    async fn get_applicable_rules_for_organization(
        &self,
        framework_id: &Uuid,
        demographics: &OrganizationDemographics,
    ) -> Result<Vec<ComplianceRule>, AppError> {
        let rules = sqlx::query_as!(
            ComplianceRule,
            r#"
            SELECT 
                id, framework_id, rule_code, rule_name, category, severity,
                applies_to_entity, applicability_criteria, is_automated
            FROM compliance_rules
            WHERE framework_id = $1 
              AND is_active = TRUE
              AND applies_to_entity IN ('ORGANIZATION', 'SYSTEM')
            "#,
            framework_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut applicable = Vec::new();

        for rule in rules {
            if self.is_rule_applicable_to_organization(&rule, demographics) {
                applicable.push(rule);
            }
        }

        Ok(applicable)
    }

    async fn get_applicable_rules_for_patient(
        &self,
        framework_id: &Uuid,
        demographics: &PatientDemographicsCompliance,
    ) -> Result<Vec<ComplianceRule>, AppError> {
        let rules = sqlx::query_as!(
            ComplianceRule,
            r#"
            SELECT 
                id, framework_id, rule_code, rule_name, category, severity,
                applies_to_entity, applicability_criteria, is_automated
            FROM compliance_rules
            WHERE framework_id = $1 
              AND is_active = TRUE
              AND applies_to_entity IN ('PATIENT', 'DATA')
            "#,
            framework_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut applicable = Vec::new();

        for rule in rules {
            if self.is_rule_applicable_to_patient(&rule, demographics) {
                applicable.push(rule);
            }
        }

        Ok(applicable)
    }

    fn is_rule_applicable_to_organization(
        &self,
        rule: &ComplianceRule,
        demographics: &OrganizationDemographics,
    ) -> bool {
        // Check applicability criteria from JSONB
        if let Some(criteria) = rule.applicability_criteria.as_object() {
            // Check facility types
            if let Some(facility_types) = criteria.get("facility_types") {
                if let Some(types_arr) = facility_types.as_array() {
                    let types: Vec<&str> = types_arr.iter().filter_map(|v| v.as_str()).collect();
                    if !types.is_empty() && !types.contains(&demographics.facility_type.as_str()) {
                        return false;
                    }
                }
            }

            // Check data types handled
            if let Some(data_types) = criteria.get("requires_data_types") {
                if let Some(types_arr) = data_types.as_array() {
                    for data_type in types_arr.iter().filter_map(|v| v.as_str()) {
                        match data_type {
                            "GENETIC" if !demographics.handles_genetic_data => return false,
                            "MENTAL_HEALTH" if !demographics.handles_mental_health_data => return false,
                            "SUBSTANCE_ABUSE" if !demographics.handles_substance_abuse_data => return false,
                            "HIV" if !demographics.handles_hiv_data => return false,
                            _ => {}
                        }
                    }
                }
            }
        }

        true
    }

    fn is_rule_applicable_to_patient(
        &self,
        rule: &ComplianceRule,
        demographics: &PatientDemographicsCompliance,
    ) -> bool {
        if let Some(criteria) = rule.applicability_criteria.as_object() {
            // Check if rule applies to minors
            if let Some(requires_minor) = criteria.get("applies_to_minors") {
                if requires_minor.as_bool() == Some(true) && !demographics.is_minor {
                    return false;
                }
            }

            // Check special populations
            if let Some(special_pop) = criteria.get("special_populations") {
                if let Some(pop_arr) = special_pop.as_array() {
                    for pop in pop_arr.iter().filter_map(|v| v.as_str()) {
                        match pop {
                            "VETERAN" if !demographics.is_veteran => return false,
                            "PRISONER" if !demographics.is_prisoner => return false,
                            _ => {}
                        }
                    }
                }
            }

            // Check data types
            if let Some(data_types) = criteria.get("requires_data_types") {
                if let Some(types_arr) = data_types.as_array() {
                    for data_type in types_arr.iter().filter_map(|v| v.as_str()) {
                        match data_type {
                            "GENETIC" if !demographics.has_genetic_data => return false,
                            "MENTAL_HEALTH" if !demographics.has_mental_health_records => return false,
                            "SUBSTANCE_ABUSE" if !demographics.has_substance_abuse_records => return false,
                            "HIV" if !demographics.has_hiv_records => return false,
                            _ => {}
                        }
                    }
                }
            }
        }

        true
    }

    // =========================================================================
    // Helper Methods: Rule Assignment
    // =========================================================================

    async fn assign_rule_to_entity(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        rule_id: Uuid,
        entity_type: &str,
        entity_id: Uuid,
        organization_id: Uuid,
        assignment_type: &str,
        reason: &str,
        triggering_criteria: serde_json::Value,
        assigned_by: Uuid,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO compliance_rule_assignments (
                rule_id, entity_type, entity_id, organization_id,
                assignment_type, assignment_reason, triggering_criteria,
                compliance_status, next_check_date, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (rule_id, entity_type, entity_id, is_active) WHERE is_active = TRUE
            DO NOTHING
            "#,
            rule_id,
            entity_type,
            entity_id,
            organization_id,
            assignment_type,
            reason,
            triggering_criteria,
            "PENDING",
            Utc::now().date_naive() + chrono::Duration::days(30),
            assigned_by
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    // =========================================================================
    // Helper Methods: Criteria Building
    // =========================================================================

    fn build_triggering_criteria(
        &self,
        framework: &ComplianceFramework,
        demographics: &OrganizationDemographics,
    ) -> serde_json::Value {
        json!({
            "framework_code": framework.code,
            "facility_type": demographics.facility_type,
            "facility_subtype": demographics.facility_subtype,
            "primary_state": demographics.primary_state,
            "operating_states": demographics.operating_states,
            "is_covered_entity": demographics.is_covered_entity,
            "is_medicare_certified": demographics.is_medicare_certified,
            "is_medicaid_certified": demographics.is_medicaid_certified,
            "handles_genetic_data": demographics.handles_genetic_data,
            "handles_mental_health_data": demographics.handles_mental_health_data,
            "handles_substance_abuse_data": demographics.handles_substance_abuse_data,
            "assigned_at": Utc::now().to_rfc3339()
        })
    }

    fn build_patient_triggering_criteria(
        &self,
        rule: &ComplianceRule,
        demographics: &PatientDemographicsCompliance,
    ) -> serde_json::Value {
        json!({
            "rule_code": rule.rule_code,
            "resident_state": demographics.resident_state,
            "resident_country": demographics.resident_country,
            "is_minor": demographics.is_minor,
            "is_veteran": demographics.is_veteran,
            "is_prisoner": demographics.is_prisoner,
            "has_genetic_data": demographics.has_genetic_data,
            "has_mental_health_records": demographics.has_mental_health_records,
            "has_substance_abuse_records": demographics.has_substance_abuse_records,
            "privacy_level": demographics.privacy_level,
            "assigned_at": Utc::now().to_rfc3339()
        })
    }

    // =========================================================================
    // Audit Logging
    // =========================================================================

    async fn log_compliance_event(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        organization_id: Uuid,
        event_type: &str,
        event_category: &str,
        severity: &str,
        description: &str,
        event_details: serde_json::Value,
        framework_id: Option<Uuid>,
        rule_id: Option<Uuid>,
        actor_user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO compliance_audit_log (
                organization_id, event_type, event_category, severity,
                description, event_details, framework_id, rule_id, actor_user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            organization_id,
            event_type,
            event_category,
            severity,
            description,
            event_details,
            framework_id,
            rule_id,
            actor_user_id
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    // =========================================================================
    // Public Query Methods
    // =========================================================================

    /// Get all compliance frameworks assigned to an organization
    pub async fn get_organization_frameworks(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ComplianceFramework>, AppError> {
        let frameworks = sqlx::query_as!(
            ComplianceFramework,
            r#"
            SELECT 
                cf.id, cf.code, cf.name, cf.jurisdiction_type,
                cf.jurisdiction_code, cf.applies_to_states,
                cf.applicability_rules, cf.is_mandatory, cf.priority
            FROM compliance_frameworks cf
            JOIN organization_compliance oc ON cf.id = oc.framework_id
            WHERE oc.organization_id = $1 AND oc.is_active = TRUE
            ORDER BY cf.priority ASC
            "#,
            organization_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(frameworks)
    }

    /// Get compliance status summary for an organization
    pub async fn get_compliance_summary(
        &self,
        organization_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let summary = sqlx::query!(
            r#"
            SELECT 
                COUNT(DISTINCT oc.framework_id) as framework_count,
                COUNT(DISTINCT CASE WHEN oc.compliance_status = 'COMPLIANT' THEN oc.id END) as compliant_frameworks,
                COUNT(DISTINCT cra.id) as total_rules_assigned,
                COUNT(DISTINCT CASE WHEN cra.compliance_status = 'COMPLIANT' THEN cra.id END) as compliant_rules,
                COUNT(DISTINCT CASE WHEN cra.compliance_status = 'NON_COMPLIANT' THEN cra.id END) as non_compliant_rules
            FROM organization_compliance oc
            LEFT JOIN compliance_rule_assignments cra ON oc.organization_id = cra.organization_id
            WHERE oc.organization_id = $1 AND oc.is_active = TRUE
            GROUP BY oc.organization_id
            "#,
            organization_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(json!({
            "framework_count": summary.framework_count,
            "compliant_frameworks": summary.compliant_frameworks,
            "total_rules_assigned": summary.total_rules_assigned,
            "compliant_rules": summary.compliant_rules,
            "non_compliant_rules": summary.non_compliant_rules,
            "compliance_percentage": if summary.total_rules_assigned.unwrap_or(0) > 0 {
                (summary.compliant_rules.unwrap_or(0) as f64 / summary.total_rules_assigned.unwrap_or(1) as f64) * 100.0
            } else {
                0.0
            }
        }))
    }
}
