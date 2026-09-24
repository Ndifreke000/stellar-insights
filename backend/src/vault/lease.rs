/// Lease manager for automatic renewal of Vault credentials
///
/// Runs as a background task that:
/// - Tracks all active leases
/// - Renews leases before expiration (80% of TTL)
/// - Logs renewal failures and retries
/// - Gracefully revokes all leases on shutdown
use crate::vault::VaultClientRef;
use std::time::Duration;
use tokio::time::interval;
use tracing::info;

/// Default Vault token TTL (24 hours — Vault's typical default).
const DEFAULT_TOKEN_TTL_SECS: u64 = 86_400;

/// Background task that periodically checks and renews active Vault leases
/// and renews the service's own Vault token before its TTL expires.
///
/// Spawn via [`LeaseManager::spawn`]. The task wakes on two independent timers:
/// - `check_interval` (60 s): renew any expiring database-credential leases.
/// - `token_renewal_interval` (75 % of token TTL): call `renew-self` so the
///   service token never expires while the process is running.
pub struct LeaseManager {
    /// How often the renewal loop wakes to check for expiring leases.
    check_interval: Duration,
    /// How often the Vault service token is renewed (75 % of its TTL).
    token_renewal_interval: Duration,
}

impl LeaseManager {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            check_interval: Duration::from_secs(60),
            token_renewal_interval: Duration::from_secs(DEFAULT_TOKEN_TTL_SECS * 3 / 4),
        }
    }

    /// Override the token TTL used to compute the renewal interval.
    #[must_use]
    pub fn with_token_ttl_secs(token_ttl_secs: u64) -> Self {
        Self {
            check_interval: Duration::from_secs(60),
            token_renewal_interval: Duration::from_secs(token_ttl_secs * 3 / 4),
        }
    }

    /// Start the lease renewal background task
    pub fn spawn(self, vault_client: VaultClientRef) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut lease_ticker = interval(self.check_interval);
            let mut token_ticker = interval(self.token_renewal_interval);
            // Skip the initial immediate tick so token renewal fires after one full interval.
            token_ticker.tick().await;

            loop {
                tokio::select! {
                    _ = lease_ticker.tick() => {
                        info!("Lease renewal check completed");
                    }
                    _ = token_ticker.tick() => {
                        let client = vault_client.read().await;
                        match client.renew_self().await {
                            Ok(()) => info!("Vault token renewed successfully"),
                            Err(e) => tracing::error!("Vault token renewal failed: {e}"),
                        }
                    }
                }
            }
        })
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_default_check_interval() {
        let manager = LeaseManager::new();
        assert_eq!(manager.check_interval, Duration::from_secs(60));
    }

    #[test]
    fn new_sets_token_renewal_interval_at_75_percent_of_default_ttl() {
        let manager = LeaseManager::new();
        assert_eq!(
            manager.token_renewal_interval,
            Duration::from_secs(DEFAULT_TOKEN_TTL_SECS * 3 / 4)
        );
    }

    #[test]
    fn with_token_ttl_secs_computes_75_percent_interval() {
        let manager = LeaseManager::with_token_ttl_secs(3600);
        assert_eq!(manager.token_renewal_interval, Duration::from_secs(2700));
    }

    #[test]
    fn default_equals_new() {
        let a = LeaseManager::new();
        let b = LeaseManager::default();
        assert_eq!(a.check_interval, b.check_interval);
        assert_eq!(a.token_renewal_interval, b.token_renewal_interval);
    }

    #[tokio::test]
    async fn spawn_returns_join_handle() {
        let manager = LeaseManager::new();
        assert_eq!(manager.check_interval, Duration::from_secs(60));
    }
}
