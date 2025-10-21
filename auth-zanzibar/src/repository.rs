use crate::{error::ZanzibarError, models::*};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

/// Repository interface for storing relationship tuples
#[async_trait]
pub trait TupleRepository: Send + Sync {
    /// Write a single tuple
    async fn write_tuple(&self, tuple: Tuple) -> Result<(), ZanzibarError>;
    
    /// Delete a single tuple
    async fn delete_tuple(&self, tuple: Tuple) -> Result<(), ZanzibarError>;
    
    /// Batch write operation (atomic)
    async fn batch_write(&self, request: WriteRequest) -> Result<(), ZanzibarError>;
    
    /// Read tuples matching the given filter
    /// None values act as wildcards
    async fn read_tuples(
        &self,
        subject: Option<Subject>,
        relation: Option<Relation>,
        object: Option<Object>,
    ) -> Result<Vec<Tuple>, ZanzibarError>;
    
    /// Check if a specific tuple exists
    async fn tuple_exists(&self, tuple: &Tuple) -> Result<bool, ZanzibarError>;
}

/// In-memory tuple repository for testing and development
pub struct InMemoryTupleRepository {
    tuples: Arc<DashMap<String, Tuple>>,
}

impl InMemoryTupleRepository {
    pub fn new() -> Self {
        Self {
            tuples: Arc::new(DashMap::new()),
        }
    }
    
    fn tuple_key(tuple: &Tuple) -> String {
        format!("{}_{}_{}", tuple.subject, tuple.relation, tuple.object)
    }
}

impl Default for InMemoryTupleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TupleRepository for InMemoryTupleRepository {
    async fn write_tuple(&self, tuple: Tuple) -> Result<(), ZanzibarError> {
        let key = Self::tuple_key(&tuple);
        self.tuples.insert(key, tuple);
        Ok(())
    }
    
    async fn delete_tuple(&self, tuple: Tuple) -> Result<(), ZanzibarError> {
        let key = Self::tuple_key(&tuple);
        self.tuples.remove(&key);
        Ok(())
    }
    
    async fn batch_write(&self, request: WriteRequest) -> Result<(), ZanzibarError> {
        // Write all tuples
        for tuple in request.writes {
            self.write_tuple(tuple).await?;
        }
        
        // Delete tuples
        for tuple in request.deletes {
            self.delete_tuple(tuple).await?;
        }
        
        Ok(())
    }
    
    async fn read_tuples(
        &self,
        subject: Option<Subject>,
        relation: Option<Relation>,
        object: Option<Object>,
    ) -> Result<Vec<Tuple>, ZanzibarError> {
        let tuples: Vec<Tuple> = self.tuples
            .iter()
            .filter(|entry| {
                let tuple = entry.value();
                
                if let Some(ref s) = subject {
                    if tuple.subject != *s {
                        return false;
                    }
                }
                
                if let Some(ref r) = relation {
                    if tuple.relation != *r {
                        return false;
                    }
                }
                
                if let Some(ref o) = object {
                    if tuple.object != *o {
                        return false;
                    }
                }
                
                true
            })
            .map(|entry| entry.value().clone())
            .collect();
        
        Ok(tuples)
    }
    
    async fn tuple_exists(&self, tuple: &Tuple) -> Result<bool, ZanzibarError> {
        let key = Self::tuple_key(tuple);
        Ok(self.tuples.contains_key(&key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_in_memory_repository() {
        let repo = InMemoryTupleRepository::new();
        
        let tuple = Tuple::new(
            Subject::user("alice"),
            Relation::new("editor"),
            Object::new("document", "doc1"),
        );
        
        // Write tuple
        repo.write_tuple(tuple.clone()).await.unwrap();
        
        // Check it exists
        assert!(repo.tuple_exists(&tuple).await.unwrap());
        
        // Read tuples
        let tuples = repo.read_tuples(
            Some(Subject::user("alice")),
            None,
            None,
        ).await.unwrap();
        assert_eq!(tuples.len(), 1);
        
        // Delete tuple
        repo.delete_tuple(tuple.clone()).await.unwrap();
        assert!(!repo.tuple_exists(&tuple).await.unwrap());
    }
}

pub struct ZanzibarRepository {}