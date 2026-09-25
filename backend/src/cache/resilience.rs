//! Redis cache resilience: circuit breaker, retry logic, and fallback metrics.
//!
//! @see https://github.com/Ndifreke000/stellar-insights/issues/2386

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker states for Redis operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisCircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Configuration for the Redis circuit breaker.
#[derive(Debug, Clone)]
pub struct RedisCircuitConfig {
    pub failure_threshold: u32,
    pub recovery_timeout_secs: u64,
}

impl Default for RedisCircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout_secs: 10,
        }
    }
}

/// Circuit breaker for Redis cache operations.
/// When Redis is unavailable, the circuit opens after `failure_threshold`
/// consecutive failures, allowing the system to immediately fall back to
/// in-memory cache without waiting for Redis timeouts.
pub struct RedisCircuitBreaker {
    state: Arc<RwLock<RedisCircuitState>>,
    failure_count: Arc<AtomicU64>,
    config: RedisCircuitConfig,
    opened_at: Arc<RwLock<Option<Instant>>>,
}

impl RedisCircuitBreaker {
    pub fn new(config: RedisCircuitConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(RedisCircuitState::Closed)),
            failure_count: Arc::new(AtomicU64::new(0)),
            config,
            opened_at: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if Redis operations should be attempted.
    /// Returns Ok if circuit is closed or half-open (trial allowed).
    /// Returns Err if circuit is open — caller should use in-memory fallback.
    pub async fn can_proceed(&self) -> Result<(), String> {
        let state = *self.state.read().await;
        match state {
            RedisCircuitState::Closed => Ok(()),
            RedisCircuitState::Open => {
                if let Some(opened_at) = *self.opened_at.read().await {
                    if opened_at.elapsed() >= Duration::from_secs(self.config.recovery_timeout_secs) {
                        *self.state.write().await = RedisCircuitState::HalfOpen;
                        tracing::info!("Redis circuit breaker transitioning to half-open — attempting trial request");
                        return Ok(());
                    }
                }
                Err("Redis circuit breaker is open — using in-memory fallback".to_string())
            }
            RedisCircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record a successful Redis operation.
    pub async fn record_success(&self) {
        let prev = self.failure_count.swap(0, Ordering::Relaxed);
        let state = *self.state.read().await;
        if state == RedisCircuitState::HalfOpen {
            *self.state.write().await = RedisCircuitState::Closed;
            *self.opened_at.write().await = None;
            tracing::info!("Redis circuit breaker closed — Redis is healthy again");
        }
        if prev > 0 {
            tracing::debug!("Redis circuit breaker failure count reset (was {})", prev);
        }
    }

    /// Record a failed Redis operation.
    pub async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let state = *self.state.read().await;
        match state {
            RedisCircuitState::Closed => {
                if count >= self.config.failure_threshold as u64 {
                    *self.state.write().await = RedisCircuitState::Open;
                    *self.opened_at.write().await = Some(Instant::now());
                    tracing::warn!(
                        "Redis circuit breaker opened after {} consecutive failures — falling back to in-memory cache",
                        count
                    );
                }
            }
            RedisCircuitState::HalfOpen => {
                *self.state.write().await = RedisCircuitState::Open;
                *self.opened_at.write().await = Some(Instant::now());
                tracing::warn!("Redis circuit breaker re-opened from half-open state");
            }
            RedisCircuitState::Open => {}
        }
    }

    pub async fn state(&self) -> RedisCircuitState {
        *self.state.read().await
    }

    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }
}

/// Retry configuration for transient Redis failures.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 50,
            max_delay_ms: 500,
        }
    }
}

