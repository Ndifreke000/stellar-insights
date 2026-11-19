//! Adaptive (AIMD) batch sizing for ledger ingestion (#2767).
//!
//! The scheduler passes `run_ingestion` a *requested* batch size. Rather than
//! fetching that many ledgers forever, the ingestion service keeps an
//! [`AimdController`] that decides how many ledgers to ask `getLedgers` for
//! on each round:
//!
//! * **Additive increase** — while we are behind the network head by more
//!   than `lag_threshold` ledgers *and* the p95 latency of recent fetches is
//!   under `latency_budget`, the batch grows by `additive_step`.
//! * **Multiplicative decrease** — on a 429, 5xx or timeout the batch is
//!   halved (never below `min_batch`).
//! * In steady state (lag at or below the threshold) the batch decays back
//!   toward the requested base size so we don't burn RPC quota on large,
//!   mostly-empty requests.
//!
//! The batch is always clamped to `[min_batch, max_batch]`; `max_batch` is the
//! hard ceiling and is never exceeded regardless of what the caller requests.

use std::collections::VecDeque;
use std::time::Duration;

use crate::rpc::error::RpcError;

/// Number of latency samples kept for the p95 estimate.
const LATENCY_WINDOW: usize = 20;

/// Tunables for [`AimdController`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimdConfig {
    /// Smallest batch we will ever request.
    pub min_batch: u32,
    /// Hard ceiling. `getLedgers` caps a page at 200 ledgers on public RPC.
    pub max_batch: u32,
    /// Ledgers added per successful round while catching up.
    pub additive_step: u32,
    /// Lag (in ledgers) above which we consider ourselves "catching up".
    pub lag_threshold: u64,
    /// p95 fetch latency above which we stop growing the batch.
    pub latency_budget: Duration,
}

impl Default for AimdConfig {
    fn default() -> Self {
        Self {
            min_batch: 1,
            max_batch: 200,
            additive_step: 10,
            lag_threshold: 50,
            latency_budget: Duration::from_millis(2_000),
        }
    }
}

/// Outcome of one fetch round, fed back into the controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchOutcome {
    /// The fetch succeeded in `latency`; we are now `lag` ledgers behind head.
    Success { latency: Duration, lag: u64 },
    /// The fetch was throttled (429), failed with a 5xx or timed out.
    Congested,
    /// Any other failure (parse error, etc.) — does not change the batch.
    OtherError,
}

impl FetchOutcome {
    /// Classify an RPC error as congestion (halve) or not.
    #[must_use]
    pub const fn from_rpc_error(err: &RpcError) -> Self {
        match err {
            RpcError::RateLimitError { .. }
            | RpcError::TimeoutError(_)
            | RpcError::CircuitBreakerOpen => Self::Congested,
            RpcError::ServerError { status, .. } if *status >= 500 => Self::Congested,
            _ => Self::OtherError,
        }
    }
}

/// Additive-increase / multiplicative-decrease batch size controller.
#[derive(Debug, Clone)]
pub struct AimdController {
    config: AimdConfig,
    /// Batch size the caller asked for; steady state decays back to it.
    base: u32,
    current: u32,
    latencies: VecDeque<Duration>,
}

impl AimdController {
    #[must_use]
    pub fn new(config: AimdConfig, requested: u32) -> Self {
        let mut c = Self {
            config,
            base: 0,
            current: 0,
            latencies: VecDeque::with_capacity(LATENCY_WINDOW),
        };
        c.set_requested(requested);
        c.current = c.base;
        c
    }

    /// Update the base (requested) batch size, e.g. when the scheduler passes
    /// a new `batch_size`. The current batch is re-clamped.
    pub fn set_requested(&mut self, requested: u32) {
        self.base = self.clamp(requested);
        self.current = self.clamp(self.current.max(self.config.min_batch));
    }

    /// Batch size to use for the next fetch.
    #[must_use]
    pub const fn batch_size(&self) -> u32 {
        self.current
    }

    #[must_use]
    pub const fn config(&self) -> &AimdConfig {
        &self.config
    }

    fn clamp(&self, v: u32) -> u32 {
        v.clamp(self.config.min_batch.max(1), self.config.max_batch.max(1))
    }

    /// p95 over the recent latency window, if any samples exist.
    #[must_use]
    pub fn p95_latency(&self) -> Option<Duration> {
        if self.latencies.is_empty() {
            return None;
        }
        let mut v: Vec<Duration> = self.latencies.iter().copied().collect();
        v.sort_unstable();
        let idx = ((v.len() as f64) * 0.95).ceil() as usize;
        Some(v[idx.saturating_sub(1).min(v.len() - 1)])
    }

