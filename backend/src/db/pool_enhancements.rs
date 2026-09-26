//! Database connection pool enhancements: leak detection, circuit breaker,
//! and separate background pool.
//!
//! @see https://github.com/Ndifreke000/stellar-insights/issues/2385

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker states for database operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Configuration for the database circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout_secs: u64,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout_secs: 30,
            half_open_max_calls: 3,
        }
    }
}

/// Circuit breaker for database operations.
pub struct DatabaseCircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU64>,
    config: CircuitBreakerConfig,
    opened_at: Arc<RwLock<Option<Instant>>>,
}

impl DatabaseCircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU64::new(0)),
            config,
            opened_at: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn can_proceed(&self) -> Result<(), String> {
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                if let Some(opened_at) = *self.opened_at.read().await {
                    if opened_at.elapsed() >= Duration::from_secs(self.config.recovery_timeout_secs) {
                        *self.state.write().await = CircuitState::HalfOpen;
                        tracing::info!("Database circuit breaker transitioning to half-open");
                        return Ok(());
                    }
                }
                Err("Database circuit breaker is open".to_string())
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    pub async fn record_success(&self) {
        let prev = self.failure_count.swap(0, Ordering::Relaxed);
        let state = *self.state.read().await;
        if state == CircuitState::HalfOpen {
            *self.state.write().await = CircuitState::Closed;
            *self.opened_at.write().await = None;
            tracing::info!("Database circuit breaker closed after successful half-open call");
        }
        if prev > 0 {
            tracing::debug!("Database circuit breaker failure count reset (was {})", prev);
        }
    }

    pub async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        let state = *self.state.read().await;
        match state {
            CircuitState::Closed => {
                if count >= self.config.failure_threshold as u64 {
                    *self.state.write().await = CircuitState::Open;
                    *self.opened_at.write().await = Some(Instant::now());
                    tracing::warn!("Database circuit breaker opened after {} consecutive failures", count);
                }
            }
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open;
                *self.opened_at.write().await = Some(Instant::now());
                tracing::warn!("Database circuit breaker re-opened from half-open state");
            }
            CircuitState::Open => {}
        }
    }

    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    pub fn failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }
}

/// Connection leak detector.
pub struct ConnectionLeakDetector {
    acquired_at: Arc<RwLock<std::collections::HashMap<String, Instant>>>,
    leak_threshold_secs: u64,
}

impl ConnectionLeakDetector {
    pub fn new(leak_threshold_secs: u64) -> Self {
        Self {
            acquired_at: Arc::new(RwLock::new(std::collections::HashMap::new())),
            leak_threshold_secs,
        }
    }

    pub async fn track_acquire(&self, operation: &str) -> String {
        let id = format!("conn_{}_{}", operation, Instant::now().elapsed().as_nanos());
        self.acquired_at.write().await.insert(id.clone(), Instant::now());
        tracing::trace!("Connection acquired: {} for operation: {}", id, operation);
        id
    }

    pub async fn track_release(&self, id: &str) {
        if let Some(acquired_at) = self.acquired_at.write().await.remove(id) {
            let held = acquired_at.elapsed();
            if held.as_secs() > self.leak_threshold_secs {
                tracing::warn!(
                    "Potential connection leak: {} held for {:?} (threshold: {}s)",
                    id, held, self.leak_threshold_secs
                );
            }
        }
    }

    pub async fn check_leaks(&self) -> Vec<String> {
        let acquired = self.acquired_at.read().await;
        let now = Instant::now();
        let threshold = Duration::from_secs(self.leak_threshold_secs);
        acquired.iter()
            .filter(|(_, &acquired_at)| now.duration_since(acquired_at) > threshold)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub async fn tracked_count(&self) -> usize {
        self.acquired_at.read().await.len()
    }
}

/// Enhanced pool metrics for monitoring.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct EnhancedPoolMetrics {
    pub size: u32,
    pub idle: usize,
    pub active: u32,
    pub max_connections: u32,
    pub utilization_percent: f64,
    pub circuit_breaker_failures: u64,
    pub tracked_connections: usize,
}

/// Configuration for a separate background job pool.
#[derive(Debug, Clone)]
pub struct BackgroundPoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
}

impl Default for BackgroundPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            connect_timeout_seconds: 15,
        }
    }
}

impl BackgroundPoolConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            max_connections: std::env::var("DB_BG_POOL_MAX_CONNECTIONS")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(10),
            min_connections: std::env::var("DB_BG_POOL_MIN_CONNECTIONS")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(2),
            connect_timeout_seconds: std::env::var("DB_BG_POOL_CONNECT_TIMEOUT_SECONDS")
                .ok().and_then(|s| s.parse().ok()).unwrap_or(15),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = DatabaseCircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3, recovery_timeout_secs: 1, half_open_max_calls: 1,
        });
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.can_proceed().await.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let cb = DatabaseCircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3, recovery_timeout_secs: 60, half_open_max_calls: 1,
        });
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(cb.can_proceed().await.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let cb = DatabaseCircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2, recovery_timeout_secs: 0, half_open_max_calls: 1,
        });
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(cb.can_proceed().await.is_ok());
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_leak_detector() {
        let detector = ConnectionLeakDetector::new(5);
        let id = detector.track_acquire("test").await;
        assert_eq!(detector.tracked_count().await, 1);
        detector.track_release(&id).await;
        assert_eq!(detector.tracked_count().await, 0);
    }

    #[tokio::test]
    async fn test_leak_detector_detects_long_held() {
        let detector = ConnectionLeakDetector::new(0);
        let id = detector.track_acquire("slow").await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        let leaks = detector.check_leaks().await;
        assert!(leaks.contains(&id));
    }

    #[test]
    fn test_background_pool_config() {
        let config = BackgroundPoolConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
    }
}
