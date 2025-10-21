use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl Default for RlsContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Row Level Security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlsContext {
    pub user_id: Uuid,
    pub tenant_id: String,
    pub organization_id: Option<Uuid>, // NEW: For PostgreSQL RLS policies
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub session_id: Option<String>,
    pub additional_context: HashMap<String, String>,
}

impl RlsContext {
    pub fn new() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            tenant_id: String::new(),
            organization_id: None,
            roles: Vec::new(),
            permissions: Vec::new(),
            session_id: None,
            additional_context: HashMap::new(),
        }
    }
    
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = user_id;
        self
    }
    
    pub fn with_tenant_id<S: Into<String>>(mut self, tenant_id: S) -> Self {
        self.tenant_id = tenant_id.into();
        self
    }
    
    pub fn with_organization_id(mut self, organization_id: Uuid) -> Self {
        self.organization_id = Some(organization_id);
        self
    }
    
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }
    
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }
    
    pub fn add_role<S: Into<String>>(mut self, role: S) -> Self {
        self.roles.push(role.into());
        self
    }
    
    pub fn add_permission<S: Into<String>>(mut self, permission: S) -> Self {
        self.permissions.push(permission.into());
        self
    }
}

/// RLS policy manager
pub struct RlsPolicyManager {
    policies: HashMap<String, RlsPolicy>,
}

#[derive(Debug, Clone)]
pub struct RlsPolicy {
    pub table_name: String,
    pub policy_name: String,
    pub operation: RlsOperation,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub enum RlsOperation {
    Select,
    Insert,
    Update,
    Delete,
    All,
}

impl RlsPolicyManager {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }
    
    pub fn add_policy(&mut self, policy: RlsPolicy) {
        let key = format!("{}_{}", policy.table_name, policy.policy_name);
        self.policies.insert(key, policy);
    }
    
    pub fn get_policies_for_table(&self, table_name: &str) -> Vec<&RlsPolicy> {
        self.policies
            .values()
            .filter(|policy| policy.table_name == table_name)
            .collect()
    }
    
    pub fn generate_rls_sql(&self, context: &RlsContext) -> String {
        let mut sql = format!(
            "SET LOCAL app.current_user_id = '{}'; SET LOCAL app.current_tenant_id = '{}';",
            context.user_id,
            context.tenant_id
        );
        
        // Set organization_id for PostgreSQL RLS policies
        if let Some(org_id) = context.organization_id {
            sql.push_str(&format!(" SET LOCAL app.organization_id = '{}';", org_id));
        }
        
        // Set user roles
        if !context.roles.is_empty() {
            sql.push_str(&format!(" SET LOCAL app.user_roles = '{}';", context.roles.join(",")));
        }
        
        sql
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rls_context_creation() {
        let context = RlsContext::new()
            .with_tenant_id("tenant_123")
            .add_role("doctor")
            .add_permission("patient.read");
            
        assert_eq!(context.tenant_id, "tenant_123");
        assert!(context.roles.contains(&"doctor".to_string()));
        assert!(context.permissions.contains(&"patient.read".to_string()));
    }
}