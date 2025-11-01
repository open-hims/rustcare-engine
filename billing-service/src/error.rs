use thiserror::Error;

#[derive(Error, Debug)]
pub enum BillingError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Charge capture error: {0}")]
    ChargeCapture(String),

    #[error("Claims generation error: {0}")]
    ClaimsGeneration(String),

    #[error("Payment processing error: {0}")]
    Payment(String),

    #[error("Insurance verification error: {0}")]
    InsuranceVerification(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type BillingResult<T> = Result<T, BillingError>;

