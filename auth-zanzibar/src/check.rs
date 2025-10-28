use crate::{
    error::ZanzibarError,
    models::*,
    repository::TupleRepository,
    schema::Schema,
};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::debug;

/// Permission checker performs authorization checks with support for:
/// - Direct permissions
/// - Inherited permissions
/// - Userset references (e.g., "all editors are viewers")
/// - Recursive permission resolution
pub struct PermissionChecker {
    repository: Arc<dyn TupleRepository>,
    schema: Arc<Schema>,
}

impl PermissionChecker {
    pub fn new(repository: Arc<dyn TupleRepository>, schema: Arc<Schema>) -> Self {
        Self { repository, schema }
    }
    
    /// Check if subject has the specified relation to object
    pub async fn check(
        &self,
        subject: Subject,
        relation: Relation,
        object: Object,
        _context: Option<serde_json::Value>,
    ) -> Result<bool, ZanzibarError> {
        let mut visited = HashSet::new();
        self.check_recursive(subject, relation, object, &mut visited, 0).await
    }
    
    async fn check_recursive(
        &self,
        subject: Subject,
        relation: Relation,
        object: Object,
        visited: &mut HashSet<String>,
        depth: u32,
    ) -> Result<bool, ZanzibarError> {
        Box::pin(async move {
        // Prevent infinite recursion
        if depth > 10 {
            return Ok(false);
        }
        
        let check_key = format!("{}_{}_{}", subject, relation, object);
        if visited.contains(&check_key) {
            return Ok(false);
        }
        visited.insert(check_key);
        
        debug!("Checking: {} {} {}", subject, relation, object);
        
        // 1. Direct check: does the tuple exist?
        let direct_tuple = Tuple::new(subject.clone(), relation.clone(), object.clone());
        if self.repository.tuple_exists(&direct_tuple).await? {
            debug!("Direct permission found");
            return Ok(true);
        }
        
        // 2. Check for inherited permissions
        // If someone has 'editor', they should also have 'viewer' (if editor inherits from viewer)
        // We need to check if any higher-level relation grants this permission
        if let Some(namespace) = self.schema.namespaces.get(&object.object_type) {
            for rel_def in &namespace.relations {
                // Check if this relation inherits from the relation we're checking
                if rel_def.inherits_from.as_ref() == Some(&relation.name) {
                    // Check if subject has the higher-level relation
                    debug!("Checking if subject has higher permission: {}", rel_def.name);
                    if self.check_recursive(
                        subject.clone(),
                        Relation::new(&rel_def.name),
                        object.clone(),
                        visited,
                        depth + 1,
                    ).await? {
                        return Ok(true);
                    }
                }
            }
        }
        
        // 3. Check for userset references
        // Find all tuples where someone has this relation to the object
        let related_tuples = self.repository.read_tuples(
            None,
            Some(relation.clone()),
            Some(object.clone()),
        ).await?;
        
        for tuple in related_tuples {
            // Check if the subject is a member of the userset
            if let Some(ref userset_relation) = tuple.subject.relation {
                // The tuple references a userset like "document:doc1#editors"
                // Check if our subject has that relation to that object
                let userset_object = Object {
                    namespace: tuple.subject.namespace.clone(),
                    object_type: tuple.subject.object_type.clone(),
                    object_id: tuple.subject.object_id.clone(),
                };
                
                if self.check_recursive(
                    subject.clone(),
                    Relation::new(userset_relation),
                    userset_object,
                    visited,
                    depth + 1,
                ).await? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryTupleRepository;
    
    #[tokio::test]
    async fn test_direct_permission() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let schema = Arc::new(Schema::healthcare_schema());
        let checker = PermissionChecker::new(repo.clone(), schema);
        
        let alice = Subject::user("alice");
        let doc = Object::new("document", "doc1");
        let viewer = Relation::new("viewer");
        
        // No permission initially
        assert!(!checker.check(alice.clone(), viewer.clone(), doc.clone(), None).await.unwrap());
        
        // Grant permission
        let tuple = Tuple::new(alice.clone(), viewer.clone(), doc.clone());
        repo.write_tuple(tuple).await.unwrap();
        
        // Now has permission
        assert!(checker.check(alice, viewer, doc, None).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_inherited_permission() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let schema = Arc::new(Schema::healthcare_schema());
        let checker = PermissionChecker::new(repo.clone(), schema);
        
        let alice = Subject::user("alice");
        let doc = Object::new("document", "doc1");
        
        // Grant editor permission (which inherits viewer)
        let tuple = Tuple::new(alice.clone(), Relation::new("editor"), doc.clone());
        repo.write_tuple(tuple).await.unwrap();
        
        // Should have viewer permission through inheritance
        assert!(checker.check(alice.clone(), Relation::new("viewer"), doc.clone(), None).await.unwrap());
        
        // Should also have editor permission
        assert!(checker.check(alice, Relation::new("editor"), doc, None).await.unwrap());
    }
}
