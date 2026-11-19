//! The single circuit breaker used by every outbound integration (Stellar RPC /
//! Horizon, webhook dispatch, email, Telegram).
//!
//! It is generic over the protected call: [`CircuitBreaker::call`] wraps any
//! `Future<Output = Result<T, E>>` and [`CircuitBreaker::call_sync`] wraps any
//! blocking `FnOnce() -> Result<T, E>`. Time is read through a [`Clock`] so the
//! state machine can be tested deterministically with [`ManualClock`].
//!
//! # State-transition table
//!
//! | From      | Event                                        | To        |
//! |-----------|----------------------------------------------|-----------|
//! | Closed    | success                                      | Closed (failure count reset) |
//! | Closed    | failure, consecutive failures < threshold    | Closed    |
//! | Closed    | failure, consecutive failures == threshold   | Open      |
//! | Open      | call before `open_duration` elapsed          | Open (call rejected) |
//! | Open      | call after `open_duration` elapsed           | HalfOpen (call admitted as a probe) |
//! | HalfOpen  | more than `half_open_max_calls` in flight    | HalfOpen (call rejected) |
//! | HalfOpen  | success, successes < `success_threshold`     | HalfOpen  |
//! | HalfOpen  | success, successes == `success_threshold`    | Closed    |
//! | HalfOpen  | any failure                                  | Open (timer restarted) |
//!
//! Every transition increments
//! `circuit_breaker_transitions_total{breaker,from,to}` and updates the
//! `circuit_breaker_state{endpoint}` gauge (0=closed, 1=open, 2=half-open).

use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::rpc::metrics;

/// Source of monotonic time for the breaker.
pub trait Clock: Send + Sync + fmt::Debug {
    fn now(&self) -> Instant;
}

/// Wall clock backed by [`Instant::now`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Manually-advanced clock for deterministic tests.
#[derive(Debug)]
pub struct ManualClock {
    now: Mutex<Instant>,
}

impl ManualClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            now: Mutex::new(Instant::now()),
        }
    }

    pub fn advance(&self, by: Duration) {
        let mut now = self
            .now
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *now += by;
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Instant {
        *self
            .now
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Circuit state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::HalfOpen => "half_open",
        }
    }

    #[must_use]
    pub const fn as_gauge(self) -> i64 {
        match self {
            Self::Closed => 0,
            Self::Open => 1,
            Self::HalfOpen => 2,
        }
    }
}

/// Error returned by a protected call.
#[derive(Debug)]
pub enum CallError<E> {
    /// The breaker is open (or half-open with no free probe slot); the call
    /// was not attempted.
    Rejected,
    /// The call ran and failed.
    Inner(E),
}

impl<E: fmt::Display> fmt::Display for CallError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected => write!(f, "Circuit breaker open"),
            Self::Inner(e) => write!(f, "{e}"),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for CallError<E> {}

impl CallError<anyhow::Error> {
    /// Flatten into an `anyhow::Error`, mapping `Rejected` to "Circuit breaker open".
    #[must_use]
    pub fn into_anyhow(self) -> anyhow::Error {
        match self {
            Self::Rejected => anyhow::anyhow!("Circuit breaker open"),
            Self::Inner(e) => e,
        }
    }
}

/// Configuration for the circuit breaker.
///
/// * `failure_threshold` - consecutive failures (in Closed) before opening
/// * `success_threshold` - successful half-open probes required to close
/// * `timeout_duration` - how long the circuit stays open before probing
/// * `half_open_max_calls` - concurrent probes admitted while half-open
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
            half_open_max_calls: 1,
        }
    }
}

#[derive(Debug)]
struct Inner {
    state: CircuitState,
    consecutive_failures: u32,
    half_open_successes: u32,
    half_open_in_flight: u32,
    opened_at: Option<Instant>,
}

/// Generic, clock-injectable circuit breaker.
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    clock: Arc<dyn Clock>,
    inner: Mutex<Inner>,
}

pub type SharedCircuitBreaker = Arc<CircuitBreaker>;