    /// Feed back the result of a fetch and return the next batch size.
    pub fn observe(&mut self, outcome: FetchOutcome) -> u32 {
        match outcome {
            FetchOutcome::Success { latency, lag } => {
                if self.latencies.len() == LATENCY_WINDOW {
                    self.latencies.pop_front();
                }
                self.latencies.push_back(latency);
                let budget = self.config.latency_budget;
                let under_budget = !matches!(self.p95_latency(), Some(p) if p > budget);

                if lag > self.config.lag_threshold && under_budget {
                    self.current =
                        self.clamp(self.current.saturating_add(self.config.additive_step));
                } else if lag <= self.config.lag_threshold && self.current > self.base {
                    // Steady state: step back down toward the requested size.
                    self.current = self
                        .clamp(self.current.saturating_sub(self.config.additive_step))
                        .max(self.base);
                } else if !under_budget {
                    // Latency over budget while catching up: back off gently.
                    self.current =
                        self.clamp(self.current.saturating_sub(self.config.additive_step));
                }
            }
            FetchOutcome::Congested => {
                self.current = self.clamp(self.current / 2);
            }
            FetchOutcome::OtherError => {}
        }
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AimdConfig {
        AimdConfig {
            min_batch: 1,
            max_batch: 100,
            additive_step: 10,
            lag_threshold: 50,
            latency_budget: Duration::from_millis(500),
        }
    }

    fn ok(ms: u64, lag: u64) -> FetchOutcome {
        FetchOutcome::Success {
            latency: Duration::from_millis(ms),
            lag,
        }
    }

    #[test]
    fn honours_requested_batch_size() {
        let c = AimdController::new(cfg(), 25);
        assert_eq!(c.batch_size(), 25);
    }

    #[test]
    fn requested_size_is_clamped_to_ceiling() {
        let c = AimdController::new(cfg(), 10_000);
        assert_eq!(c.batch_size(), 100);
        let c = AimdController::new(cfg(), 0);
        assert_eq!(c.batch_size(), 1);
    }

    #[test]
    fn grows_while_lagging_and_never_exceeds_ceiling() {
        let mut c = AimdController::new(cfg(), 10);
        for _ in 0..50 {
            let b = c.observe(ok(100, 10_000));
            assert!(b <= 100);
        }
        assert_eq!(c.batch_size(), 100);
    }

    #[test]
    fn halves_on_rate_limit_and_5xx_and_timeout() {
        let mut c = AimdController::new(cfg(), 80);
        let rl = RpcError::RateLimitError { retry_after: None };
        assert_eq!(c.observe(FetchOutcome::from_rpc_error(&rl)), 40);
        let s5 = RpcError::ServerError {
            status: 503,
            message: "x".into(),
        };
        assert_eq!(c.observe(FetchOutcome::from_rpc_error(&s5)), 20);
        let to = RpcError::TimeoutError("t".into());
        assert_eq!(c.observe(FetchOutcome::from_rpc_error(&to)), 10);
        // Never below the floor.
        for _ in 0..10 {
            c.observe(FetchOutcome::Congested);
        }
        assert_eq!(c.batch_size(), 1);
    }

    #[test]
    fn parse_errors_do_not_change_batch() {
        let mut c = AimdController::new(cfg(), 30);
        let pe = RpcError::ParseError("bad".into());
        assert_eq!(c.observe(FetchOutcome::from_rpc_error(&pe)), 30);
        let s4 = RpcError::ServerError {
            status: 400,
            message: "x".into(),
        };
        assert_eq!(FetchOutcome::from_rpc_error(&s4), FetchOutcome::OtherError);
    }

    #[test]
    fn does_not_grow_when_latency_over_budget() {
        let mut c = AimdController::new(cfg(), 50);
        for _ in 0..5 {
            c.observe(ok(2_000, 10_000));
        }
        assert!(c.batch_size() < 50);
    }

    #[test]
    fn decays_to_base_in_steady_state() {
        let mut c = AimdController::new(cfg(), 20);
        for _ in 0..10 {
            c.observe(ok(50, 10_000));
        }
        assert!(c.batch_size() > 20);
        for _ in 0..20 {
            c.observe(ok(50, 0));
        }
        assert_eq!(c.batch_size(), 20);
    }

    /// Simulated mock RPC with injected latency and periodic 429s: the
    /// controller converges, respects the ceiling, and catching up 50k ledgers
    /// takes measurably fewer rounds than fixed batching.
    #[test]
    fn converges_under_injected_429s_and_beats_fixed_batching() {
        let head = 50_000u64;
        let fixed_batch = 20u32;

        let fixed_rounds = head.div_ceil(u64::from(fixed_batch));

        let mut c = AimdController::new(cfg(), fixed_batch);
        let mut cursor = 0u64;
        let mut rounds = 0u64;
        while cursor < head {
            rounds += 1;
            if rounds % 25 == 0 {
                c.observe(FetchOutcome::Congested);
                continue;
            }
            let b = u64::from(c.batch_size());
            assert!(b <= 100, "exceeded ceiling");
            cursor = (cursor + b).min(head);
            c.observe(ok(150, head - cursor));
        }
        assert!(
            rounds < fixed_rounds,
            "adaptive {rounds} rounds vs fixed {fixed_rounds}"
        );
    }

    #[test]
    fn p95_picks_high_tail() {
        let mut c = AimdController::new(cfg(), 10);
        for ms in 1..=20 {
            c.observe(ok(ms * 10, 0));
        }
        assert_eq!(c.p95_latency(), Some(Duration::from_millis(190)));
    }
}
