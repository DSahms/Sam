//! Crate-wide error type. Errors are sanitized before crossing to the
//! frontend: secret material (keys, passphrases, private prompt text) must
//! never appear in error messages or logs.

use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

/// Serialization for the Tauri boundary. We serialize into an object with a
/// stable `kind` string and a human `message`. The message comes from the
/// `Display` impl, which is deliberately generic ("cryptographic operation
/// failed", "database error") and never includes secret material.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let kind = match self {
            AppError::VaultLocked => "vault_locked",
            AppError::Vault(_) => "vault",
            AppError::Crypto => "crypto",
            AppError::InvalidArgument(_) => "invalid_argument",
            AppError::NotFound(_) => "not_found",
            AppError::Io(_) => "io",
            AppError::Database => "database",
            AppError::Serde(_) => "serde",
            AppError::Config(_) => "config",
        };
        let mut st = serializer.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", kind)?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

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
