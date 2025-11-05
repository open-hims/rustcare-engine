use thiserror::Error;

#[derive(Error, Debug)]
pub enum EventBusError {
    #[error("Event serialization failed")]
    SerializationError,
    
    #[error("Event deserialization failed")]
    DeserializationError,
    
    #[error("Event publishing failed")]
    PublishError,
    
    #[error("Event subscription failed")]
    SubscriptionError,
    
    #[error("Event broker connection failed")]
    BrokerConnectionError,
    
    #[error("Event processing timeout")]
    ProcessingTimeout,
    
    #[error("Invalid event format")]
    InvalidEventFormat,
    
    #[error("Event queue full")]
    QueueFullError,
    
    #[error("Event handler not found")]
    HandlerNotFound,
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, EventBusError>;