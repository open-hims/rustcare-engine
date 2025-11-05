use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmailError {
    #[error("Send failed: {0}")]
    SendFailed(String),
    
    #[error("Template error: {0}")]
    TemplateError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Compliance violation: {0}")]
    ComplianceViolation(String),
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type EmailResult<T> = Result<T, EmailError>;