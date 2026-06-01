/// Error types for Vault operations
use std::fmt;

#[derive(Debug, Clone)]
pub enum VaultError {
    ConfigError(String),
    ClientError(String),
    RequestError(String),
    ParseError(String),
    VaultUnavailable,
    SecretNotFound(String),
    FieldNotFound(String),
    NoDataInSecret,
    CredentialsFailed(String),
    LeaseRenewalFailed(String),
    LeaseRevokeFailed(String),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::ConfigError(msg) => write!(f, "Vault config error: {}", msg),
            VaultError::ClientError(msg) => write!(f, "Vault client error: {}", msg),
            VaultError::RequestError(msg) => write!(f, "Vault request error: {}", msg),
            VaultError::ParseError(msg) => write!(f, "Vault parse error: {}", msg),
            VaultError::VaultUnavailable => write!(f, "Vault is unavailable"),
            VaultError::SecretNotFound(path) => write!(f, "Secret not found: {}", path),
            VaultError::FieldNotFound(field) => write!(f, "Field not found: {}", field),
            VaultError::NoDataInSecret => write!(f, "No data in secret"),
            VaultError::CredentialsFailed(role) => {
                write!(f, "Failed to get credentials for role: {}", role)
            }
            VaultError::LeaseRenewalFailed(lease_id) => {
                write!(f, "Failed to renew lease: {}", lease_id)
            }
            VaultError::LeaseRevokeFailed(lease_id) => {
                write!(f, "Failed to revoke lease: {}", lease_id)
            }
        }
    }
}

impl std::error::Error for VaultError {}
