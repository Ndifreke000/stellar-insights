/// High-level secrets service for application secret retrieval
///
/// Provides a single interface for fetching application secrets from Vault
/// with fallback to environment variables for development environments.

use crate::vault::{VaultClient, VaultError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application secrets needed by payraider
#[derive(Debug, Clone)]
pub struct ApplicationSecrets {
    pub jwt_secret: String,
    pub encryption_key: String,
    pub database_password: Option<String>,
}

/// Secrets service for managing application secret lifecycle
pub struct SecretsService {
    vault_client: Option<Arc<RwLock<VaultClient>>>,
}

impl SecretsService {
    /// Create a new secrets service
    ///
    /// Attempts to initialize Vault client if VAULT_ADDR is set,
    /// otherwise falls back to environment variables for development.
    pub async fn new() -> Result<Self, VaultError> {
        let vault_client = if std::env::var("VAULT_ADDR").is_ok() {
            let client = VaultClient::new(crate::vault::VaultConfig::from_env()?).await?;
            Some(Arc::new(RwLock::new(client)))
        } else {
            None
        };

        Ok(Self { vault_client })
    }

    /// Fetch application secrets from Vault or environment
    pub async fn get_secrets(&self) -> Result<ApplicationSecrets, VaultError> {
        if let Some(vault) = &self.vault_client {
            self.fetch_from_vault(vault).await
        } else {
            self.fetch_from_env()
        }
    }

    /// Fetch JWT secret directly from Vault or environment
    pub async fn get_jwt_secret(&self) -> Result<String, VaultError> {
        let secrets = self.get_secrets().await?;
        Ok(secrets.jwt_secret)
    }

    /// Synchronous helper to fetch secrets directly from environment
    pub fn from_env() -> Result<ApplicationSecrets, VaultError> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| VaultError::ConfigError("JWT_SECRET not set".to_string()))?;

        // Previously fell back to a hardcoded key baked into source when
        // ENCRYPTION_KEY was unset -- inconsistent with jwt_secret just
        // above (which fails hard) and with fetch_from_env below (its own
        // instance-method twin, which already required this var). Every
        // secret this key protects (2FA TOTP secrets, via crypto::
        // CryptoService::from_env) was silently encrypted with a key
        // visible to anyone reading this repo whenever the env var was
        // merely unset, rather than failing loudly.
        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| VaultError::ConfigError("ENCRYPTION_KEY not set".to_string()))?;

        let database_password = std::env::var("DATABASE_PASSWORD").ok();

        Ok(ApplicationSecrets {
            jwt_secret,
            encryption_key,
            database_password,
        })
    }

    /// Fetch secrets from Vault
    async fn fetch_from_vault(
        &self,
        vault: &Arc<RwLock<VaultClient>>,
    ) -> Result<ApplicationSecrets, VaultError> {
        let client = vault.read().await;

        let jwt_secret = client
            .read_secret("app/secrets", Some("jwt_secret"))
            .await?;

        let encryption_key = client
            .read_secret("app/secrets", Some("encryption_key"))
            .await?;

        // Database password is optional (can use dynamic credentials)
        let database_password = client
            .read_secret("app/secrets", Some("database_password"))
            .await
            .ok();

        Ok(ApplicationSecrets {
            jwt_secret,
            encryption_key,
            database_password,
        })
    }

    /// Fetch secrets from environment variables (development fallback)
    fn fetch_from_env(&self) -> Result<ApplicationSecrets, VaultError> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| VaultError::ConfigError("JWT_SECRET not set".to_string()))?;

        let encryption_key = std::env::var("ENCRYPTION_KEY")
            .map_err(|_| VaultError::ConfigError("ENCRYPTION_KEY not set".to_string()))?;

        let database_password = std::env::var("DATABASE_PASSWORD").ok();

        Ok(ApplicationSecrets {
            jwt_secret,
            encryption_key,
            database_password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn secrets_service_fallback_to_env() {
        // Set up environment variables for this test
        std::env::set_var("JWT_SECRET", "test-jwt-secret-1234567890");
        std::env::set_var("ENCRYPTION_KEY", "test-encryption-key-1234567890");
        std::env::remove_var("VAULT_ADDR");

        let service = SecretsService::new().await.unwrap();
        let secrets = service.get_secrets().await.unwrap();

        assert_eq!(secrets.jwt_secret, "test-jwt-secret-1234567890");
        assert_eq!(secrets.encryption_key, "test-encryption-key-1234567890");

        // Clean up
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn application_secrets_structure() {
        let secrets = ApplicationSecrets {
            jwt_secret: "secret".to_string(),
            encryption_key: "key".to_string(),
            database_password: Some("password".to_string()),
        };

        assert_eq!(secrets.jwt_secret, "secret");
        assert_eq!(secrets.encryption_key, "key");
        assert_eq!(secrets.database_password, Some("password".to_string()));
    }

    #[test]
    fn application_secrets_database_password_optional() {
        let secrets = ApplicationSecrets {
            jwt_secret: "secret".to_string(),
            encryption_key: "key".to_string(),
            database_password: None,
        };

        assert_eq!(secrets.jwt_secret, "secret");
        assert_eq!(secrets.encryption_key, "key");
        assert_eq!(secrets.database_password, None);
    }

    #[tokio::test]
    async fn secrets_service_get_jwt_secret() {
        std::env::set_var("JWT_SECRET", "test-jwt-secret");
        std::env::set_var("ENCRYPTION_KEY", "test-encryption-key");
        std::env::remove_var("VAULT_ADDR");

        let service = SecretsService::new().await.unwrap();
        let jwt_secret = service.get_jwt_secret().await.unwrap();

        assert_eq!(jwt_secret, "test-jwt-secret");

        // Clean up
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[tokio::test]
    async fn secrets_service_fetch_from_env() {
        std::env::set_var("JWT_SECRET", "env-jwt-secret");
        std::env::set_var("ENCRYPTION_KEY", "env-encryption-key");
        std::env::remove_var("VAULT_ADDR");

        let service = SecretsService::new().await.unwrap();
        let secrets = service.fetch_from_env().unwrap();

        assert_eq!(secrets.jwt_secret, "env-jwt-secret");
        assert_eq!(secrets.encryption_key, "env-encryption-key");

        // Clean up
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn secrets_service_from_env() {
        std::env::set_var("JWT_SECRET", "env-jwt-secret");
        std::env::set_var("ENCRYPTION_KEY", "env-encryption-key");

        let secrets = SecretsService::from_env().unwrap();

        assert_eq!(secrets.jwt_secret, "env-jwt-secret");
        assert_eq!(secrets.encryption_key, "env-encryption-key");

        // Clean up
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn secrets_service_from_env_fails_when_jwt_secret_missing() {
        std::env::remove_var("JWT_SECRET");
        std::env::set_var("ENCRYPTION_KEY", "test-key");

        let result = SecretsService::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("JWT_SECRET"));

        // Clean up
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn secrets_service_from_env_fails_when_encryption_key_missing() {
        std::env::set_var("JWT_SECRET", "test-secret");
        std::env::remove_var("ENCRYPTION_KEY");

        let result = SecretsService::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("ENCRYPTION_KEY"));

        // Clean up
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("ENCRYPTION_KEY");
    }

    #[test]
    fn application_secrets_is_send_and_sync() {
        let secrets = ApplicationSecrets {
            jwt_secret: "secret".to_string(),
            encryption_key: "key".to_string(),
            database_password: Some("password".to_string()),
        };

        // Compile-time check that ApplicationSecrets is Send and Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<ApplicationSecrets>();
        assert_sync::<ApplicationSecrets>();
        let _ = secrets;
    }

    #[tokio::test]
    async fn secrets_service_is_send_and_sync() {
        std::env::set_var("JWT_SECRET", "test-secret");
        std::env::set_var("ENCRYPTION_KEY", "test-key");
        std::env::remove_var("VAULT_ADDR");

        let service = SecretsService::new().await.unwrap();

        // Compile-time check that SecretsService is Send and Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<SecretsService>();
        assert_sync::<SecretsService>();

        let _ = service;
    }
}
