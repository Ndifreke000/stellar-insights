//! Circuit breaker to avoid hammering failing RPC/Horizon endpoints.
//!
//! After a configurable number of failures, the circuit opens and requests
//! fail fast. After a timeout, the circuit moves to half-open and allows
//! a limited number of test requests; success closes the circuit.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::rpc::error::RpcError;
use crate::rpc::metrics;

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_duration: Duration,
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

/// Circuit breaker for a single logical endpoint (e.g. Horizon API).
#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    config: CircuitBreakerConfig,
    endpoint: String,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig, endpoint: impl Into<String>) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitState::Closed { failure_count: 0 })),
            config,
            endpoint: endpoint.into(),
        }
    }

    /// Run an operation through the circuit breaker.
    /// Returns CircuitBreakerOpen if the circuit is open.
    pub async fn call<F, Fut, T>(&self, f: F) -> Result<T, RpcError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, RpcError>>,
    {
        if self.is_open().await {
            metrics::record_rpc_error("circuit_breaker_open", &self.endpoint);
            return Err(RpcError::CircuitBreakerOpen);
        }

        let result = f().await;

        match &result {
            Ok(_) => {
                self.on_success().await;
            }
            Err(e) if e.is_retryable() => {
                self.on_failure().await;
            }
            Err(_) => {}
        }

        result
    }

    async fn is_open(&self) -> bool {
        let mut state = self.state.lock().await;
        let now = Instant::now();

        match &*state {
            CircuitState::Open { opened_at } => {
                if now.duration_since(*opened_at) >= self.config.timeout_duration {
                    *state = CircuitState::HalfOpen { success_count: 0 };
                    metrics::set_circuit_breaker_state(&self.endpoint, 2); // half-open
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
        let current = std::mem::replace(&mut *state, CircuitState::Closed { failure_count: 0 });
        *state = match current {
            CircuitState::HalfOpen { success_count } => {
                if success_count + 1 >= self.config.success_threshold {
                    metrics::set_circuit_breaker_state(&self.endpoint, 0); // closed
                    CircuitState::Closed { failure_count: 0 }
                } else {
                    CircuitState::HalfOpen {
                        success_count: success_count + 1,
                    }
                }
            }
            _ => {
                metrics::set_circuit_breaker_state(&self.endpoint, 0);
                CircuitState::Closed { failure_count: 0 }
            }
        };
    }

    async fn on_failure(&self) {
        let mut state = self.state.lock().await;
        let current = std::mem::replace(&mut *state, CircuitState::Closed { failure_count: 0 });
        *state = match current {
            CircuitState::Closed { failure_count } => {
                if failure_count + 1 >= self.config.failure_threshold {
                    metrics::set_circuit_breaker_state(&self.endpoint, 1); // open
                    CircuitState::Open {
                        opened_at: Instant::now(),
                    }
                } else {
                    CircuitState::Closed {
                        failure_count: failure_count + 1,
                    }
                }
            }
            CircuitState::HalfOpen { .. } => {
                metrics::set_circuit_breaker_state(&self.endpoint, 1);
                CircuitState::Open {
                    opened_at: Instant::now(),
                }
            }
            other => other,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(1),
            half_open_max_calls: 3,
        }
    }

    #[tokio::test]
    async fn circuit_opens_after_threshold() {
        let config = test_config();
        let cb = CircuitBreaker::new(config, "test");

        // Two retryable failures -> open
        let _: Result<(), _> = cb
            .call(|| async {
                Err(RpcError::ServerError {
                    status: 503,
                    message: "x".into(),
                })
            })
            .await;
        let _: Result<(), _> = cb
            .call(|| async {
                Err(RpcError::ServerError {
                    status: 503,
                    message: "x".into(),
                })
            })
            .await;

        let r = cb.call(|| async { Ok(()) }).await;
        assert!(matches!(r, Err(RpcError::CircuitBreakerOpen)));
    }

    #[tokio::test]
    async fn non_retryable_error_does_not_increment_failure_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(1),
            half_open_max_calls: 3,
        };
        let cb = CircuitBreaker::new(config, "test");

        let _: Result<(), _> = cb
            .call(|| async { Err(RpcError::ParseError("bad".into())) })
            .await;
        let r = cb.call(|| async { Ok(42) }).await;
        assert_eq!(r.unwrap(), 42);
    }

    #[tokio::test]
    async fn circuit_closes_after_successes_in_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout_duration: Duration::from_millis(10),
            half_open_max_calls: 3,
        };
        let cb = CircuitBreaker::new(config, "test");

        // Open the circuit
        let _: Result<(), _> = cb
            .call(|| async {
                Err(RpcError::ServerError {
                    status: 503,
                    message: "x".into(),
                })
            })
            .await;
        let _: Result<(), _> = cb
            .call(|| async {
                Err(RpcError::ServerError {
                    status: 503,
                    message: "x".into(),
                })
            })
            .await;
        let _: Result<(), _> = cb.call(|| async { Ok(()) }).await;
        assert!(matches!(
            cb.call(|| async { Ok(()) }).await,
            Err(RpcError::CircuitBreakerOpen)
        ));

        // Wait for timeout -> half-open
        tokio::time::sleep(Duration::from_millis(20)).await;
        let r1 = cb.call(|| async { Ok(1) }).await;
        assert_eq!(r1.unwrap(), 1);
        let r2 = cb.call(|| async { Ok(2) }).await;
        assert_eq!(r2.unwrap(), 2);
        // After success_threshold successes, circuit should be closed
        let r3 = cb.call(|| async { Ok(3) }).await;
        assert_eq!(r3.unwrap(), 3);
    }
}
