//! Custom error types for RPC failures with categorization for retry and circuit breaker logic.

use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Rate limit exceeded. Retry after {retry_after:?}")]
    RateLimitError { retry_after: Option<Duration> },

    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Request timeout after {0:?}")]
    TimeoutError(Duration),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("RPC error: {code} - {message}")]
    JsonRpcError { code: i32, message: String },
}

impl RpcError {
    /// Returns true if the error is transient and retrying may succeed.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RpcError::NetworkError(_)
                | RpcError::TimeoutError(_)
                | RpcError::ServerError {
                    status: 500..=599,
                    ..
                }
        )
    }

    /// Returns suggested wait duration before retry (e.g. from Retry-After header).
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            RpcError::RateLimitError { retry_after } => *retry_after,
            _ => None,
        }
    }

    /// Convert to a user-facing message suitable for API responses.
    pub fn to_user_message(&self) -> String {
        match self {
            RpcError::CircuitBreakerOpen => {
                "Service temporarily unavailable. Please try again shortly.".to_string()
            }
            RpcError::RateLimitError { .. } => {
                "Too many requests. Please try again later.".to_string()
            }
            RpcError::TimeoutError(_) => "Request timed out. Please try again.".to_string(),
            _ => "Unable to load data from the network. Please try again later.".to_string(),
        }
    }

    /// Error type label for metrics and structured logging.
    pub fn error_type(&self) -> &'static str {
        match self {
            RpcError::NetworkError(_) => "network",
            RpcError::RateLimitError { .. } => "rate_limit",
            RpcError::ServerError { .. } => "server",
            RpcError::ParseError(_) => "parse",
            RpcError::TimeoutError(_) => "timeout",
            RpcError::CircuitBreakerOpen => "circuit_breaker_open",
            RpcError::JsonRpcError { .. } => "json_rpc",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_errors() {
        // TimeoutError and 5xx ServerError are retryable
        assert!(RpcError::TimeoutError(Duration::from_secs(5)).is_retryable());
        assert!(RpcError::ServerError {
            status: 503,
            message: "unavailable".to_string()
        }
        .is_retryable());
    }

    #[test]
    fn test_non_retryable_errors() {
        assert!(!RpcError::ParseError("bad json".to_string()).is_retryable());
        assert!(!RpcError::CircuitBreakerOpen.is_retryable());
        assert!(!RpcError::ServerError {
            status: 400,
            message: "bad request".to_string()
        }
        .is_retryable());
    }

    #[test]
    fn test_retry_after_rate_limit() {
        let e = RpcError::RateLimitError {
            retry_after: Some(Duration::from_secs(60)),
        };
        assert_eq!(e.retry_after(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_error_type_labels() {
        assert_eq!(
            RpcError::CircuitBreakerOpen.error_type(),
            "circuit_breaker_open"
        );
        assert_eq!(
            RpcError::ParseError("x".to_string()).error_type(),
            "parse"
        );
    }

    #[test]
    fn test_to_user_message() {
        assert!(!RpcError::CircuitBreakerOpen
            .to_user_message()
            .is_empty());
    }
}
