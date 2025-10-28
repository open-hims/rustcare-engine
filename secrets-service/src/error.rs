//! Error types for secrets service

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecretsError {
    #[error("Secret not found: {0}")]
    NotFound(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
    
    #[error("Provider error: {0}")]
    ProviderError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    #[error("Invalid secret format: {0}")]
    InvalidFormat(String),
    
    #[error("Secret expired: {0}")]
    Expired(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<reqwest::Error> for SecretsError {
    fn from(err: reqwest::Error) -> Self {
        SecretsError::NetworkError(err.to_string())
    }
}
