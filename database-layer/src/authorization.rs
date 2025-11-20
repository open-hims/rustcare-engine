//! Integration layer connecting Zanzibar authorization with RLS and field masking
//!
//! This module provides:
//! - Authorization middleware that enforces Zanzibar permissions
//! - RLS context generation from Zanzibar permissions
//! - Field masking based on authorization checks
//! - Query rewriting to enforce security policies

use auth_zanzibar::{
    AuthorizationEngine, Subject, Relation, Object, FieldVisibility,
};
use crate::{
    rls::RlsContext,
    encryption::MaskPattern,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info};

/// Authorization middleware that integrates Zanzibar with database operations
pub struct AuthorizationMiddleware {
    engine: Arc<AuthorizationEngine>,
}

impl AuthorizationMiddleware {
    pub fn new(engine: Arc<AuthorizationEngine>) -> Self {
        Self { engine }
    }
    
    /// Generate RLS context from Zanzibar permissions
    /// This creates a PostgreSQL RLS context based on user's permissions
    pub async fn create_rls_context(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<RlsContext, anyhow::Error> {
        debug!("Creating RLS context for user {} in org {}", user_id, organization_id);
        
        // Get permissions from Zanzibar
        let rls_data = self.engine.generate_rls_context(user_id, organization_id).await?;
        
        // Convert to database RLS context
        let rls_context = RlsContext::new()
            .with_user_id(user_id)
            .with_organization_id(organization_id)
            .with_tenant_id(organization_id.to_string())
            .with_roles(rls_data.roles)
            .with_permissions(rls_data.permissions);
        
        info!("Created RLS context with {} roles and {} permissions", 
            rls_context.roles.len(), 
            rls_context.permissions.len()
        );
        
        Ok(rls_context)
    }
    
    /// Get field masking configuration based on permissions
    pub async fn get_field_masking(
        &self,
        user_id: Uuid,
        resource_type: &str,
        resource_id: &str,
        field_names: &[String],
    ) -> Result<Vec<FieldMaskingRule>, anyhow::Error> {
        let subject = Subject::user(&user_id.to_string());
        let object = Object::new(resource_type, resource_id);
        
        let mut rules = Vec::new();
        
        for field_name in field_names {
            let visibility = self.engine.get_field_visibility(
                subject.clone(),
                object.clone(),
                field_name,
            ).await?;
            
            let rule = match visibility {
                FieldVisibility::Full => FieldMaskingRule {
                    field_name: field_name.clone(),
                    mask_pattern: None,
                    visibility: FieldAccess::Full,
                },
                FieldVisibility::Masked => FieldMaskingRule {
                    field_name: field_name.clone(),
                    mask_pattern: Some(Self::get_default_mask_pattern(field_name)),
                    visibility: FieldAccess::Masked,
                },
                FieldVisibility::Hidden => FieldMaskingRule {
                    field_name: field_name.clone(),
                    mask_pattern: None,
                    visibility: FieldAccess::Hidden,
                },
            };
            
            rules.push(rule);
        }
        
        Ok(rules)
    }
    
    fn get_default_mask_pattern(field_name: &str) -> MaskPattern {
        match field_name {
            "ssn" | "tax_id" => MaskPattern::Partial { show_first: 0, show_last: 4 },
            "email" => MaskPattern::Partial { show_first: 2, show_last: 0 },
            "phone" => MaskPattern::Partial { show_first: 0, show_last: 4 },
            "medical_record_number" => MaskPattern::Partial { show_first: 0, show_last: 4 },
            "diagnosis" | "medication" | "prescription" | "treatment_notes" => {
                MaskPattern::Redacted
            }
            _ => MaskPattern::Hashed,
        }
    }
    
    /// Apply field masking to query results
    pub fn apply_masking<T: Maskable>(
        &self,
        data: T,
        rules: &[FieldMaskingRule],
    ) -> T {
        data.apply_masks(rules)
    }
    
    /// Check if user can perform operation on resource
    pub async fn check_permission(
        &self,
        user_id: Uuid,
        action: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<bool, anyhow::Error> {
        let subject = Subject::user(&user_id.to_string());
        let relation = Relation::new(action);
        let object = Object::new(resource_type, resource_id);
        
        Ok(self.engine.check(subject, relation, object).await?)
    }
    
    /// Batch check permissions for multiple resources
    pub async fn batch_check_permissions(
        &self,
        user_id: Uuid,
        action: &str,
        resources: Vec<(String, String)>, // (type, id) pairs
    ) -> Result<Vec<(String, String, bool)>, anyhow::Error> {
        let subject = Subject::user(&user_id.to_string());
        let relation = Relation::new(action);
        
        let mut results = Vec::new();
        for (resource_type, resource_id) in resources {
            let object = Object::new(&resource_type, &resource_id);
            let allowed = self.engine.check(subject.clone(), relation.clone(), object).await?;
            results.push((resource_type, resource_id, allowed));
        }
        
        Ok(results)
    }
}

/// Field masking rule derived from Zanzibar permissions
#[derive(Debug, Clone)]
pub struct FieldMaskingRule {
    pub field_name: String,
    pub mask_pattern: Option<MaskPattern>,
    pub visibility: FieldAccess,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldAccess {
    /// Full access to field
    Full,
    /// Field should be masked
    Masked,
    /// Field should be hidden
    Hidden,
}

/// Trait for types that can have field masking applied
pub trait Maskable: Sized {
    fn apply_masks(self, rules: &[FieldMaskingRule]) -> Self;
}

/// Helper functions for setting up authorization
pub mod setup {
    use super::*;
    use auth_zanzibar::{Tuple, Subject, Relation, Object};
    
    /// Initialize healthcare roles and permissions
    pub async fn initialize_healthcare_roles(
        engine: &AuthorizationEngine,
    ) -> Result<(), anyhow::Error> {
        info!("Initializing healthcare roles");
        
        // Doctor role with PHI access
        let doctor_role = Object::new("role", "doctor");
        engine.write_tuple(Tuple::new(
            Subject::group("doctor"),
            Relation::new("read_phi"),
            Object::new("permission", "phi_access"),
        )).await?;
        
        // Nurse role with limited PHI access
        let nurse_role = Object::new("role", "nurse");
        engine.write_tuple(Tuple::new(
            Subject::group("nurse"),
            Relation::new("read_phi"),
            Object::new("permission", "limited_phi_access"),
        )).await?;
        
        // Admin role with full access
        engine.write_tuple(Tuple::new(
            Subject::group("admin"),
            Relation::new("admin"),
            Object::new("organization", "*"),
        )).await?;
        
        // Patient role (can only see own data)
        engine.write_tuple(Tuple::new(
            Subject::group("patient"),
            Relation::new("owner"),
            Object::new("patient", "self"),
        )).await?;
        
        info!("Healthcare roles initialized");
        Ok(())
    }
    
    /// Grant user a role in an organization
    pub async fn grant_role(
        engine: &AuthorizationEngine,
        user_id: Uuid,
        role: &str,
        organization_id: Uuid,
    ) -> Result<(), anyhow::Error> {
        info!("Granting role {} to user {} in org {}", role, user_id, organization_id);
        
        engine.write_tuple(Tuple::new(
            Subject::user(&user_id.to_string()),
            Relation::new("member"),
            Object::new("role", role),
        )).await?;
        
        Ok(())
    }
    
    /// Grant resource access to user
    pub async fn grant_access(
        engine: &AuthorizationEngine,
        user_id: Uuid,
        permission: &str,
        resource_type: &str,
        resource_id: &str,
    ) -> Result<(), anyhow::Error> {
        info!("Granting {} permission on {}:{} to user {}", 
            permission, resource_type, resource_id, user_id);
        
        engine.write_tuple(Tuple::new(
            Subject::user(&user_id.to_string()),
            Relation::new(permission),
            Object::new(resource_type, resource_id),
        )).await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth_zanzibar::repository::InMemoryTupleRepository;
    
    #[tokio::test]
    async fn test_rls_context_generation() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let engine = Arc::new(AuthorizationEngine::new(repo).await.unwrap());
        let middleware = AuthorizationMiddleware::new(engine.clone());
        
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        
        // Grant user a role
        setup::grant_role(&engine, user_id, "doctor", org_id).await.unwrap();
        
        // Generate RLS context
        let rls_ctx = middleware.create_rls_context(user_id, org_id).await.unwrap();
        
        assert_eq!(rls_ctx.user_id, user_id);
        assert_eq!(rls_ctx.organization_id, Some(org_id));
    }
    
    #[tokio::test]
    async fn test_field_masking_rules() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let engine = Arc::new(AuthorizationEngine::new(repo).await.unwrap());
        let middleware = AuthorizationMiddleware::new(engine.clone());
        
        let user_id = Uuid::new_v4();
        let patient_id = "patient123";
        
        // User has viewer access (not owner)
        setup::grant_access(&engine, user_id, "viewer", "patient", patient_id).await.unwrap();
        
        // Get masking rules
        let fields = vec!["name".to_string(), "ssn".to_string(), "diagnosis".to_string()];
        let rules = middleware.get_field_masking(
            user_id,
            "patient",
            patient_id,
            &fields,
        ).await.unwrap();
        
        // name should be visible, PHI fields should be masked
        assert_eq!(rules.len(), 3);
    }
}
