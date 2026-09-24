use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub rejected_calls: u64,
}

pub struct CircuitBreakerPattern {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    metrics: Arc<RwLock<CircuitBreakerMetrics>>,
}

impl CircuitBreakerPattern {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(CircuitBreakerMetrics {
                total_calls: 0,
                successful_calls: 0,
                failed_calls: 0,
                rejected_calls: 0,
            })),
        }
    }

    pub async fn execute<F, T>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, String>>,
    {
        let state = *self.state.read().await;

        match state {
            CircuitState::Open => {
                let last_failure = *self.last_failure_time.read().await;
                if let Some(last_time) = last_failure {
                    if last_time.elapsed() >= self.config.timeout {
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        self.execute_call(f).await
                    } else {
                        self.record_rejection().await;
                        Err(CircuitBreakerError::CircuitOpen)
                    }
                } else {
                    self.record_rejection().await;
                    Err(CircuitBreakerError::CircuitOpen)
                }
            }
            CircuitState::HalfOpen => {
                let success_count = *self.success_count.read().await;
                if success_count >= self.config.half_open_max_calls {
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    *self.success_count.write().await = 0;
                }
                self.execute_call(f).await
            }
            CircuitState::Closed => self.execute_call(f).await,
        }
    }

    async fn execute_call<F, T>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, String>>,
    {
        self.record_call().await;

        match f.await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::ExecutionFailed(e))
            }
        }
    }

    async fn record_call(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.total_calls += 1;
    }

    async fn record_success(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.successful_calls += 1;

        let mut success_count = self.success_count.write().await;
        *success_count += 1;

        let mut failure_count = self.failure_count.write().await;
        *failure_count = 0;
    }

    async fn record_failure(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.failed_calls += 1;

        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        *self.last_failure_time.write().await = Some(Instant::now());

        if *failure_count >= self.config.failure_threshold {
            *self.state.write().await = CircuitState::Open;
        }
    }

    async fn record_rejection(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.rejected_calls += 1;
    }

    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }

    pub async fn get_metrics(&self) -> CircuitBreakerMetrics {
        self.metrics.read().await.clone()
    }

    pub async fn reset(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        *self.last_failure_time.write().await = None;
        let mut metrics = self.metrics.write().await;
        *metrics = CircuitBreakerMetrics {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            rejected_calls: 0,
        };
    }
}

#[derive(Debug)]
pub enum CircuitBreakerError {
    CircuitOpen,
    ExecutionFailed(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::ExecutionFailed(e) => write!(f, "Execution failed: {}", e),
        }
    }
}

impl std::error::Error for CircuitBreakerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreakerPattern::new(CircuitBreakerConfig::default());
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreakerPattern::new(config);

        for _ in 0..2 {
            let _ = cb
                .execute(async { Err::<(), _>("test error".to_string()) })
                .await;
        }

        assert_eq!(cb.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let cb = CircuitBreakerPattern::new(config);

        let _ = cb
            .execute(async { Err::<(), _>("test error".to_string()) })
            .await;

        assert_eq!(cb.get_state().await, CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(150)).await;

        let _ = cb.execute(async { Ok::<(), _>(()) }).await;
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_metrics() {
        let cb = CircuitBreakerPattern::new(CircuitBreakerConfig::default());

        let _ = cb.execute(async { Ok::<(), _>(()) }).await;
        let metrics = cb.get_metrics().await;

        assert_eq!(metrics.total_calls, 1);
        assert_eq!(metrics.successful_calls, 1);
    }
}
