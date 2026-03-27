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
            Self::ConfigError(msg) => write!(f, "Vault config error: {msg}"),
            Self::ClientError(msg) => write!(f, "Vault client error: {msg}"),
            Self::RequestError(msg) => write!(f, "Vault request error: {msg}"),
            Self::ParseError(msg) => write!(f, "Vault parse error: {msg}"),
            Self::VaultUnavailable => write!(f, "Vault is unavailable"),
            Self::SecretNotFound(path) => write!(f, "Secret not found: {path}"),
            Self::FieldNotFound(field) => write!(f, "Field not found: {field}"),
            Self::NoDataInSecret => write!(f, "No data in secret"),
            Self::CredentialsFailed(role) => {
                write!(f, "Failed to get credentials for role: {role}")
            }
            Self::LeaseRenewalFailed(lease_id) => {
                write!(f, "Failed to renew lease: {lease_id}")
            }
            Self::LeaseRevokeFailed(lease_id) => {
                write!(f, "Failed to revoke lease: {lease_id}")
            }
        }
    }
}

impl std::error::Error for VaultError {}
