//! Crate-wide error type. Errors are sanitized before crossing to the
//! frontend: secret material (keys, passphrases, private prompt text) must
//! never appear in error messages or logs.

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("vault is locked")]
    VaultLocked,
    #[error("vault operation failed: {0}")]
    Vault(String),
    #[error("cryptographic operation failed")]
    Crypto,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error")]
    Database,
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("configuration error: {0}")]
    Config(String),
}

// Note: rusqlite::Error is intentionally mapped to the opaque `Database`
// variant so SQL details do not leak to the frontend or logs. Internal callers
// that need detail can log at debug level before converting.
impl From<rusqlite::Error> for AppError {
    fn from(_: rusqlite::Error) -> Self {
        AppError::Database
    }
}
