//! Circuit breaker to avoid hammering failing RPC/Horizon endpoints.
//! Uses the failsafe crate for battle-tested reliability.

use failsafe::{backoff, failure_policy, Config, StateMachine};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Concrete circuit breaker type using a fixed backoff and consecutive-failure policy.
pub type CircuitBreaker =
    StateMachine<failure_policy::ConsecutiveFailures<std::iter::Repeat<Duration>>, ()>;
pub type SharedCircuitBreaker = Arc<CircuitBreaker>;

pub fn rpc_circuit_breaker() -> SharedCircuitBreaker {
    static BREAKER: OnceLock<SharedCircuitBreaker> = OnceLock::new();
    BREAKER
        .get_or_init(|| {
            let config = CircuitBreakerConfig::default();
            let backoff = backoff::constant(config.timeout_duration);
            let policy = failure_policy::consecutive_failures(config.failure_threshold, backoff);
            let cb: CircuitBreaker = Config::new().failure_policy(policy).build();
            Arc::new(cb)
        })
        .clone()
}

/// Configuration for the circuit breaker.
///
/// Controls when the circuit opens (stops forwarding requests) and when it
/// attempts recovery via the half-open state.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Consecutive retryable failures required to trip the circuit open.
    pub failure_threshold: u32,
    /// Retained for config compatibility; not supported by failsafe 1.3.
    pub success_threshold: u32,
    /// How long the circuit stays open before attempting recovery.
    pub timeout_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(30),
        }
    }
}
