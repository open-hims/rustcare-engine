use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration source not found")]
    SourceNotFound,
    
    #[error("Configuration parsing failed")]
    ParseError,
    
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
    
    #[error("Configuration encryption failed")]
    EncryptionError,
    
    #[error("Configuration decryption failed")]
    DecryptionError,
    
    #[error("Configuration watcher error")]
    WatcherError,
    
    #[error("Configuration template error")]
    TemplateError,
    
    #[error("Remote configuration store connection failed")]
    RemoteStoreError,
    
    #[error("Configuration schema mismatch")]
    SchemaMismatch,
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ConfigError>;