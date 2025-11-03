use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Permission error: {0}")]
    Permission(String),
}

pub type McpResult<T> = Result<T, McpError>;

