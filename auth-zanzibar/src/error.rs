use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZanzibarError {
    #[error("Invalid tuple: {0}")]
    InvalidTuple(String),
    
    #[error("Invalid subject: {0}")]
    InvalidSubject(String),
    
    #[error("Invalid object: {0}")]
    InvalidObject(String),
    
    #[error("Invalid relation: {0}")]
    InvalidRelation(String),
    
    #[error("Schema validation failed: {0}")]
    SchemaValidationFailed(String),
    
    #[error("Invalid schema: {0}")]
    InvalidSchema(String),
    
    #[error("Relation not found: {0}")]
    RelationNotFound(String),
    
    #[error("Object type not found: {0}")]
    ObjectTypeNotFound(String),
    
    #[error("Circular dependency detected")]
    CircularDependency,
    
    #[error("Maximum recursion depth exceeded")]
    MaxRecursionDepthExceeded,
    
    #[error("Consistency token invalid")]
    InvalidConsistencyToken,
    
    #[error("Repository error: {0}")]
    RepositoryError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ZanzibarError>;