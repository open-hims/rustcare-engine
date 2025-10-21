//! Error types for the sync engine

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    #[error("Conflict detected: {0}")]
    Conflict(String),
    
    #[error("Sync operation failed: {0}")]
    SyncFailed(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Vector clock error: {0}")]
    VectorClock(String),
    
    #[error("CRDT operation error: {0}")]
    CrdtError(String),
    
    #[error("Encryption error: {0}")]
    Encryption(#[from] crypto::CryptoError),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for SyncError {
    fn from(err: serde_json::Error) -> Self {
        SyncError::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for SyncError {
    fn from(err: bincode::Error) -> Self {
        SyncError::Serialization(err.to_string())
    }
}

pub type SyncResult<T> = Result<T, SyncError>;
