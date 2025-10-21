use crate::{
    error::ZanzibarError,
    models::*,
    repository::TupleRepository,
};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::debug;

/// Subject expander finds all subjects with a given relation to an object
pub struct SubjectExpander {
    repository: Arc<dyn TupleRepository>,
}

impl SubjectExpander {
    pub fn new(repository: Arc<dyn TupleRepository>) -> Self {
        Self { repository }
    }
    
    /// Expand all subjects that have the specified relation to an object
    pub async fn expand(
        &self,
        relation: Relation,
        object: Object,
        max_depth: Option<u32>,
    ) -> Result<SubjectTree, ZanzibarError> {
        let max_depth = max_depth.unwrap_or(5);
        let mut visited = HashSet::new();
        
        let root = SubjectTree {
            subject: Subject {
                namespace: "root".to_string(),
                object_type: "root".to_string(),
                object_id: "root".to_string(),
                relation: None,
            },
            children: Vec::new(),
        };
        
        self.expand_recursive(relation, object, &mut visited, 0, max_depth).await
    }
    
    async fn expand_recursive(
        &self,
        relation: Relation,
        object: Object,
        visited: &mut HashSet<String>,
        depth: u32,
        max_depth: u32,
    ) -> Result<SubjectTree, ZanzibarError> {
        Box::pin(async move {
        if depth >= max_depth {
            return Ok(SubjectTree {
                subject: Subject {
                    namespace: "max_depth".to_string(),
                    object_type: "max_depth".to_string(),
                    object_id: "reached".to_string(),
                    relation: None,
                },
                children: Vec::new(),
            });
        }
        
        let key = format!("{}_{}", relation, object);
        if visited.contains(&key) {
            return Ok(SubjectTree {
                subject: Subject {
                    namespace: "cycle".to_string(),
                    object_type: "cycle".to_string(),
                    object_id: "detected".to_string(),
                    relation: None,
                },
                children: Vec::new(),
            });
        }
        visited.insert(key);
        
        debug!("Expanding: {} on {}", relation, object);
        
        // Find all tuples with this relation to this object
        let tuples = self.repository.read_tuples(
            None,
            Some(relation.clone()),
            Some(object.clone()),
        ).await?;
        
        let mut children = Vec::new();
        
        for tuple in tuples {
            // If the subject is a userset, expand it recursively
            if let Some(ref userset_relation) = tuple.subject.relation {
                let userset_object = Object {
                    namespace: tuple.subject.namespace.clone(),
                    object_type: tuple.subject.object_type.clone(),
                    object_id: tuple.subject.object_id.clone(),
                };
                
                let child_tree = self.expand_recursive(
                    Relation::new(userset_relation),
                    userset_object,
                    visited,
                    depth + 1,
                    max_depth,
                ).await?;
                
                children.push(child_tree);
            } else {
                // Direct subject
                children.push(SubjectTree {
                    subject: tuple.subject.clone(),
                    children: Vec::new(),
                });
            }
        }
        
        Ok(SubjectTree {
            subject: Subject {
                namespace: object.namespace,
                object_type: object.object_type,
                object_id: object.object_id,
                relation: Some(relation.name),
            },
            children,
        })
        }).await
    }
    
    /// List all subjects (flattened) with the given relation to an object
    pub async fn list_subjects(
        &self,
        relation: Relation,
        object: Object,
    ) -> Result<Vec<Subject>, ZanzibarError> {
        let tree = self.expand(relation, object, Some(10)).await?;
        Ok(self.flatten_tree(&tree))
    }
    
    fn flatten_tree(&self, tree: &SubjectTree) -> Vec<Subject> {
        let mut subjects = Vec::new();
        
        // Don't include root/cycle/max_depth markers
        if tree.subject.namespace != "root" 
            && tree.subject.namespace != "cycle" 
            && tree.subject.namespace != "max_depth" {
            subjects.push(tree.subject.clone());
        }
        
        for child in &tree.children {
            subjects.extend(self.flatten_tree(child));
        }
        
        subjects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::InMemoryTupleRepository;
    
    #[tokio::test]
    async fn test_expand_subjects() {
        let repo = Arc::new(InMemoryTupleRepository::new());
        let expander = SubjectExpander::new(repo.clone());
        
        let doc = Object::new("document", "doc1");
        let viewer = Relation::new("viewer");
        
        // Add some subjects with viewer relation
        repo.write_tuple(Tuple::new(
            Subject::user("alice"),
            viewer.clone(),
            doc.clone(),
        )).await.unwrap();
        
        repo.write_tuple(Tuple::new(
            Subject::user("bob"),
            viewer.clone(),
            doc.clone(),
        )).await.unwrap();
        
        // Expand
        let subjects = expander.list_subjects(viewer, doc).await.unwrap();
        assert_eq!(subjects.len(), 2);
    }
}

pub struct ExpandEngine {}