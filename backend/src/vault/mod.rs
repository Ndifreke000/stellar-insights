/// Vault secrets management module for secure credential handling
///
/// This module provides integration with HashiCorp Vault for:
/// - Static secrets (API keys, OAuth credentials)
/// - Dynamic database credentials (PostgreSQL)
/// - Lease lifecycle management and renewal
/// - Audit logging of secret access
pub mod client;
pub mod config;
pub mod errors;
pub mod lease;

pub use client::VaultClient;
pub use config::VaultConfig;
pub use errors::VaultError;
pub use lease::LeaseManager;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Vault client instance shared across the application
pub type VaultClientRef = Arc<RwLock<VaultClient>>;

/// Initialize Vault client from environment configuration
pub async fn init_vault() -> Result<VaultClientRef, VaultError> {
    let config = VaultConfig::from_env()?;
    let client = VaultClient::new(config).await?;
    Ok(Arc::new(RwLock::new(client)))
}
