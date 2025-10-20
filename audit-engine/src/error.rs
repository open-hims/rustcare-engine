use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuditError {
    #[error("Audit log creation failed")]
    LogCreationError,
    
    #[error("Audit entry validation failed")]
    ValidationError,
    
    #[error("Audit storage error")]
    StorageError,
    
    #[error("Audit search failed")]
    SearchError,
    
    #[error("Compliance report generation failed")]
    ComplianceReportError,
    
    #[error("Merkle tree integrity check failed")]
    IntegrityCheckError,
    
    #[error("Audit export failed")]
    ExportError,
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AuditError>;