use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::fmt;

/// Represents a subject in the authorization system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Subject {
    pub namespace: String,
    pub object_type: String,
    pub object_id: String,
    pub relation: Option<String>,
}

impl Subject {
    pub fn user(user_id: &str) -> Self {
        Self {
            namespace: "user".to_string(),
            object_type: "user".to_string(),
            object_id: user_id.to_string(),
            relation: None,
        }
    }

    pub fn group(group_id: &str) -> Self {
        Self {
            namespace: "group".to_string(),
            object_type: "group".to_string(),
            object_id: group_id.to_string(),
            relation: None,
        }
    }

    pub fn service(service_id: &str) -> Self {
        Self {
            namespace: "service".to_string(),
            object_type: "service".to_string(),
            object_id: service_id.to_string(),
            relation: None,
        }
    }

    pub fn userset(object_type: &str, object_id: &str, relation: &str) -> Self {
        Self {
            namespace: "object".to_string(),
            object_type: object_type.to_string(),
            object_id: object_id.to_string(),
            relation: Some(relation.to_string()),
        }
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref relation) = self.relation {
            write!(f, "{}:{}#{}#{}", self.namespace, self.object_type, self.object_id, relation)
        } else {
            write!(f, "{}:{}#{}", self.namespace, self.object_type, self.object_id)
        }
    }
}

/// Represents an object (resource) in the authorization system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Object {
    pub namespace: String,
    pub object_type: String,
    pub object_id: String,
}

impl Object {
    pub fn new(object_type: &str, object_id: &str) -> Self {
        Self {
            namespace: "object".to_string(),
            object_type: object_type.to_string(),
            object_id: object_id.to_string(),
        }
    }
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}#{}", self.namespace, self.object_type, self.object_id)
    }
}

/// Represents a relation (permission type) in the authorization system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Relation {
    pub name: String,
}

impl Relation {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl fmt::Display for Relation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents a relationship tuple: subject has relation to object
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tuple {
    pub subject: Subject,
    pub relation: Relation,
    pub object: Object,
    pub created_at: DateTime<Utc>,
}

impl Tuple {
    pub fn new(subject: Subject, relation: Relation, object: Object) -> Self {
        Self {
            subject,
            relation,
            object,
            created_at: Utc::now(),
        }
    }
}

impl fmt::Display for Tuple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.subject, self.relation, self.object)
    }
}

/// Authorization check request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    pub subject: Subject,
    pub relation: Relation,
    pub object: Object,
    pub context: Option<serde_json::Value>,
}

/// Authorization check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    pub allowed: bool,
    pub debug_trace: Option<Vec<String>>,
}

/// Expand request to get all subjects with a relation to an object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandRequest {
    pub relation: Relation,
    pub object: Object,
    pub max_depth: Option<u32>,
}

/// Subject tree node for expand responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectTree {
    pub subject: Subject,
    pub children: Vec<SubjectTree>,
}

/// Batch write request for multiple tuples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRequest {
    pub writes: Vec<Tuple>,
    pub deletes: Vec<Tuple>,
}

/// Consistency token for read-after-write consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyToken {
    pub token: String,
    pub created_at: DateTime<Utc>,
}