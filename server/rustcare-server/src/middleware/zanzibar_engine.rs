//! Zanzibar authorization engine wrapper
//!
//! This module provides a wrapper around the auth-zanzibar AuthorizationEngine
//! that implements the ZanzibarCheck trait for use in AuthContext.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use auth_zanzibar::{AuthorizationEngine, Subject, Relation, Object};
use crate::middleware::auth_context::ZanzibarCheck;

/// Wrapper around AuthorizationEngine that implements ZanzibarCheck trait
pub struct ZanzibarEngineWrapper {
    engine: Arc<AuthorizationEngine>,
}

impl std::fmt::Debug for ZanzibarEngineWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZanzibarEngineWrapper")
            .field("engine", &"<AuthorizationEngine>")
            .finish()
    }
}

impl ZanzibarEngineWrapper {
    /// Create a new wrapper around an AuthorizationEngine
    pub fn new(engine: Arc<AuthorizationEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait]
impl ZanzibarCheck for ZanzibarEngineWrapper {
    async fn check_permission(
        &self,
        user_id: Uuid,
        resource_type: &str,
        resource_id: Option<Uuid>,
        permission: &str,
        organization_id: Uuid,
    ) -> Result<bool, String> {
        // Convert to Zanzibar Subject
        let subject = Subject::user(&user_id.to_string());
        
        // Convert to Zanzibar Object
        let object = if let Some(id) = resource_id {
            Object::new(resource_type, &id.to_string())
        } else {
            // For organization-level permissions, use organization as object
            Object::new(resource_type, &organization_id.to_string())
        };
        
        // Convert permission to Relation
        let relation = Relation::new(permission);
        
        // Perform the check
        self.engine
            .check(subject, relation, object)
            .await
            .map_err(|e| format!("Zanzibar check error: {}", e))
    }
}

