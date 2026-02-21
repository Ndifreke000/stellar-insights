//! Circuit breaker to avoid hammering failing RPC endpoints.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::rpc::error::RpcError;
use crate::rpc::metrics;

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit.
    pub failure_threshold: u32,
    /// Number of successes in half-open required to close the circuit.
    pub success_threshold: u32,
    /// Time to wait before trying half-open.
    pub timeout_duration: Duration,
    /// Max test calls in half-open before deciding.
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }
}

#[derive(Debug, Clone)]
enum CircuitState {
    Closed { failure_count: u32 },
    Open { opened_at: Instant },
    HalfOpen { success_count: u32 },
}

/// Circuit breaker for RPC calls. Tracks failures and opens after threshold to avoid
/// cascading failures; allows probing (half-open) after a timeout.
#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    config: CircuitBreakerConfig,
    endpoint_name: String,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig, endpoint_name: String) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed { failure_count: 0 })),
            config,
            endpoint_name,
        }
    }

    /// Execute a call through the circuit breaker. Returns CircuitBreakerOpen if open.
    pub async fn call<F, T>(&self, f: F) -> Result<T, RpcError>
    where
        F: std::future::Future<Output = Result<T, RpcError>>,
    {
        if self.is_open().await {
            return Err(RpcError::CircuitBreakerOpen);
        }

        match f.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                if e.is_retryable() {
                    self.on_failure().await;
                }
                Err(e)
            }
        }
    }

    async fn is_open(&self) -> bool {
        let mut state = self.state.lock().await;
        match *state {
            CircuitState::Open { opened_at } => {
                if opened_at.elapsed() >= self.config.timeout_duration {
                    *state = CircuitState::HalfOpen { success_count: 0 };
                    false
                } else {
                    true
                }
            }
            _ => false,
        }
    }

    async fn on_success(&self) {
        let mut state = self.state.lock().await;
        *state = match *state {
            CircuitState::HalfOpen { success_count } => {
                if success_count + 1 >= self.config.success_threshold {
                    CircuitState::Closed { failure_count: 0 }
                } else {
                    CircuitState::HalfOpen {
                        success_count: success_count + 1,
                    }
                }
            }
            _ => CircuitState::Closed { failure_count: 0 },
        };
        metrics::set_circuit_breaker_state(self.endpoint_name(), self.state_value_locked(&state));
    }

    async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        *state = match *state {
            CircuitState::Closed { failure_count } => {
                if failure_count + 1 >= self.config.failure_threshold {
                    CircuitState::Open {
                        opened_at: Instant::now(),
                    }
                } else {
                    CircuitState::Closed {
                        failure_count: failure_count + 1,
                    }
                }
            }
            CircuitState::HalfOpen { .. } => CircuitState::Open {
                opened_at: Instant::now(),
            },
            CircuitState::Open { opened_at } => CircuitState::Open { opened_at },
        };
        metrics::set_circuit_breaker_state(self.endpoint_name(), self.state_value_locked(&state));
    }

    fn state_value_locked(&self, state: &CircuitState) -> i64 {
        match state {
            CircuitState::Closed { .. } => 0,
            CircuitState::Open { .. } => 1,
            CircuitState::HalfOpen { .. } => 2,
        }
    }

    /// Current state for metrics: 0 = closed, 1 = open, 2 = half-open.
    pub async fn state_value(&self) -> i64 {
        let state = self.state.lock().await;
        self.state_value_locked(&state)
    }

    pub fn endpoint_name(&self) -> &str {
        &self.endpoint_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout_duration: Duration::from_secs(1),
            half_open_max_calls: 1,
        };
        let cb = CircuitBreaker::new(config, "test".to_string());

        // First failure
        let _ = cb
            .call(async { Err::<(), _>(RpcError::TimeoutError(Duration::from_secs(1))) })
            .await;
        assert_eq!(cb.state_value().await, 0); // still closed

        // Second failure -> open
        let _ = cb
            .call(async { Err::<(), _>(RpcError::TimeoutError(Duration::from_secs(1))) })
            .await;
        assert_eq!(cb.state_value().await, 1); // open

        // Next call fails fast with CircuitBreakerOpen
        let res = cb.call(async { Ok::<(), RpcError>(()) }).await;
        assert!(matches!(res, Err(RpcError::CircuitBreakerOpen)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_resets() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 1,
            timeout_duration: Duration::from_secs(30),
            half_open_max_calls: 1,
        };
        let cb = CircuitBreaker::new(config, "test".to_string());

        let _ = cb.call(async { Ok::<i32, RpcError>(42) }).await;
        assert_eq!(cb.state_value().await, 0);
    }
}