impl CircuitBreaker {
    #[must_use]
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self::with_clock(name, config, Arc::new(SystemClock))
    }

    #[must_use]
    pub fn with_clock(
        name: impl Into<String>,
        mut config: CircuitBreakerConfig,
        clock: Arc<dyn Clock>,
    ) -> Self {
        config.failure_threshold = config.failure_threshold.max(1);
        config.success_threshold = config.success_threshold.max(1);
        config.half_open_max_calls = config.half_open_max_calls.max(1);
        Self {
            name: name.into(),
            config,
            clock,
            inner: Mutex::new(Inner {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                half_open_successes: 0,
                half_open_in_flight: 0,
                opened_at: None,
            }),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn transition(&self, inner: &mut Inner, to: CircuitState) {
        let from = inner.state;
        if from == to {
            return;
        }
        inner.state = to;
        match to {
            CircuitState::Open => {
                inner.opened_at = Some(self.clock.now());
                inner.half_open_successes = 0;
                inner.half_open_in_flight = 0;
            }
            CircuitState::HalfOpen => {
                inner.half_open_successes = 0;
                inner.half_open_in_flight = 0;
            }
            CircuitState::Closed => {
                inner.consecutive_failures = 0;
                inner.opened_at = None;
            }
        }
        metrics::record_circuit_breaker_transition(&self.name, from.as_str(), to.as_str());
        metrics::set_circuit_breaker_state(&self.name, to.as_gauge());
        tracing::info!(
            breaker = %self.name,
            from = from.as_str(),
            to = to.as_str(),
            "circuit breaker state transition"
        );
    }

    /// Current state, applying the Open -> HalfOpen timeout if it has elapsed.
    #[must_use]
    pub fn state(&self) -> CircuitState {
        let mut inner = self.lock();
        self.maybe_half_open(&mut inner);
        inner.state
    }

    fn maybe_half_open(&self, inner: &mut Inner) {
        if inner.state == CircuitState::Open {
            let elapsed = inner.opened_at.map_or(Duration::MAX, |t| {
                self.clock.now().saturating_duration_since(t)
            });
            if elapsed >= self.config.timeout_duration {
                self.transition(inner, CircuitState::HalfOpen);
            }
        }
    }

    /// Reserve permission for one call. Returns `false` if it must be rejected.
    #[must_use]
    pub fn try_acquire(&self) -> bool {
        let mut inner = self.lock();
        self.maybe_half_open(&mut inner);
        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => {
                if inner.half_open_in_flight < self.config.half_open_max_calls {
                    inner.half_open_in_flight += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Record a successful call (after a successful [`Self::try_acquire`]).
    pub fn on_success(&self) {
        let mut inner = self.lock();
        match inner.state {
            CircuitState::Closed => inner.consecutive_failures = 0,
            CircuitState::HalfOpen => {
                inner.half_open_in_flight = inner.half_open_in_flight.saturating_sub(1);
                inner.half_open_successes += 1;
                if inner.half_open_successes >= self.config.success_threshold {
                    self.transition(&mut inner, CircuitState::Closed);
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed call (after a successful [`Self::try_acquire`]).
    pub fn on_failure(&self) {
        let mut inner = self.lock();
        match inner.state {
            CircuitState::Closed => {
                inner.consecutive_failures += 1;
                if inner.consecutive_failures >= self.config.failure_threshold {
                    self.transition(&mut inner, CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => self.transition(&mut inner, CircuitState::Open),
            CircuitState::Open => {}
        }
    }

    /// Release a reserved call that neither succeeded nor counts as a
    /// provider failure (e.g. a response rejected only for being too large).
    pub fn on_ignored(&self) {
        let mut inner = self.lock();
        if inner.state == CircuitState::HalfOpen {
            inner.half_open_in_flight = inner.half_open_in_flight.saturating_sub(1);
        }
    }

    /// Run an async call through the breaker. Every `Err` counts as a failure.
    pub async fn call<F, T, E>(&self, fut: F) -> Result<T, CallError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        self.call_classified(fut, |_| true).await
    }

    /// Run an async call through the breaker; `is_failure` decides which
    /// errors count against the circuit.
    pub async fn call_classified<F, T, E, C>(
        &self,
        fut: F,
        is_failure: C,
    ) -> Result<T, CallError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
        C: FnOnce(&E) -> bool,
    {
        if !self.try_acquire() {
            return Err(CallError::Rejected);
        }
        match fut.await {
            Ok(v) => {
                self.on_success();
                Ok(v)
            }
            Err(e) => {
                if is_failure(&e) {
                    self.on_failure();
                } else {
                    self.on_ignored();
                }
                Err(CallError::Inner(e))
            }
        }
    }

    /// Run a blocking call through the breaker.
    pub fn call_sync<F, T, E>(&self, f: F) -> Result<T, CallError<E>>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if !self.try_acquire() {
            return Err(CallError::Rejected);
        }
        match f() {
            Ok(v) => {
                self.on_success();
                Ok(v)
            }
            Err(e) => {
                self.on_failure();
                Err(CallError::Inner(e))
            }
        }
    }
}

/// Process-wide breaker shared by all real RPC/Horizon calls.
pub fn rpc_circuit_breaker() -> SharedCircuitBreaker {
    static BREAKER: OnceLock<SharedCircuitBreaker> = OnceLock::new();
    BREAKER.get_or_init(new_rpc_circuit_breaker).clone()
}

/// A fresh, unshared breaker with the RPC configuration. Used by clients
/// that never touch the network (mock mode), so failures recorded against the
/// real endpoints cannot block them and vice versa.
pub fn new_rpc_circuit_breaker() -> SharedCircuitBreaker {
    Arc::new(CircuitBreaker::new(
        "stellar_rpc",
        crate::rpc::config::circuit_breaker_config_from_env(),
    ))
}

/// Named breaker with default config, for non-RPC integrations
/// (webhooks, email, Telegram).
#[must_use]
pub fn integration_circuit_breaker(name: &str) -> SharedCircuitBreaker {
    Arc::new(CircuitBreaker::new(name, CircuitBreakerConfig::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker(clock: Arc<ManualClock>) -> CircuitBreaker {
        CircuitBreaker::with_clock(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 2,
                timeout_duration: Duration::from_secs(10),
                half_open_max_calls: 1,
            },
            clock,
        )
    }

    fn fail(cb: &CircuitBreaker) -> Result<(), CallError<&'static str>> {
        cb.call_sync(|| Err("boom"))
    }

    fn ok(cb: &CircuitBreaker) -> Result<(), CallError<&'static str>> {
        cb.call_sync(|| Ok(()))
    }

    #[test]
    fn closed_to_open_after_threshold() {
        let clock = Arc::new(ManualClock::new());
        let cb = breaker(clock);
        assert!(matches!(fail(&cb), Err(CallError::Inner(_))));
        assert!(matches!(fail(&cb), Err(CallError::Inner(_))));
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(matches!(fail(&cb), Err(CallError::Inner(_))));
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(matches!(ok(&cb), Err(CallError::Rejected)));
    }

    #[test]
    fn success_resets_consecutive_failures() {
        let cb = breaker(Arc::new(ManualClock::new()));
        let _ = fail(&cb);
        let _ = fail(&cb);
        let _ = ok(&cb);
        let _ = fail(&cb);
        let _ = fail(&cb);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn open_half_open_closed_cycle() {
        let clock = Arc::new(ManualClock::new());
        let cb = breaker(clock.clone());
        for _ in 0..3 {
            let _ = fail(&cb);
        }
        assert_eq!(cb.state(), CircuitState::Open);

        clock.advance(Duration::from_secs(9));
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(matches!(ok(&cb), Err(CallError::Rejected)));

        clock.advance(Duration::from_secs(1));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(ok(&cb).is_ok());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(ok(&cb).is_ok());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn half_open_failure_reopens_and_restarts_timer() {
        let clock = Arc::new(ManualClock::new());
        let cb = breaker(clock.clone());
        for _ in 0..3 {
            let _ = fail(&cb);
        }
        clock.advance(Duration::from_secs(10));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(matches!(fail(&cb), Err(CallError::Inner(_))));
        assert_eq!(cb.state(), CircuitState::Open);
        clock.advance(Duration::from_secs(5));
        assert_eq!(cb.state(), CircuitState::Open);
        clock.advance(Duration::from_secs(5));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn half_open_limits_concurrent_probes() {
        let clock = Arc::new(ManualClock::new());
        let cb = breaker(clock.clone());
        for _ in 0..3 {
            let _ = fail(&cb);
        }
        clock.advance(Duration::from_secs(10));
        assert!(cb.try_acquire());
        // Second probe while the first is still in flight is rejected.
        assert!(!cb.try_acquire());
        cb.on_success();
        assert!(cb.try_acquire());
    }

    #[tokio::test]
    async fn async_call_passes_through_values() {
        let cb = breaker(Arc::new(ManualClock::new()));
        let v: Result<u32, CallError<String>> = cb.call(async { Ok(7) }).await;
        assert_eq!(v.unwrap(), 7);
        let e: Result<u32, CallError<String>> = cb.call(async { Err("x".to_string()) }).await;
        assert_eq!(e.unwrap_err().to_string(), "x");
    }

    #[tokio::test]
    async fn classified_non_failures_do_not_trip() {
        let cb = breaker(Arc::new(ManualClock::new()));
        for _ in 0..10 {
            let r: Result<(), CallError<&str>> = cb
                .call_classified(async { Err("too big") }, |_| false)
                .await;
            assert!(matches!(r, Err(CallError::Inner(_))));
        }
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn rejected_displays_open_message() {
        let e: CallError<String> = CallError::Rejected;
        assert_eq!(e.to_string(), "Circuit breaker open");
    }
}
