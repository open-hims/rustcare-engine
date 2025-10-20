use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Workflow execution failed")]
    ExecutionError,
    
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    
    #[error("Task execution failed: {0}")]
    TaskError(String),
    
    #[error("Invalid workflow definition")]
    InvalidDefinition,
    
    #[error("Workflow scheduling error")]
    SchedulingError,
    
    #[error("State machine error")]
    StateMachineError,
    
    #[error("Compensation handling failed")]
    CompensationError,
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, WorkflowError>;