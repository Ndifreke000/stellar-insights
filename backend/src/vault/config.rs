/// Vault configuration from environment variables
///
/// Expects:
/// - VAULT_ADDR: Base URL of Vault cluster (e.g., https://vault.example.com)
/// - VAULT_TOKEN: Authentication token (for development/testing)
/// - VAULT_NAMESPACE: Optional namespace path
/// - DB_ROLE: Database role for credential generation (e.g., stellar-app)
use crate::vault::VaultError;
use std::env;

#[derive(Clone, Debug)]
pub struct VaultConfig {
    pub vault_addr: String,
    pub vault_token: String,
    pub vault_namespace: Option<String>,
    pub db_role: String,
}

impl VaultConfig {
    /// Load Vault configuration from environment
    pub fn from_env() -> Result<Self, VaultError> {
        let vault_addr = env::var("VAULT_ADDR")
            .map_err(|_| VaultError::ConfigError("VAULT_ADDR not set".to_string()))?;

        let vault_token = env::var("VAULT_TOKEN")
            .map_err(|_| VaultError::ConfigError("VAULT_TOKEN not set".to_string()))?;

        let vault_namespace = env::var("VAULT_NAMESPACE").ok();

        let db_role = env::var("DB_ROLE").unwrap_or_else(|_| "stellar-app".to_string());

        Ok(VaultConfig {
            vault_addr,
            vault_token,
            vault_namespace,
            db_role,
        })
    }

    /// Create config with explicit values (for testing)
    pub fn new(vault_addr: String, vault_token: String, db_role: String) -> Self {
        VaultConfig {
            vault_addr,
            vault_token,
            vault_namespace: None,
            db_role,
        }
    }
}
