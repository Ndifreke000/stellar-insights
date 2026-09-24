use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::network_context_middleware::NetworkContext;

#[derive(Clone, Debug)]
pub struct ClientRateLimit {
    requests: u32,
    reset_time: DateTime<Utc>,
}

#[derive(Clone)]
pub struct RateLimitingByClient {
    clients: Arc<RwLock<HashMap<String, ClientRateLimit>>>,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimitingByClient {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }

    pub async fn check_limit(&self, client_id: &str, _context: &NetworkContext) -> Result<bool> {
        let mut clients = self.clients.write().await;
        let now = Utc::now();

        let limit = clients
            .entry(client_id.to_string())
            .or_insert_with(|| ClientRateLimit {
                requests: 0,
                reset_time: now + chrono::Duration::seconds(self.window_secs as i64),
            });

        if now >= limit.reset_time {
            limit.requests = 0;
            limit.reset_time = now + chrono::Duration::seconds(self.window_secs as i64);
        }

        if limit.requests < self.max_requests {
            limit.requests += 1;
            tracing::debug!(
                client_id = client_id,
                requests = limit.requests,
                max = self.max_requests,
                "Rate limit check passed"
            );
            Ok(true)
        } else {
            tracing::warn!(client_id = client_id, "Rate limit exceeded for client");
            Ok(false)
        }
    }

    pub async fn get_remaining(&self, client_id: &str) -> Result<u32> {
        let clients = self.clients.read().await;
        Ok(clients
            .get(client_id)
            .map(|limit| self.max_requests.saturating_sub(limit.requests))
            .unwrap_or(self.max_requests))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_allows_requests_below_limit() {
        let limiter = RateLimitingByClient::new(5, 60);
        let ctx = NetworkContext::testnet();

        for _ in 0..5 {
            let result = limiter.check_limit("client1", &ctx).await;
            assert!(result.is_ok() && result.unwrap());
        }
    }

    #[tokio::test]
    async fn test_rate_limit_blocks_requests_above_limit() {
        let limiter = RateLimitingByClient::new(2, 60);
        let ctx = NetworkContext::testnet();

        for _ in 0..2 {
            let _ = limiter.check_limit("client2", &ctx).await;
        }

        let result = limiter.check_limit("client2", &ctx).await;
        assert!(result.is_ok() && !result.unwrap());
    }

    #[tokio::test]
    async fn test_different_clients_independent() {
        let limiter = RateLimitingByClient::new(1, 60);
        let ctx = NetworkContext::testnet();

        let result1 = limiter.check_limit("client3", &ctx).await;
        let result2 = limiter.check_limit("client4", &ctx).await;

        assert!(result1.is_ok() && result1.unwrap());
        assert!(result2.is_ok() && result2.unwrap());
    }

    #[tokio::test]
    async fn test_get_remaining_requests() {
        let limiter = RateLimitingByClient::new(5, 60);
        let ctx = NetworkContext::testnet();

        let _ = limiter.check_limit("client5", &ctx).await;
        let remaining = limiter.get_remaining("client5").await;

        assert!(remaining.is_ok() && remaining.unwrap() == 4);
    }

    #[tokio::test]
    async fn test_mainnet_context() {
        let limiter = RateLimitingByClient::new(3, 60);
        let ctx = NetworkContext::mainnet();

        for _ in 0..3 {
            let result = limiter.check_limit("client6", &ctx).await;
            assert!(result.is_ok() && result.unwrap());
        }
    }
}
