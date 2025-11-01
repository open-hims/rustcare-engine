use thiserror::Error;

#[derive(Error, Debug)]
pub enum InsuranceError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Eligibility check error: {0}")]
    Eligibility(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Insurance provider error: {0}")]
    Provider(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type InsuranceResult<T> = Result<T, InsuranceError>;

