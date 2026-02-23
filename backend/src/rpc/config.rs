//! RPC client configuration from environment.

use std::time::Duration;

use super::circuit_breaker::CircuitBreakerConfig;

/// Load circuit breaker and retry config from environment with defaults.
pub fn circuit_breaker_config_from_env() -> CircuitBreakerConfig {
    let failure_threshold = std::env::var("RPC_CIRCUIT_BREAKER_FAILURE_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let success_threshold = std::env::var("RPC_CIRCUIT_BREAKER_SUCCESS_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let timeout_secs = std::env::var("RPC_CIRCUIT_BREAKER_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    CircuitBreakerConfig {
        failure_threshold,
        success_threshold,
        timeout_duration: Duration::from_secs(timeout_secs),
        half_open_max_calls: 3,
    }
}

/// Max retries for retry_with_backoff (from RPC_MAX_RETRIES, default 3).
pub fn max_retries_from_env() -> u32 {
    std::env::var("RPC_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3)
}

/// Initial backoff duration (from RPC_INITIAL_BACKOFF_MS, default 100).
pub fn initial_backoff_from_env() -> Duration {
    let ms = std::env::var("RPC_INITIAL_BACKOFF_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    Duration::from_millis(ms)
}

/// Max backoff duration (from RPC_MAX_BACKOFF_MS, default 5000).
pub fn max_backoff_from_env() -> Duration {
    let ms = std::env::var("RPC_MAX_BACKOFF_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5000);
    Duration::from_millis(ms)
}
