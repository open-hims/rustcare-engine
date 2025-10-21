use crate::{
    models::*,
    repository::TupleRepository,
    schema::{Schema, PermissionDefinition},
    check::PermissionChecker,
    expand::SubjectExpander,
    error::ZanzibarError,
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Core Zanzibar authorization engine
pub struct AuthorizationEngine {
    /// Storage for relationship tuples
    repository: Arc<dyn TupleRepository>,
    
    /// Permission schema definitions
    schema: Arc<Schema>,
    
    /// Permission checker for authorization queries
    checker: Arc<PermissionChecker>,
    
    /// Subject expander for listing subjects
    expander: Arc<SubjectExpander>,
    
    /// Cache for permission checks (optional)
    cache: Option<Arc<DashMap<String, bool>>>,
    
    /// Enable debug mode for detailed traces
    debug_mode: bool,
}

impl AuthorizationEngine {
    /// Create a new authorization engine with the given repository
    pub async fn new(repository: Arc<dyn TupleRepository>) -> Result<Self, ZanzibarError> {
        let schema = Arc::new(Schema::default());
        let checker = Arc::new(PermissionChecker::new(repository.clone(), schema.clone()));
        let expander = Arc::new(SubjectExpander::new(repository.clone()));
        
        Ok(Self {
            repository,
            schema,
            checker,
            expander,
            cache: None,
            debug_mode: false,
        })
    }
    
    /// Create with a custom schema
    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Arc::new(schema);
        self.checker = Arc::new(PermissionChecker::new(
            self.repository.clone(),
            self.schema.clone(),
        ));
        self
    }
    
    /// Enable caching for permission checks
    pub fn with_cache(mut self) -> Self {
        self.cache = Some(Arc::new(DashMap::new()));
        self
    }
    
    /// Enable debug mode for detailed permission traces
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_mode = enabled;
        self
    }
    
    // =============================================================================
    // Core Authorization Operations
    // =============================================================================
    
    /// Check if a subject has permission to perform an action on an object
    pub async fn check(
        &self,
        subject: Subject,
        relation: Relation,
        object: Object,
    ) -> Result<bool, ZanzibarError> {
        self.check_with_context(subject, relation, object, None).await
    }
    
    /// Check permission with additional context
    pub async fn check_with_context(
        &self,
        subject: Subject,
        relation: Relation,
        object: Object,
        context: Option<serde_json::Value>,
    ) -> Result<bool, ZanzibarError> {
        let cache_key = format!("{}_{}_{}", subject, relation, object);
        
        // Check cache first if enabled
        if let Some(ref cache) = self.cache {
            if let Some(result) = cache.get(&cache_key) {
                debug!("Cache hit for permission check: {}", cache_key);
                return Ok(*result);
            }
        }
        
        // Perform the check
        let result = self.checker.check(subject, relation, object, context).await?;
        
        // Update cache if enabled
        if let Some(ref cache) = self.cache {
            cache.insert(cache_key, result);
        }
        
        Ok(result)
    }
    
    /// Batch check multiple permissions at once
    pub async fn batch_check(
        &self,
        requests: Vec<CheckRequest>,
    ) -> Result<Vec<CheckResponse>, ZanzibarError> {
        let mut responses = Vec::with_capacity(requests.len());
        
        for request in requests {
            let allowed = self.check_with_context(
                request.subject,
                request.relation,
                request.object,
                request.context,
            ).await?;
            
            responses.push(CheckResponse {
                allowed,
                debug_trace: if self.debug_mode {
                    Some(vec![format!("Check completed: {}", allowed)])
                } else {
                    None
                },
            });
        }
        
        Ok(responses)
    }
    
    // =============================================================================
    // Tuple Management
    // =============================================================================
    
    /// Write a relationship tuple
    pub async fn write_tuple(&self, tuple: Tuple) -> Result<(), ZanzibarError> {
        info!("Writing tuple: {}", tuple);
        
        // Validate tuple against schema
        self.schema.validate_tuple(&tuple)?;
        
        // Write to repository
        self.repository.write_tuple(tuple).await?;
        
        // Invalidate cache if enabled
        if let Some(ref cache) = self.cache {
            cache.clear();
        }
        
        Ok(())
    }
    
    /// Delete a relationship tuple
    pub async fn delete_tuple(&self, tuple: Tuple) -> Result<(), ZanzibarError> {
        info!("Deleting tuple: {}", tuple);
        
        self.repository.delete_tuple(tuple).await?;
        
        // Invalidate cache
        if let Some(ref cache) = self.cache {
            cache.clear();
        }
        
        Ok(())
    }
    
    /// Batch write operation (atomic)
    pub async fn batch_write(&self, request: WriteRequest) -> Result<(), ZanzibarError> {
        // Validate all tuples
        for tuple in &request.writes {
            self.schema.validate_tuple(tuple)?;
        }
        
        // Perform batch write
        self.repository.batch_write(request).await?;
        
        // Invalidate cache
        if let Some(ref cache) = self.cache {
            cache.clear();
        }
        
        Ok(())
    }
    
    /// Read tuples matching a filter
    pub async fn read_tuples(
        &self,
        subject: Option<Subject>,
        relation: Option<Relation>,
        object: Option<Object>,
    ) -> Result<Vec<Tuple>, ZanzibarError> {
        self.repository.read_tuples(subject, relation, object).await
    }
    
    // =============================================================================
    // Permission Expansion
    // =============================================================================
    
    /// Expand all subjects that have a relation to an object
    pub async fn expand(
        &self,
        relation: Relation,
        object: Object,
        max_depth: Option<u32>,
    ) -> Result<SubjectTree, ZanzibarError> {
        self.expander.expand(relation, object, max_depth).await
    }
    
    /// List all objects a subject has a specific relation to
    pub async fn list_objects(
        &self,
        subject: Subject,
        relation: Relation,
        object_type: String,
    ) -> Result<Vec<Object>, ZanzibarError> {
        let tuples = self.repository.read_tuples(
            Some(subject),
            Some(relation),
            None,
        ).await?;
        
        Ok(tuples
            .into_iter()
            .filter(|t| t.object.object_type == object_type)
            .map(|t| t.object)
            .collect())
    }
    
    // =============================================================================
    // Schema Management
    // =============================================================================
    
    /// Get the current schema
    pub fn get_schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    
    /// Update the schema (requires validation)
    pub async fn update_schema(&mut self, schema: Schema) -> Result<(), ZanzibarError> {
        // Validate schema is well-formed
        schema.validate()?;
        
        self.schema = Arc::new(schema);
        self.checker = Arc::new(PermissionChecker::new(
            self.repository.clone(),
            self.schema.clone(),
        ));
        
        // Clear cache after schema update
        if let Some(ref cache) = self.cache {
            cache.clear();
        }
        
        Ok(())
    }
    
    // =============================================================================
    // RLS Integration
    // =============================================================================
    
    /// Generate RLS context from Zanzibar permissions
    /// This bridges Zanzibar authorization to PostgreSQL RLS
    pub async fn generate_rls_context(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<RlsContextData, ZanzibarError> {
        // Find all roles for this user
        let user_subject = Subject::user(&user_id.to_string());
        let role_tuples = self.repository.read_tuples(
            Some(user_subject),
            Some(Relation::new("member")),
            None,
        ).await?;
        
        let roles: Vec<String> = role_tuples
            .iter()
            .filter_map(|t| {
                if t.object.object_type == "role" {
                    Some(t.object.object_id.clone())
                } else {
                    None
                }
            })
            .collect();
        
        // Find all permissions for this user
        let mut permissions = Vec::new();
        for role in &roles {
            let role_subject = Subject::group(role);
            let perm_tuples = self.repository.read_tuples(
                Some(role_subject),
                None,
                None,
            ).await?;
            
            for tuple in perm_tuples {
                let perm = format!("{}:{}", tuple.object.object_type, tuple.relation.name);
                if !permissions.contains(&perm) {
                    permissions.push(perm);
                }
            }
        }
        
        Ok(RlsContextData {
            user_id,
            organization_id,
            roles,
            permissions,
        })
    }
    
    // =============================================================================
    // Field Masking Integration
    // =============================================================================
    
    /// Determine field masking level based on permissions
    pub async fn get_field_visibility(
        &self,
        subject: Subject,
        object: Object,
        field_name: &str,
    ) -> Result<FieldVisibility, ZanzibarError> {
        // Check for full access
        if self.check(subject.clone(), Relation::new("owner"), object.clone()).await? {
            return Ok(FieldVisibility::Full);
        }
        
        // Check for edit access
        if self.check(subject.clone(), Relation::new("editor"), object.clone()).await? {
            return Ok(FieldVisibility::Full);
        }
        
        // Check field-specific permissions
        let field_read = format!("read_{}", field_name);
        if self.check(subject.clone(), Relation::new(&field_read), object.clone()).await? {
            return Ok(FieldVisibility::Full);
        }
        
        // Check for viewer access - might be masked
        if self.check(subject.clone(), Relation::new("viewer"), object.clone()).await? {
            // Determine if field contains PHI
            if self.is_phi_field(field_name) {
                return Ok(FieldVisibility::Masked);
            }
            return Ok(FieldVisibility::Full);
        }
        
        Ok(FieldVisibility::Hidden)
    }
    
    fn is_phi_field(&self, field_name: &str) -> bool {
        matches!(
            field_name,
            "ssn" | "diagnosis" | "medication" | "prescription" | 
            "lab_result" | "treatment_notes" | "medical_record_number"
        )
    }
}

/// RLS context data generated from Zanzibar permissions
#[derive(Debug, Clone)]
pub struct RlsContextData {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

/// Field visibility level for masking
#[derive(Debug, Clone, PartialEq)]
pub enum FieldVisibility {
    /// Field is fully visible
    Full,
    /// Field should be masked (e.g., SSN -> ***-**-1234)
    Masked,
    /// Field should not be shown at all
    Hidden,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryTupleRepository;
    
    #[tokio::test]
    async fn test_basic_check() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let engine = AuthorizationEngine::new(repo).await.unwrap();
        
        let alice = Subject::user("alice");
        let doc = Object::new("document", "doc1");
        let editor = Relation::new("editor");
        
        // Initially no permission
        let allowed = engine.check(alice.clone(), editor.clone(), doc.clone()).await.unwrap();
        assert!(!allowed);
        
        // Grant permission
        let tuple = Tuple::new(alice.clone(), editor.clone(), doc.clone());
        engine.write_tuple(tuple).await.unwrap();
        
        // Now should have permission
        let allowed = engine.check(alice, editor, doc).await.unwrap();
        assert!(allowed);
    }
}
