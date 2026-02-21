/// VaultClient provides secure access to secrets stored in HashiCorp Vault
///
/// Supports:
/// - Static secrets from KV v2 secrets engine
/// - Dynamic database credentials with automatic renewal
/// - Lease tracking and background renewal
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::vault::{VaultConfig, VaultError};

/// Vault Client for secret operations
pub struct VaultClient {
    http_client: reqwest::Client,
    config: VaultConfig,
    lease_manager: Arc<RwLock<HashMap<String, LeaseInfo>>>,
}

/// Information about an active Vault lease
#[derive(Clone, Debug)]
struct LeaseInfo {
    lease_id: String,
    lease_duration: u64,
    renewable: bool,
    created_at: std::time::Instant,
}

/// Response from Vault KV v2 read operation
#[derive(Debug, Deserialize)]
struct KvReadResponse {
    request_id: String,
    lease_id: String,
    lease_duration: u64,
    renewable: bool,
    data: KvData,
}

#[derive(Debug, Deserialize)]
struct KvData {
    data: HashMap<String, serde_json::Value>,
    metadata: serde_json::Value,
}

/// Response from Vault database credentials operation
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseCredentials {
    pub username: String,
    pub password: String,
    pub ttl: u64,
}

/// Response from Vault secret read
#[derive(Debug, Deserialize)]
struct VaultSecretResponse {
    request_id: String,
    lease_id: String,
    lease_duration: u64,
    renewable: bool,
    data: serde_json::Value,
}

impl VaultClient {
    /// Create a new Vault client
    pub async fn new(config: VaultConfig) -> Result<Self, VaultError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| VaultError::ClientError(e.to_string()))?;

        // Test connection to Vault
        let client = VaultClient {
            http_client,
            config,
            lease_manager: Arc::new(RwLock::new(HashMap::new())),
        };

        client.health_check().await?;
        Ok(client)
    }

    /// Check Vault health
    async fn health_check(&self) -> Result<(), VaultError> {
        let url = format!("{}/v1/sys/health", self.config.vault_addr);
        match self.http_client.get(&url).send().await {
            Ok(resp) => {
                if resp.status().is_success() || resp.status().as_u16() == 473 {
                    Ok(())
                } else {
                    Err(VaultError::VaultUnavailable)
                }
            }
            Err(_) => Err(VaultError::VaultUnavailable),
        }
    }

    /// Read a static secret from KV v2
    ///
    /// # Arguments
    /// * `path` - Secret path (e.g., "stellar/jwt_secret")
    /// * `field` - Specific field to extract (e.g., "value"), None returns all fields
    ///
    /// # Returns
    /// Secret value as String
    pub async fn read_secret(&self, path: &str, field: Option<&str>) -> Result<String, VaultError> {
        let url = format!(
            "{}/v1/data/{}",
            self.config.vault_addr,
            path.trim_start_matches('/')
        );

        let resp = self
            .http_client
            .get(&url)
            .header("X-Vault-Token", &self.config.vault_token)
            .send()
            .await
            .map_err(|e| VaultError::RequestError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(VaultError::SecretNotFound(path.to_string()));
        }

        let secret: KvReadResponse = resp
            .json()
            .await
            .map_err(|e| VaultError::ParseError(e.to_string()))?;

        if let Some(field_name) = field {
            secret
                .data
                .data
                .get(field_name)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| VaultError::FieldNotFound(field_name.to_string()))
        } else {
            // Return first available value if no field specified
            secret
                .data
                .data
                .values()
                .next()
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| VaultError::NoDataInSecret)
        }
    }

    /// Request dynamic PostgreSQL database credentials
    ///
    /// # Arguments
    /// * `role` - Database role (e.g., "stellar-app") must exist in Vault
    ///
    /// # Returns
    /// (username, password) tuple with 1-hour TTL
    pub async fn get_database_credentials(
        &self,
        role: &str,
    ) -> Result<DatabaseCredentials, VaultError> {
        let url = format!("{}/v1/database/creds/{}", self.config.vault_addr, role);

        let resp = self
            .http_client
            .get(&url)
            .header("X-Vault-Token", &self.config.vault_token)
            .send()
            .await
            .map_err(|e| VaultError::RequestError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(VaultError::CredentialsFailed(role.to_string()));
        }

        let secret: VaultSecretResponse = resp
            .json()
            .await
            .map_err(|e| VaultError::ParseError(e.to_string()))?;

        let data = &secret.data;
        let username = data["username"]
            .as_str()
            .ok_or(VaultError::ParseError("Missing username".to_string()))?
            .to_string();

        let password = data["password"]
            .as_str()
            .ok_or(VaultError::ParseError("Missing password".to_string()))?
            .to_string();

        // Store lease for renewal
        let mut leases = self.lease_manager.write().await;
        leases.insert(
            secret.lease_id.clone(),
            LeaseInfo {
                lease_id: secret.lease_id,
                lease_duration: secret.lease_duration,
                renewable: secret.renewable,
                created_at: std::time::Instant::now(),
            },
        );

        Ok(DatabaseCredentials {
            username,
            password,
            ttl: secret.lease_duration,
        })
    }

    /// Renew a lease before it expires
    pub async fn renew_lease(&self, lease_id: &str) -> Result<(), VaultError> {
        let url = format!("{}/v1/sys/leases/renew", self.config.vault_addr);

        let body = serde_json::json!({
            "lease_id": lease_id,
        });

        let resp = self
            .http_client
            .put(&url)
            .header("X-Vault-Token", &self.config.vault_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| VaultError::RequestError(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(VaultError::LeaseRenewalFailed(lease_id.to_string()))
        }
    }

    /// Revoke a lease (e.g., when shutting down)
    pub async fn revoke_lease(&self, lease_id: &str) -> Result<(), VaultError> {
        let url = format!("{}/v1/sys/leases/revoke", self.config.vault_addr);

        let body = serde_json::json!({
            "lease_id": lease_id,
        });

        let resp = self
            .http_client
            .put(&url)
            .header("X-Vault-Token", &self.config.vault_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| VaultError::RequestError(e.to_string()))?;

        if resp.status().is_success() {
            // Remove from tracking
            let mut leases = self.lease_manager.write().await;
            leases.remove(lease_id);
            Ok(())
        } else {
            Err(VaultError::LeaseRevokeFailed(lease_id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lease_info_creation() {
        let lease = LeaseInfo {
            lease_id: "test".to_string(),
            lease_duration: 3600,
            renewable: true,
            created_at: std::time::Instant::now(),
        };
        assert_eq!(lease.lease_duration, 3600);
        assert!(lease.renewable);
    }
}
