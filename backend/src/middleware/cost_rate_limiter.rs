//! Cost-based rate limiter for expensive operations.
//!
//! This middleware debits cost units from the client's budget before executing
//! expensive operations. If the client has insufficient budget, the request
//! is rejected with 429 Too Many Requests.
//!
//! Cost estimates are returned in response headers so clients can plan ahead.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::models::cost::{ClientTier, CostConfig, CostHeaders};
use crate::state::AppState;

/// Tracks cost budget per client using in-memory storage.
/// For production with multiple instances, replace with Redis-backed counters.
#[derive(Clone)]
pub struct CostRateLimiter {
    /// Per-client cost state: client_key -> (state, last_refresh)
    client_states: Arc<RwLock<HashMap<String, (crate::models::cost::ClientCostState, i64)>>>,
    /// Cost configuration
    config: CostConfig,
    /// Budget refresh interval in seconds
    budget_refresh_interval: Duration,
}

impl Default for CostRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl CostRateLimiter {
    /// Create a new cost rate limiter with default configuration.
    pub fn new() -> Self {
        Self {
            client_states: Arc::new(RwLock::new(HashMap::new())),
            config: CostConfig::default(),
            budget_refresh_interval: Duration::from_secs(24 * 3600), // Daily refresh
        }
    }

    /// Create a new cost rate limiter with custom configuration.
    pub fn with_config(config: CostConfig) -> Self {
        Self {
            client_states: Arc::new(RwLock::new(HashMap::new())),
            config,
            budget_refresh_interval: Duration::from_secs(24 * 3600),
        }
    }

    /// Get or create client cost state.
    async fn get_or_create_client_state(
        &self,
        client_key: &str,
        tier: ClientTier,
    ) -> crate::models::cost::ClientCostState {
        let mut states = self.client_states.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Check if we need to refresh the budget
        if let Some((state, last_refresh)) = states.get_mut(client_key) {
            let elapsed = now - *last_refresh;
            if elapsed >= self.budget_refresh_interval.as_secs() as i64 {
                // Refresh budget
                let budget = state.budget(&self.config);
                state.credit(budget - state.balance);
                state.last_updated = now;
                return state.clone();
            }
            return state.clone();
        }

        // Create new client state with initial budget
        let budget = tier.budget(&self.config);
        let state = crate::models::cost::ClientCostState::new(budget, tier);
        states.insert(client_key.to_string(), (state.clone(), now));
        state
    }

    /// Debit cost from client's budget.
    /// Returns Ok(true) if successful, Ok(false) if insufficient funds.
    pub async fn debit(
        &self,
        client_key: &str,
        tier: ClientTier,
        cost: f64,
    ) -> Result<bool, &'static str> {
        let mut state = self.get_or_create_client_state(client_key, tier).await;

        state.try_debit(cost).map_err(|_| "Failed to debit cost")
    }

    /// Check if client has sufficient budget for the given cost.
    pub async fn has_sufficient_budget(
        &self,
        client_key: &str,
        tier: ClientTier,
        cost: f64,
    ) -> bool {
        let state = self.get_or_create_client_state(client_key, tier).await;
        state.has_sufficient_balance(cost)
    }

    /// Get cost headers for a client.
    pub async fn get_cost_headers(
        &self,
        client_key: &str,
        tier: ClientTier,
        cost_estimate: f64,
    ) -> CostHeaders {
        let state = self.get_or_create_client_state(client_key, tier).await;
        CostHeaders::from_state(&state, cost_estimate)
    }

    /// Clean up stale client states to prevent memory growth.
    pub async fn cleanup(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let mut states = self.client_states.write().await;
        states.retain(|_, (_, last_refresh)| {
            now - *last_refresh < self.budget_refresh_interval.as_secs() as i64 * 2
        });
    }
}

/// Extract client identifier from request.
/// Falls back to client IP if no API key or user ID is available.
pub fn extract_client_key(
    api_key: Option<&str>,
    user_id: Option<&str>,
    client_ip: Option<&str>,
) -> String {
    if let Some(key) = api_key {
        format!("apikey:{}", key)
    } else if let Some(uid) = user_id {
        format!("user:{}", uid)
    } else if let Some(ip) = client_ip {
        format!("ip:{}", ip)
    } else {
        "anonymous".to_string()
    }
}

/// Get client tier from request context.
pub fn get_client_tier(api_key: Option<&str>, user_id: Option<&str>) -> ClientTier {
    if api_key.is_some() || user_id.is_some() {
        ClientTier::Authenticated
    } else {
        ClientTier::Anonymous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_debit_succeeds() {
        let limiter = CostRateLimiter::new();
        let result = limiter
            .debit("test-client", ClientTier::Anonymous, 50.0)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_debit_fails_insufficient() {
        let limiter = CostRateLimiter::new();
        // Try to debit more than the default anonymous budget (100)
        let result = limiter
            .debit("test-client-2", ClientTier::Anonymous, 150.0)
            .await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_different_clients_have_separate_budgets() {
        let limiter = CostRateLimiter::new();

        // Exhaust budget for client 1
        limiter
            .debit("client-1", ClientTier::Anonymous, 100.0)
            .await
            .unwrap();

        // Client 2 should still have full budget
        let result = limiter.debit("client-2", ClientTier::Anonymous, 50.0).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_authenticated_clients_have_more_budget() {
        let limiter = CostRateLimiter::new();

        // Anonymous client should have 100 budget
        let has_budget = limiter
            .has_sufficient_budget("anon", ClientTier::Anonymous, 150.0)
            .await;
        assert!(!has_budget);

        // Authenticated client should have 1000 budget
        let has_budget = limiter
            .has_sufficient_budget("auth", ClientTier::Authenticated, 500.0)
            .await;
        assert!(has_budget);
    }

    #[test]
    fn test_extract_client_key() {
        assert_eq!(
            extract_client_key(Some("api_key_123"), None, None),
            "apikey:api_key_123"
        );
        assert_eq!(
            extract_client_key(None, Some("user_456"), None),
            "user:user_456"
        );
        assert_eq!(
            extract_client_key(None, None, Some("192.168.1.1")),
            "ip:192.168.1.1"
        );
        assert_eq!(extract_client_key(None, None, None), "anonymous");
    }

    #[test]
    fn test_get_client_tier() {
        assert_eq!(
            get_client_tier(Some("api_key"), None),
            ClientTier::Authenticated
        );
        assert_eq!(
            get_client_tier(None, Some("user_id")),
            ClientTier::Authenticated
        );
        assert_eq!(get_client_tier(None, None), ClientTier::Anonymous);
    }
}
