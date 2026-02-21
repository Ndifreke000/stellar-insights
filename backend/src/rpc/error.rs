use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum RpcError {
    NetworkError(String),
    RateLimitError(String),
    ServerError(String),
    ParseError(String),
    TimeoutError(String),
    CircuitBreakerOpen,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            RpcError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
            RpcError::ServerError(msg) => write!(f, "Server error: {}", msg),
            RpcError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RpcError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            RpcError::CircuitBreakerOpen => write!(f, "Circuit breaker is open"),
        }
    }
}

impl RpcError {
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            RpcError::NetworkError(_) | RpcError::TimeoutError(_) | RpcError::RateLimitError(_)
        )
    }

    pub fn categorize(err: &str) -> Self {
        let lowered = err.to_ascii_lowercase();
        if lowered.contains("timeout") || lowered.contains("timed out") {
            RpcError::TimeoutError(err.to_string())
        } else if lowered.contains("rate limit") || lowered.contains("429") {
            RpcError::RateLimitError(err.to_string())
        } else if lowered.contains("parse") || lowered.contains("deserialize") {
            RpcError::ParseError(err.to_string())
        } else if lowered.contains("network")
            || lowered.contains("connection")
            || lowered.contains("dns")
        {
            RpcError::NetworkError(err.to_string())
        } else {
            RpcError::ServerError(err.to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    half_open_calls: u32,
    last_failure_time: Option<Instant>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            half_open_calls: 0,
            last_failure_time: None,
            config,
        }
    }

    pub fn can_attempt(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(t) = self.last_failure_time {
                    if t.elapsed() >= self.config.timeout_duration {
                        self.state = CircuitState::HalfOpen;
                        self.half_open_calls = 0;
                        self.success_count = 0;
                        tracing::info!("Circuit breaker transitioning to HalfOpen");
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                if self.half_open_calls < self.config.half_open_max_calls {
                    self.half_open_calls += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    tracing::info!("Circuit breaker closing after successful recovery");
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.half_open_calls = 0;
                }
            }
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.config.failure_threshold {
                    tracing::warn!(
                        "Circuit breaker opening after {} failures",
                        self.failure_count
                    );
                    self.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!("Circuit breaker re-opening after HalfOpen failure");
                self.state = CircuitState::Open;
                self.success_count = 0;
                self.half_open_calls = 0;
            }
            CircuitState::Open => {}
        }
    }

    pub fn state(&self) -> &CircuitState {
        &self.state
    }
}

pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5_000,
        }
    }
}

pub async fn with_retry<F, Fut, T>(
    operation: F,
    config: RetryConfig,
    circuit_breaker: Arc<Mutex<CircuitBreaker>>,
) -> Result<T, RpcError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, RpcError>>,
{
    let mut attempt = 0;

    loop {
        {
            let mut cb = circuit_breaker
                .lock()
                .expect("circuit breaker lock poisoned");
            if !cb.can_attempt() {
                tracing::error!("Circuit breaker is open, rejecting request");
                return Err(RpcError::CircuitBreakerOpen);
            }
        }

        attempt += 1;
        match operation().await {
            Ok(result) => {
                circuit_breaker
                    .lock()
                    .expect("circuit breaker lock poisoned")
                    .record_success();
                return Ok(result);
            }
            Err(e) => {
                circuit_breaker
                    .lock()
                    .expect("circuit breaker lock poisoned")
                    .record_failure();

                tracing::error!(attempt = attempt, error = %e, "RPC call failed");

                if !e.is_transient() || attempt >= config.max_attempts {
                    tracing::error!("Permanent error or max retries reached: {}", e);
                    return Err(e);
                }

                let delay = std::cmp::min(
                    config
                        .base_delay_ms
                        .saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1))),
                    config.max_delay_ms,
                );
                tracing::warn!(
                    "Retrying in {}ms (attempt {}/{})",
                    delay,
                    attempt,
                    config.max_attempts
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }
    }
}
