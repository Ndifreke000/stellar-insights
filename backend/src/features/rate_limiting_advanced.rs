use crate::error::AppError;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    LimitExceeded,
    #[error("Redis error: {0}")]
    RedisError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_window: u32,
    pub window_size_seconds: u64,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100,
            window_size_seconds: 60,
            burst_size: 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitingAdvanced {
    config: RateLimitConfig,
    redis_client: Arc<ConnectionManager>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
    pub remaining_requests: u32,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            success: true,
            message: "Rate limit check passed".to_string(),
            remaining_requests: 0,
        }
    }
}

impl RateLimitingAdvanced {
    pub fn new(config: RateLimitConfig, redis_client: Arc<ConnectionManager>) -> Self {
        Self { config, redis_client }
    }

    pub async fn execute(&self, client_id: &str) -> Result<Response, RateLimitError> {
        debug!("Checking rate limit for client: {}", client_id);

        let key = format!("rate_limit:{}", client_id);
        let burst_key = format!("rate_limit_burst:{}", client_id);

        let mut conn = self.redis_client.clone();

        // Check burst limit first
        let burst_count: u32 = redis::cmd("GET")
            .arg(&burst_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?
            .unwrap_or(0);

        if burst_count >= self.config.burst_size {
            error!("Burst limit exceeded for client: {}", client_id);
            return Err(RateLimitError::LimitExceeded);
        }

        // Check sliding window rate limit
        let current_count: u32 = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?
            .unwrap_or(0);

        if current_count >= self.config.requests_per_window {
            error!("Rate limit exceeded for client: {}", client_id);
            return Err(RateLimitError::LimitExceeded);
        }

        // Increment counters
        redis::cmd("INCR")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(self.config.window_size_seconds)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        redis::cmd("INCR")
            .arg(&burst_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        redis::cmd("EXPIRE")
            .arg(&burst_key)
            .arg(1)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        let remaining = self.config.requests_per_window.saturating_sub(current_count + 1);

        info!(
            "Rate limit check passed for client: {}, remaining: {}",
            client_id, remaining
        );

        Ok(Response {
            success: true,
            message: "Rate limit check passed".to_string(),
            remaining_requests: remaining,
        })
    }

    pub async fn reset_limit(&self, client_id: &str) -> Result<(), RateLimitError> {
        let key = format!("rate_limit:{}", client_id);
        let burst_key = format!("rate_limit_burst:{}", client_id);

        let mut conn = self.redis_client.clone();

        redis::cmd("DEL")
            .arg(&key)
            .arg(&burst_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::RedisError(e.to_string()))?;

        info!("Rate limit reset for client: {}", client_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_window, 100);
        assert_eq!(config.window_size_seconds, 60);
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn test_response_default() {
        let response = Response::default();
        assert!(response.success);
        assert_eq!(response.message, "Rate limit check passed");
    }
}