/// Execute a Redis operation with retry logic.
/// Returns Ok(Some(result)) on success, Ok(None) if all retries exhausted
/// (caller should fall back to in-memory cache), Err on non-retryable errors.
pub async fn with_retry<F, T>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<Option<T>, String>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, String>> + Send>>,
{
    let mut last_error = String::new();
    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => return Ok(Some(result)),
            Err(e) => {
                last_error = e;
                if attempt < config.max_retries {
                    let delay = std::cmp::min(
                        config.base_delay_ms * (1 << attempt),
                        config.max_delay_ms,
                    );
                    tracing::debug!(
                        "Redis operation failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1,
                        config.max_retries + 1,
                        delay,
                        last_error
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
    tracing::warn!(
        "Redis operation failed after {} retries: {} — falling back to in-memory cache",
        config.max_retries,
        last_error
    );
    Ok(None)
}

/// Fallback metrics for tracking in-memory cache usage when Redis is unavailable.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct FallbackMetrics {
    pub redis_hits: u64,
    pub redis_misses: u64,
    pub redis_errors: u64,
    pub fallback_hits: u64,
    pub fallback_misses: u64,
    pub circuit_open_count: u64,
    pub retry_attempts: u64,
}

/// Thread-safe fallback metrics tracker.
pub struct FallbackMetricsTracker {
    metrics: Arc<RwLock<FallbackMetrics>>,
}

impl FallbackMetricsTracker {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(FallbackMetrics::default())),
        }
    }

    pub async fn record_redis_hit(&self) {
        self.metrics.write().await.redis_hits += 1;
    }

    pub async fn record_redis_miss(&self) {
        self.metrics.write().await.redis_misses += 1;
    }

    pub async fn record_redis_error(&self) {
        self.metrics.write().await.redis_errors += 1;
    }

    pub async fn record_fallback_hit(&self) {
        self.metrics.write().await.fallback_hits += 1;
    }

    pub async fn record_fallback_miss(&self) {
        self.metrics.write().await.fallback_misses += 1;
    }

    pub async fn record_circuit_open(&self) {
        self.metrics.write().await.circuit_open_count += 1;
    }

    pub async fn record_retry(&self) {
        self.metrics.write().await.retry_attempts += 1;
    }

    pub async fn get(&self) -> FallbackMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn reset(&self) {
        *self.metrics.write().await = FallbackMetrics::default();
    }
}

impl Default for FallbackMetricsTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_circuit_breaker_closed() {
        let cb = RedisCircuitBreaker::new(RedisCircuitConfig {
            failure_threshold: 3,
            recovery_timeout_secs: 10,
        });
        assert_eq!(cb.state().await, RedisCircuitState::Closed);
        assert!(cb.can_proceed().await.is_ok());
    }

    #[tokio::test]
    async fn test_redis_circuit_breaker_opens() {
        let cb = RedisCircuitBreaker::new(RedisCircuitConfig {
            failure_threshold: 3,
            recovery_timeout_secs: 60,
        });
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, RedisCircuitState::Closed);
        cb.record_failure().await;
        assert_eq!(cb.state().await, RedisCircuitState::Open);
        assert!(cb.can_proceed().await.is_err());
    }

    #[tokio::test]
    async fn test_redis_circuit_breaker_recovery() {
        let cb = RedisCircuitBreaker::new(RedisCircuitConfig {
            failure_threshold: 2,
            recovery_timeout_secs: 0,
        });
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, RedisCircuitState::Open);
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(cb.can_proceed().await.is_ok());
        assert_eq!(cb.state().await, RedisCircuitState::HalfOpen);
        cb.record_success().await;
        assert_eq!(cb.state().await, RedisCircuitState::Closed);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_on_first_attempt() {
        let config = RetryConfig::default();
        let result = with_retry(&config, || Box::pin(async { Ok(42) })).await;
        assert_eq!(result.unwrap(), Some(42));
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_after_retries() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1,
            max_delay_ms: 10,
        };
        let attempts = Arc::new(AtomicU64::new(0));
        let attempts_clone = attempts.clone();
        let result = with_retry(&config, move || {
            let attempts = attempts_clone.clone();
            Box::pin(async move {
                let n = attempts.fetch_add(1, Ordering::Relaxed);
                if n < 2 {
                    Err("transient failure".to_string())
                } else {
                    Ok(99)
                }
            })
        })
        .await;
        assert_eq!(result.unwrap(), Some(99));
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_with_retry_exhausted() {
        let config = RetryConfig {
            max_retries: 2,
            base_delay_ms: 1,
            max_delay_ms: 5,
        };
        let result = with_retry(&config, || {
            Box::pin(async { Err("persistent failure".to_string()) })
        })
        .await;
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_fallback_metrics() {
        let tracker = FallbackMetricsTracker::new();
        tracker.record_redis_hit().await;
        tracker.record_redis_hit().await;
        tracker.record_fallback_hit().await;
        tracker.record_redis_error().await;

        let metrics = tracker.get().await;
        assert_eq!(metrics.redis_hits, 2);
        assert_eq!(metrics.fallback_hits, 1);
        assert_eq!(metrics.redis_errors, 1);
    }
}
