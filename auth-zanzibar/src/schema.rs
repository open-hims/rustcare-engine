use crate::{error::ZanzibarError, models::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Permission schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Namespace definitions (e.g., "document", "folder", "organization")
    pub namespaces: HashMap<String, NamespaceDefinition>,
}

impl Default for Schema {
    fn default() -> Self {
        Self::healthcare_schema()
    }
}

impl Schema {
    pub fn new() -> Self {
        Self {
            namespaces: HashMap::new(),
        }
    }
    
    /// Create a healthcare-specific schema with HIPAA-aligned permissions
    pub fn healthcare_schema() -> Self {
        let mut namespaces = HashMap::new();
        
        // Patient records
        namespaces.insert("patient".to_string(), NamespaceDefinition {
            name: "patient".to_string(),
            relations: vec![
                RelationDefinition {
                    name: "owner".to_string(),
                    inherits_from: None,
                    description: "Full access to patient record".to_string(),
                },
                RelationDefinition {
                    name: "provider".to_string(),
                    inherits_from: Some("viewer".to_string()),
                    description: "Healthcare provider with treatment access".to_string(),
                },
                RelationDefinition {
                    name: "viewer".to_string(),
                    inherits_from: None,
                    description: "Read-only access to patient record".to_string(),
                },
                RelationDefinition {
                    name: "read_phi".to_string(),
                    inherits_from: None,
                    description: "Permission to read PHI fields".to_string(),
                },
            ],
        });
        
        // Documents (medical records, lab results, etc.)
        namespaces.insert("document".to_string(), NamespaceDefinition {
            name: "document".to_string(),
            relations: vec![
                RelationDefinition {
                    name: "owner".to_string(),
                    inherits_from: None,
                    description: "Full control over document".to_string(),
                },
                RelationDefinition {
                    name: "editor".to_string(),
                    inherits_from: Some("viewer".to_string()),
                    description: "Can edit document".to_string(),
                },
                RelationDefinition {
                    name: "viewer".to_string(),
                    inherits_from: None,
                    description: "Can view document".to_string(),
                },
            ],
        });
        
        // Organization (multi-tenant)
        namespaces.insert("organization".to_string(), NamespaceDefinition {
            name: "organization".to_string(),
            relations: vec![
                RelationDefinition {
                    name: "admin".to_string(),
                    inherits_from: Some("member".to_string()),
                    description: "Organization administrator".to_string(),
                },
                RelationDefinition {
                    name: "member".to_string(),
                    inherits_from: None,
                    description: "Organization member".to_string(),
                },
            ],
        });
        
        // Roles
        namespaces.insert("role".to_string(), NamespaceDefinition {
            name: "role".to_string(),
            relations: vec![
                RelationDefinition {
                    name: "member".to_string(),
                    inherits_from: None,
                    description: "Member of this role".to_string(),
                },
            ],
        });
        
        Self { namespaces }
    }
    
    /// Validate that a tuple conforms to the schema
    pub fn validate_tuple(&self, tuple: &Tuple) -> Result<(), ZanzibarError> {
        let namespace = self.namespaces.get(&tuple.object.object_type)
            .ok_or_else(|| ZanzibarError::InvalidTuple(
                format!("Unknown object type: {}", tuple.object.object_type)
            ))?;
        
        let relation_exists = namespace.relations.iter()
            .any(|r| r.name == tuple.relation.name);
        
        if !relation_exists {
            return Err(ZanzibarError::InvalidTuple(
                format!("Unknown relation '{}' for object type '{}'", 
                    tuple.relation.name, tuple.object.object_type)
            ));
        }
        
        Ok(())
    }
    
    /// Validate the entire schema is well-formed
    pub fn validate(&self) -> Result<(), ZanzibarError> {
        for (name, namespace) in &self.namespaces {
            if name != &namespace.name {
                return Err(ZanzibarError::InvalidSchema(
                    format!("Namespace key '{}' doesn't match name '{}'", name, namespace.name)
                ));
            }
            
            // Validate relation inheritance
            for relation in &namespace.relations {
                if let Some(ref parent) = relation.inherits_from {
                    if !namespace.relations.iter().any(|r| &r.name == parent) {
                        return Err(ZanzibarError::InvalidSchema(
                            format!("Relation '{}' inherits from unknown relation '{}'", 
                                relation.name, parent)
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Get permission definition for a namespace and relation
    pub fn get_permission(&self, object_type: &str, relation: &str) -> Option<PermissionDefinition> {
        let namespace = self.namespaces.get(object_type)?;
        let relation_def = namespace.relations.iter()
            .find(|r| r.name == relation)?;
        
        Some(PermissionDefinition {
            name: relation.to_string(),
            inherits_from: relation_def.inherits_from.clone(),
        })
    }
}

/// Definition of a namespace (object type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceDefinition {
    pub name: String,
    pub relations: Vec<RelationDefinition>,
}

/// Definition of a relation within a namespace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDefinition {
    pub name: String,
    pub inherits_from: Option<String>,
    pub description: String,
}

/// Permission definition with inheritance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDefinition {
    pub name: String,
    pub inherits_from: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_healthcare_schema() {
        let schema = Schema::healthcare_schema();
        assert!(schema.namespaces.contains_key("patient"));
        assert!(schema.namespaces.contains_key("document"));
        assert!(schema.validate().is_ok());
    }
    
    #[test]
    fn test_validate_tuple() {
        let schema = Schema::healthcare_schema();
        let tuple = Tuple::new(
            Subject::user("alice"),
            Relation::new("owner"),
            Object::new("patient", "patient1"),
        );
        assert!(schema.validate_tuple(&tuple).is_ok());
        
        let invalid_tuple = Tuple::new(
            Subject::user("alice"),
            Relation::new("invalid_relation"),
            Object::new("patient", "patient1"),
        );
        assert!(schema.validate_tuple(&invalid_tuple).is_err());
    }
}
