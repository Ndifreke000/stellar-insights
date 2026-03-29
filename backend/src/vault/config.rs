/// Vault configuration from environment variables
///
/// Expects:
/// - `VAULT_ADDR`: Base URL of Vault cluster (e.g., <https://vault.example.com>)
/// - `VAULT_TOKEN`: Authentication token (for development/testing)
/// - `VAULT_NAMESPACE`: Optional namespace path
/// - `DB_ROLE`: Database role for credential generation (e.g., stellar-app)
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

        Ok(Self {
            vault_addr,
            vault_token,
            vault_namespace,
            db_role,
        })
    }

    /// Create config with explicit values (for testing)
    #[must_use]
    pub const fn new(vault_addr: String, vault_token: String, db_role: String) -> Self {
        Self {
            vault_addr,
            vault_token,
            vault_namespace: None,
            db_role,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn new_builds_config_with_correct_fields() {
        let config = VaultConfig::new(
            "https://vault.example.com".to_string(),
            "s.token123".to_string(),
            "stellar-app".to_string(),
        );
        assert_eq!(config.vault_addr, "https://vault.example.com");
        assert_eq!(config.vault_token, "s.token123");
        assert_eq!(config.db_role, "stellar-app");
        assert!(config.vault_namespace.is_none());
    }

    #[test]
    fn from_env_returns_error_when_vault_addr_missing() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
        std::env::remove_var("VAULT_NAMESPACE");
        std::env::remove_var("DB_ROLE");

        let result = VaultConfig::from_env();
        assert!(result.is_err());
        let err = match result {
            Ok(_) => panic!("expected VAULT_ADDR error"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("VAULT_ADDR"));
    }

    #[test]
    fn from_env_returns_error_when_vault_token_missing() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("VAULT_ADDR", "https://vault.example.com");
        std::env::remove_var("VAULT_TOKEN");
        std::env::remove_var("VAULT_NAMESPACE");
        std::env::remove_var("DB_ROLE");

        let result = VaultConfig::from_env();
        assert!(result.is_err());
        let err = match result {
            Ok(_) => panic!("expected VAULT_TOKEN error"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("VAULT_TOKEN"));

        std::env::remove_var("VAULT_ADDR");
    }

    #[test]
    fn from_env_uses_default_db_role() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("VAULT_ADDR", "https://vault.example.com");
        std::env::set_var("VAULT_TOKEN", "s.testtoken");
        std::env::remove_var("VAULT_NAMESPACE");
        std::env::remove_var("DB_ROLE");

        let config = match VaultConfig::from_env() {
            Ok(config) => config,
            Err(err) => panic!("expected config from env, got {err}"),
        };
        assert_eq!(config.db_role, "stellar-app");
        assert!(config.vault_namespace.is_none());

        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
    }

    #[test]
    fn from_env_reads_optional_namespace_and_role() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::env::set_var("VAULT_ADDR", "https://vault.example.com");
        std::env::set_var("VAULT_TOKEN", "s.testtoken");
        std::env::set_var("VAULT_NAMESPACE", "admin");
        std::env::set_var("DB_ROLE", "custom-role");

        let config = match VaultConfig::from_env() {
            Ok(config) => config,
            Err(err) => panic!("expected config from env, got {err}"),
        };
        assert_eq!(config.vault_namespace, Some("admin".to_string()));
        assert_eq!(config.db_role, "custom-role");

        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
        std::env::remove_var("VAULT_NAMESPACE");
        std::env::remove_var("DB_ROLE");
    }
}
