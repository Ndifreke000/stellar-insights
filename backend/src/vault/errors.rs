/// Error types for Vault operations
use std::fmt;

#[derive(Debug, Clone)]
pub enum VaultError {
    /// Invalid or missing Vault configuration (e.g. missing env vars).
    ConfigError(String),
    /// Failed to build or initialize the HTTP client.
    ClientError(String),
    /// HTTP request to Vault failed.
    RequestError(String),
    /// Failed to parse the Vault response.
    ParseError(String),
    /// Vault cluster is unreachable or returned an unhealthy status.
    VaultUnavailable,
    /// Secret at the given path was not found.
    SecretNotFound(String),
    /// A required field was absent in the secret data.
    FieldNotFound(String),
    /// Secret response contained no data payload.
    NoDataInSecret,
    /// Dynamic credentials could not be generated for the given role.
    CredentialsFailed(String),
    /// Lease renewal request failed for the given lease ID.
    LeaseRenewalFailed(String),
    /// Lease revocation request failed for the given lease ID.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_config_error() {
        let e = VaultError::ConfigError("VAULT_ADDR not set".to_string());
        assert_eq!(e.to_string(), "Vault config error: VAULT_ADDR not set");
    }

    #[test]
    fn display_client_error() {
        let e = VaultError::ClientError("tls failure".to_string());
        assert_eq!(e.to_string(), "Vault client error: tls failure");
    }

    #[test]
    fn display_request_error() {
        let e = VaultError::RequestError("timeout".to_string());
        assert_eq!(e.to_string(), "Vault request error: timeout");
    }

    #[test]
    fn display_parse_error() {
        let e = VaultError::ParseError("missing field".to_string());
        assert_eq!(e.to_string(), "Vault parse error: missing field");
    }

    #[test]
    fn display_vault_unavailable() {
        assert_eq!(
            VaultError::VaultUnavailable.to_string(),
            "Vault is unavailable"
        );
    }

    #[test]
    fn display_secret_not_found() {
        let e = VaultError::SecretNotFound("secret/myapp".to_string());
        assert_eq!(e.to_string(), "Secret not found: secret/myapp");
    }

    #[test]
    fn display_field_not_found() {
        let e = VaultError::FieldNotFound("api_key".to_string());
        assert_eq!(e.to_string(), "Field not found: api_key");
    }

    #[test]
    fn display_no_data_in_secret() {
        assert_eq!(VaultError::NoDataInSecret.to_string(), "No data in secret");
    }

    #[test]
    fn display_credentials_failed() {
        let e = VaultError::CredentialsFailed("stellar-app".to_string());
        assert_eq!(
            e.to_string(),
            "Failed to get credentials for role: stellar-app"
        );
    }

    #[test]
    fn display_lease_renewal_failed() {
        let e = VaultError::LeaseRenewalFailed("lease/abc123".to_string());
        assert_eq!(e.to_string(), "Failed to renew lease: lease/abc123");
    }

    #[test]
    fn display_lease_revoke_failed() {
        let e = VaultError::LeaseRevokeFailed("lease/abc123".to_string());
        assert_eq!(e.to_string(), "Failed to revoke lease: lease/abc123");
    }

    #[test]
    fn vault_error_implements_std_error() {
        // Ensure the trait bound is satisfied — compile-time check.
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&VaultError::VaultUnavailable);
    }
}
