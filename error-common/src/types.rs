use thiserror::Error;

/// Simplified error enum for common use cases
#[derive(Error, Debug)]
pub enum RustCareError {
    /// WebSocket-related errors
    #[error("WebSocket error: {0}")]
    WebSocketError(String),
    
    /// gRPC service errors
    #[error("gRPC error: {0}")]
    GrpcError(String),
    
    /// Network communication errors
    #[error("Network error: {0}")]
    NetworkError(String),
    
    /// Server configuration errors
    #[error("Server error: {0}")]
    ServerError(String),
    
    /// Authentication/authorization errors
    #[error("Authentication error: {0}")]
    AuthError(String),
    
    /// Database operation errors
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    /// Business logic errors
    #[error("Business logic error: {0}")]
    BusinessError(String),
    
    /// Validation errors
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    /// Internal system errors
    #[error("Internal error: {0}")]
    InternalError(String),
    
    /// External service errors
    #[error("External service error: {0}")]
    ExternalError(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    /// Generic error with context
    #[error("Error: {message}")]
    Generic { message: String },
    
    /// Wrapped external errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type alias for RustCare operations
pub type Result<T> = std::result::Result<T, RustCareError>;

/// Async logging function for errors
pub async fn log_error(context: &str, error: &RustCareError) {
    tracing::error!(
        context = context,
        error = %error,
        "RustCare error occurred"
    );
}