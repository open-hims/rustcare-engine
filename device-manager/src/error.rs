use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeviceError {
    #[error("Device not found: {0}")]
    NotFound(String),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("Communication error: {0}")]
    CommunicationError(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Parsing error: {0}")]
    ParsingError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
        #[error("Device is busy: {0}")]
    Busy(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Plugin error: {0}")]
    PluginError(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, DeviceError>;
