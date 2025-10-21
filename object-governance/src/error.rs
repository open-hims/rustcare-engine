use thiserror::Error;

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("Policy validation error: {0}")]
    PolicyValidation(String),

    #[error("Classification error: {0}")]
    Classification(String),

    #[error("Retention policy error: {0}")]
    Retention(String),

    #[error("Data discovery error: {0}")]
    Discovery(String),

    #[error("Privacy compliance error: {0}")]
    Privacy(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Audit error: {0}")]
    Audit(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Invalid object: {0}")]
    InvalidObject(String),

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Lifecycle error: {0}")]
    Lifecycle(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Uuid(#[from] uuid::Error),

    #[error(transparent)]
    ZanzibarError(#[from] auth_zanzibar::error::ZanzibarError),

    #[error(transparent)]
    AuditError(#[from] audit_engine::error::AuditError),

    #[error(transparent)]
    CryptoError(#[from] crypto::error::CryptoError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type GovernanceResult<T> = Result<T, GovernanceError>;
