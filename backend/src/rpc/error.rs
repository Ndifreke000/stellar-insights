use std::fmt;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum RpcError {
    NetworkError(String),
    RateLimitError { retry_after: Option<Duration> },
    ServerError { status: u16, message: String },
    ParseError(String),
    TimeoutError(String),
    CircuitBreakerOpen,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {msg}"),
            Self::RateLimitError { retry_after } => {
                write!(f, "Rate limit error")?;
                if let Some(delay) = retry_after {
                    write!(f, " (retry after {}s)", delay.as_secs())?;
                }
                Ok(())
            }
            Self::ServerError { status, message } => {
                write!(f, "Server error ({status}): {message}")
            }
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::TimeoutError(msg) => write!(f, "Timeout error: {msg}"),
            Self::CircuitBreakerOpen => write!(f, "Circuit breaker is open"),
        }
    }
}

impl std::error::Error for RpcError {}

impl RpcError {
    #[must_use] 
    pub const fn is_retryable(&self) -> bool {
        self.is_transient()
            || matches!(self, Self::ServerError { status, .. } if *status >= 500)
    }

    #[must_use] 
    pub const fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::NetworkError(_) | Self::TimeoutError(_) | Self::RateLimitError { .. }
        )
    }

    #[must_use] 
    pub fn categorize(err: &str) -> Self {
        let lowered = err.to_ascii_lowercase();
        if lowered.contains("timeout") || lowered.contains("timed out") {
            Self::TimeoutError(err.to_string())
        } else if lowered.contains("rate limit") || lowered.contains("429") {
            Self::RateLimitError { retry_after: None }
        } else if lowered.contains("parse") || lowered.contains("deserialize") {
            Self::ParseError(err.to_string())
        } else if lowered.contains("network")
            || lowered.contains("connection")
            || lowered.contains("dns")
        {
            Self::NetworkError(err.to_string())
        } else {
            Self::ServerError {
                status: 500,
                message: err.to_string(),
            }
        }
    }

    #[must_use] 
    pub const fn error_type_label(&self) -> &'static str {
        match self {
            Self::NetworkError(_) => "network_error",
            Self::RateLimitError { .. } => "rate_limit_error",
            Self::ServerError { .. } => "server_error",
            Self::ParseError(_) => "parse_error",
            Self::TimeoutError(_) => "timeout_error",
            Self::CircuitBreakerOpen => "circuit_breaker_open",
        }
    }
}

use crate::rpc::circuit_breaker::CircuitBreaker;

#[derive(Debug, Clone)]
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
    circuit_breaker: Arc<CircuitBreaker>,
) -> Result<T, RpcError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, RpcError>>,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        let result = circuit_breaker.call(&operation).await;

        match result {
            Ok(val) => return Ok(val),
            Err(e) => {
                if !e.is_transient() || attempt >= config.max_attempts {
                    return Err(e);
                }

                let delay = std::cmp::min(
                    config
                        .base_delay_ms
                        .saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1))),
                    config.max_delay_ms,
                );

                tokio::time::sleep(Duration::from_millis(delay)).await;
            }
        }
    }
}
