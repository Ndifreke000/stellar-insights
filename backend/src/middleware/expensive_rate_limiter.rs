//! Per-endpoint rate limiting for expensive operations.
//!
//! Applies granular rate limits to resource-intensive endpoints:
//! - CSV/PDF export: 5 requests/minute
//! - Analytics aggregation: 20 requests/minute
//! - RPC proxy: 100 requests/minute
//! - Contract event indexing: 10 requests/minute
//!
//! Returns 429 Too Many Requests with Retry-After header when exceeded.
//!
//! Closes #2387

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for a single rate-limited endpoint category.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window: Duration,
    pub retry_after_seconds: u64,
}

impl RateLimitConfig {
    pub fn export() -> Self {
        Self {
            max_requests: 5,
            window: Duration::from_secs(60),
            retry_after_seconds: 60,
        }
    }

    pub fn analytics() -> Self {
        Self {
            max_requests: 20,
            window: Duration::from_secs(60),
            retry_after_seconds: 60,
        }
    }

    pub fn rpc_proxy() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            retry_after_seconds: 60,
        }
    }

    pub fn contract_events() -> Self {
        Self {
            max_requests: 10,
            window: Duration::from_secs(60),
            retry_after_seconds: 60,
        }
    }
}

/// Tracks request counts per user/identifier within a sliding window.
#[derive(Clone, Debug)]
struct WindowEntry {
    count: u32,
    window_start: Instant,
}

/// Per-endpoint rate limiter using in-memory sliding window counters.
/// For production with multiple instances, replace with Redis-backed counters.
#[derive(Clone)]
pub struct ExpensiveRateLimiter {
    windows: Arc<RwLock<HashMap<(String, String), WindowEntry>>>,
}

impl Default for ExpensiveRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpensiveRateLimiter {
    pub fn new() -> Self {
        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request is allowed under the rate limit.
    /// Returns `Ok(())` if allowed, or `Err(retry_after_secs)` if rate limited.
    pub async fn check(
        &self,
        endpoint_category: &str,
        identifier: &str,
        config: &RateLimitConfig,
    ) -> Result<(), u64> {
        let key = (endpoint_category.to_string(), identifier.to_string());
        let now = Instant::now();

        let mut windows = self.windows.write().await;
        let entry = windows.entry(key.clone()).or_insert(WindowEntry {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(entry.window_start) >= config.window {
            entry.count = 0;
            entry.window_start = now;
        }

        if entry.count >= config.max_requests {
            return Err(config.retry_after_seconds);
        }

        entry.count += 1;
        Ok(())
    }

    /// Clean up expired windows to prevent memory growth.
    /// Call periodically (e.g., every 5 minutes).
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut windows = self.windows.write().await;
        windows.retain(|_, entry| {
            now.duration_since(entry.window_start) < Duration::from_secs(300)
        });
    }

    /// Get current usage stats for an identifier on a given endpoint.
    pub async fn get_usage(
        &self,
        endpoint_category: &str,
        identifier: &str,
    ) -> Option<(u32, u32)> {
        let key = (endpoint_category.to_string(), identifier.to_string());
        let windows = self.windows.read().await;
        windows.get(&key).map(|entry| (entry.count, entry.window_start.elapsed().as_secs()))
    }
}

/// Categorize a request path into an endpoint category for rate limiting.
/// Returns `Some((category, config))` if the path should be rate-limited,
/// or `None` if it's not an expensive endpoint.
pub fn categorize_endpoint(path: &str) -> Option<(&'static str, RateLimitConfig)> {
    if path.starts_with("/api/export/") {
        Some(("export", RateLimitConfig::export()))
    } else if path.starts_with("/api/analytics/") || path.starts_with("/api/dashboard/") {
        Some(("analytics", RateLimitConfig::analytics()))
    } else if path.starts_with("/api/rpc/") {
        Some(("rpc_proxy", RateLimitConfig::rpc_proxy()))
    } else if path.starts_with("/api/contract-events/") {
        Some(("contract_events", RateLimitConfig::contract_events()))
    } else {
        None
    }
}

/// Extract a user identifier from request headers for per-user rate limiting.
/// Falls back to client IP if no user ID is available.
pub fn extract_identifier(
    user_id: Option<&str>,
    api_key: Option<&str>,
    client_ip: Option<&str>,
) -> String {
    if let Some(uid) = user_id {
        format!("user:{}", uid)
    } else if let Some(key) = api_key {
        format!("key:{}", key)
    } else if let Some(ip) = client_ip {
        format!("ip:{}", ip)
    } else {
        "anonymous".to_string()
    }
}

/// HTTP headers to include in a 429 Too Many Requests response.
pub fn rate_limit_headers(config: &RateLimitConfig, remaining: u32) -> Vec<(String, String)> {
    vec![
        ("X-RateLimit-Limit".to_string(), config.max_requests.to_string()),
        ("X-RateLimit-Remaining".to_string(), remaining.to_string()),
        ("Retry-After".to_string(), config.retry_after_seconds.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allows_under_limit() {
        let limiter = ExpensiveRateLimiter::new();
        let config = RateLimitConfig::export();

        for _ in 0..5 {
            assert!(limiter.check("export", "user:1", &config).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_blocks_over_limit() {
        let limiter = ExpensiveRateLimiter::new();
        let config = RateLimitConfig::export();

        for _ in 0..5 {
            limiter.check("export", "user:2", &config).await.unwrap();
        }
        let result = limiter.check("export", "user:2", &config).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 60);
    }

    #[tokio::test]
    async fn test_separate_limits_per_user() {
        let limiter = ExpensiveRateLimiter::new();
        let config = RateLimitConfig::analytics();

        // User 1 uses all their quota
        for _ in 0..20 {
            limiter.check("analytics", "user:1", &config).await.unwrap();
        }
        // User 2 should still be allowed
        assert!(limiter.check("analytics", "user:2", &config).await.is_ok());
    }

    #[tokio::test]
    async fn test_separate_limits_per_category() {
        let limiter = ExpensiveRateLimiter::new();

        // Exhaust export limit
        let export_config = RateLimitConfig::export();
        for _ in 0..5 {
            limiter.check("export", "user:3", &export_config).await.unwrap();
        }
        assert!(limiter.check("export", "user:3", &export_config).await.is_err());

        // Analytics should still work for same user
        let analytics_config = RateLimitConfig::analytics();
        assert!(limiter.check("analytics", "user:3", &analytics_config).await.is_ok());
    }

    #[test]
    fn test_categorize_export() {
        let result = categorize_endpoint("/api/export/csv");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "export");
    }

    #[test]
    fn test_categorize_analytics() {
        let result = categorize_endpoint("/api/analytics/earnings");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "analytics");
    }

    #[test]
    fn test_categorize_unlimited() {
        let result = categorize_endpoint("/api/transactions");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_identifier_priority() {
        assert_eq!(extract_identifier(Some("u1"), Some("k1"), Some("ip1")), "user:u1");
        assert_eq!(extract_identifier(None, Some("k1"), Some("ip1")), "key:k1");
        assert_eq!(extract_identifier(None, None, Some("ip1")), "ip:ip1");
        assert_eq!(extract_identifier(None, None, None), "anonymous");
    }
}
