use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountingError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Ledger error: {0}")]
    Ledger(String),

    #[error("General ledger error: {0}")]
    GeneralLedger(String),

    #[error("Accounts receivable error: {0}")]
    AccountsReceivable(String),

    #[error("Reconciliation error: {0}")]
    Reconciliation(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type AccountingResult<T> = Result<T, AccountingError>;

