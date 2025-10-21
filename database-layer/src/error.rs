use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
    
    #[error("Query error: {0}")]
    QueryError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("RLS policy violation")]
    RlsPolicyViolation,
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Migration error: {0}")]
    MigrationError(String),
    
    #[error("Database error: {0}")]
    SqlxError(#[from] sqlx::Error),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;