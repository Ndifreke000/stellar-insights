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

/// Snapshot of a client's rate limit state, used to emit IETF
/// `RateLimit-*` headers on every response (both 200 and 429).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
    /// Seconds until the current window resets (never negative).
    pub reset: u64,
}

impl RateLimitHeaders {
    /// Render the IETF `RateLimit-Limit/Remaining/Reset` header triples.
    pub fn to_header_pairs(&self) -> [(&'static str, String); 3] {
        [
            ("RateLimit-Limit", self.limit.to_string()),
            ("RateLimit-Remaining", self.remaining.to_string()),
            ("RateLimit-Reset", self.reset.to_string()),
        ]
    }
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

    /// Compute the IETF `RateLimit-*` header values for a client.
    ///
    /// Returns the configured limit, the remaining requests in the current
    /// window, and the number of seconds until the window resets. Clients
    /// that have not been seen yet report a full quota with a reset equal to
    /// the configured window length.
    pub async fn headers_for(&self, client_id: &str) -> RateLimitHeaders {
        let clients = self.clients.read().await;
        let now = Utc::now();

        match clients.get(client_id) {
            Some(limit) => {
                let remaining = self.max_requests.saturating_sub(limit.requests);
                let reset = (limit.reset_time - now).num_seconds().max(0) as u64;
                RateLimitHeaders {
                    limit: self.max_requests,
                    remaining,
                    reset,
                }
            }
            None => RateLimitHeaders {
                limit: self.max_requests,
                remaining: self.max_requests,
                reset: self.window_secs,
            },
        }
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

    #[tokio::test]
    async fn test_headers_for_unknown_client() {
        let limiter = RateLimitingByClient::new(10, 30);
        let headers = limiter.headers_for("never-seen").await;

        assert_eq!(headers.limit, 10);
        assert_eq!(headers.remaining, 10);
        assert_eq!(headers.reset, 30);
    }

    #[tokio::test]
    async fn test_headers_on_success_response() {
        let limiter = RateLimitingByClient::new(5, 60);
        let ctx = NetworkContext::testnet();

        assert!(limiter.check_limit("client7", &ctx).await.unwrap());
        let headers = limiter.headers_for("client7").await;

        assert_eq!(headers.limit, 5);
        assert_eq!(headers.remaining, 4);
        assert!(headers.reset <= 60);
    }

    #[tokio::test]
    async fn test_headers_on_rate_limited_response() {
        let limiter = RateLimitingByClient::new(2, 60);
        let ctx = NetworkContext::testnet();

        for _ in 0..2 {
            let _ = limiter.check_limit("client8", &ctx).await;
        }
        assert!(!limiter.check_limit("client8", &ctx).await.unwrap());

        let headers = limiter.headers_for("client8").await;
        assert_eq!(headers.limit, 2);
        assert_eq!(headers.remaining, 0);
        assert!(headers.reset <= 60);
    }

    #[tokio::test]
    async fn test_header_pairs_are_ietf_named() {
        let headers = RateLimitHeaders {
            limit: 100,
            remaining: 42,
            reset: 7,
        };
        let pairs = headers.to_header_pairs();

        assert_eq!(pairs[0], ("RateLimit-Limit", "100".to_string()));
        assert_eq!(pairs[1], ("RateLimit-Remaining", "42".to_string()));
        assert_eq!(pairs[2], ("RateLimit-Reset", "7".to_string()));
    }
}
