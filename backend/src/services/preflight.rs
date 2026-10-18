use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Minimum number of samples required before a raw success rate is trusted.
/// Below this, the Wilson lower bound is used so small samples are not
/// reported as misleadingly confident.
pub const MIN_SAMPLE_SIZE: usize = 20;

/// z-score for a 95% confidence interval (two-sided).
const Z_95: f64 = 1.959_963_984_540_054;

/// Short-window configuration for time-decayed success-rate checks.
///
/// A corridor that failed in the last few minutes should not be reported as
/// `proceed` merely because its 24h average is healthy. We therefore compare a
/// short recent window against the long (24h) window and downgrade when the
/// recent window is far below the long one.
///
/// Formula:
///   short_rate = successes_in_short_window / payments_in_short_window
///   long_rate  = successes_in_long_window  / payments_in_long_window
///   if short_window has >= min_payments samples and
///      short_rate < long_rate - drop_threshold  -> downgrade
///
/// The downgrade is `hold` when the short rate is below `hold_floor`, otherwise
/// `caution`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortWindowConfig {
    /// Length of the recent window in minutes.
    pub window_minutes: u64,
    /// Maximum number of recent payments to consider.
    pub max_payments: usize,
    /// Minimum samples required before the short window is trusted.
    pub min_payments: usize,
    /// Absolute drop (short vs long rate) that triggers a downgrade.
    pub drop_threshold: f64,
    /// Short-window rate below which we escalate to `hold`.
    pub hold_floor: f64,
}

impl Default for ShortWindowConfig {
    fn default() -> Self {
        Self {
            window_minutes: 30,
            max_payments: 50,
            min_payments: 10,
            drop_threshold: 0.20,
            hold_floor: 0.50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreflightStatus {
    Proceed,
    Caution,
    Hold,
}

impl PreflightStatus {
    fn severity(self) -> u8 {
        match self {
            PreflightStatus::Proceed => 0,
            PreflightStatus::Caution => 1,
            PreflightStatus::Hold => 2,
        }
    }

    fn max(self, other: PreflightStatus) -> PreflightStatus {
        if other.severity() > self.severity() {
            other
        } else {
            self
        }
    }
}

/// A single historical payment outcome used for success-rate computation.
#[derive(Debug, Clone)]
pub struct PaymentOutcome {
    /// Age of the payment in minutes (0 = just now).
    pub age_minutes: u64,
    pub success: bool,
}

/// Result of evaluating the short window against the long window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortWindowEvaluation {
    pub short_rate: Option<f64>,
    pub long_rate: Option<f64>,
    /// Wilson score lower bound (95%) for the short window, when it has samples.
    pub short_rate_lower_bound: Option<f64>,
    /// Wilson score lower bound (95%) for the long window, when it has samples.
    pub long_rate_lower_bound: Option<f64>,
    pub short_samples: usize,
    pub downgraded: bool,
    pub status: PreflightStatus,
}

/// Compute the Wilson score lower bound at 95% confidence for a binomial
/// proportion.
///
/// `(p + z²/2n - z*sqrt(p(1-p)/n + z²/4n²)) / (1 + z²/n)`
///
/// Returns `None` when there are no samples. For small samples the bound is
/// pulled well below the point estimate, so 19/20 is not treated like 950/1000.
pub fn wilson_lower_bound(successes: usize, total: usize) -> Option<f64> {
    if total == 0 {
        return None;
    }
    let n = total as f64;
    let p = successes as f64 / n;
    let z = Z_95;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let margin = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    let bound = (centre - margin) / denom;
    Some(bound.clamp(0.0, 1.0))
}

/// Compute the success rate over a set of outcomes.
fn success_rate(outcomes: &[PaymentOutcome]) -> Option<f64> {
    if outcomes.is_empty() {
        return None;
    }
    let successes = outcomes.iter().filter(|o| o.success).count();
    Some(successes as f64 / outcomes.len() as f64)
}

/// Compute the Wilson lower bound over a set of outcomes.
fn success_rate_lower_bound(outcomes: &[PaymentOutcome]) -> Option<f64> {
    if outcomes.is_empty() {
        return None;
    }
    let successes = outcomes.iter().filter(|o| o.success).count();
    wilson_lower_bound(successes, outcomes.len())
}

/// Evaluate the short (recent) window against the long (24h) window.
///
/// Returns a `ShortWindowEvaluation` describing whether the recent behaviour
/// warrants a downgrade from the long-window baseline.
pub fn evaluate_short_window(
    outcomes: &[PaymentOutcome],
    config: &ShortWindowConfig,
) -> ShortWindowEvaluation {
    let long_rate = success_rate(outcomes);
    let long_rate_lower_bound = success_rate_lower_bound(outcomes);

    // Collect the most recent payments within the short window, capped at
    // `max_payments`. Outcomes are assumed to be ordered newest-first; we keep
    // the first `max_payments` that fall inside the window.
    let mut recent: VecDeque<&PaymentOutcome> = VecDeque::new();
    for outcome in outcomes {
        if outcome.age_minutes <= config.window_minutes {
            if recent.len() < config.max_payments {
                recent.push_back(outcome);
            }
        }
    }

    let recent_slice: Vec<PaymentOutcome> = recent.into_iter().cloned().collect();
    let short_rate = success_rate(&recent_slice);
    let short_rate_lower_bound = success_rate_lower_bound(&recent_slice);
    let short_samples = recent_slice.len();

    let mut status = PreflightStatus::Proceed;
    let mut downgraded = false;

    if short_samples >= config.min_payments {
        if let (Some(short), Some(long)) = (short_rate, long_rate) {
            // For small samples, compare the Wilson lower bound against the
            // threshold rather than the raw point estimate so that a handful of
            // successes is not reported as misleadingly confident.
            let effective_short = if short_samples < MIN_SAMPLE_SIZE {
                short_rate_lower_bound.unwrap_or(short)
            } else {
                short
            };
            if effective_short < long - config.drop_threshold {
                downgraded = true;
                status = if effective_short < config.hold_floor {
                    PreflightStatus::Hold
                } else {
                    PreflightStatus::Caution
                };
            }
        }
    }

    ShortWindowEvaluation {
        short_rate,
        long_rate,
        short_rate_lower_bound,
        long_rate_lower_bound,
        short_samples,
        downgraded,
        status,
    }
}

/// Combine a baseline status with the short-window evaluation, taking the more
/// severe of the two.
pub fn apply_short_window(
    baseline: PreflightStatus,
    outcomes: &[PaymentOutcome],
    config: &ShortWindowConfig,
) -> (PreflightStatus, ShortWindowEvaluation) {
    let evaluation = evaluate_short_window(outcomes, config);
    (baseline.max(evaluation.status), evaluation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(age_minutes: u64, success: bool) -> PaymentOutcome {
        PaymentOutcome {
            age_minutes,
            success,
        }
    }

    #[test]
    fn wilson_lower_bound_known_values() {
        // 19/20: point estimate 0.95, but the lower bound is much lower.
        let small = wilson_lower_bound(19, 20).unwrap();
        assert!((small - 0.7638).abs() < 1e-3, "got {small}");

        // 950/1000: point estimate 0.95, lower bound stays close to it.
        let large = wilson_lower_bound(950, 1000).unwrap();
        assert!((large - 0.9347).abs() < 1e-3, "got {large}");

        // Small samples are penalised more than large ones.
        assert!(small < large);
    }

    #[test]
    fn wilson_lower_bound_edge_cases() {
        assert_eq!(wilson_lower_bound(0, 0), None);
        assert_eq!(wilson_lower_bound(0, 10), Some(0.0));
        let all = wilson_lower_bound(10, 10).unwrap();
        assert!(all > 0.0 && all < 1.0);
    }

    #[test]
    fn healthy_corridor_proceeds() {
        let outcomes: Vec<_> = (0..40).map(|i| outcome(i * 30, true)).collect();
        let eval = evaluate_short_window(&outcomes, &ShortWindowConfig::default());
        assert_eq!(eval.status, PreflightStatus::Proceed);
        assert!(!eval.downgraded);
    }

    #[test]
    fn synthetic_recent_outage_holds() {
        // Long window is healthy (mostly successes), but the last 30 minutes
        // are almost all failures.
        let mut outcomes: Vec<_> = (0..40).map(|i| outcome(60 + i * 30, true)).collect();
        for i in 0..20 {
            outcomes.insert(0, outcome(i, false));
        }
        let eval = evaluate_short_window(&outcomes, &ShortWindowConfig::default());
        assert!(eval.downgraded);
        assert_eq!(eval.status, PreflightStatus::Hold);
    }

    #[test]
    fn recent_partial_degradation_cautions() {
        // Recent window is below long window but not below hold floor.
        let mut outcomes: Vec<_> = (0..40).map(|i| outcome(60 + i * 30, true)).collect();
        for i in 0..20 {
            outcomes.insert(0, outcome(i, i % 2 == 0));
        }
        let eval = evaluate_short_window(&outcomes, &ShortWindowConfig::default());
        assert!(eval.downgraded);
        assert_eq!(eval.status, PreflightStatus::Caution);
    }

    #[test]
    fn insufficient_recent_samples_do_not_downgrade() {
        let mut outcomes: Vec<_> = (0..40).map(|i| outcome(60 + i * 30, true)).collect();
        outcomes.insert(0, outcome(1, false));
        let eval = evaluate_short_window(&outcomes, &ShortWindowConfig::default());
        assert!(!eval.downgraded);
        assert_eq!(eval.status, PreflightStatus::Proceed);
    }

    #[test]
    fn small_sample_uses_lower_bound() {
        // 19/20 recent successes: raw rate 0.95 would look healthy, but the
        // Wilson lower bound (~0.76) is below the long rate minus threshold.
        let mut outcomes: Vec<_> = (0..40).map(|i| outcome(60 + i * 30, true)).collect();
        for i in 0..20 {
            outcomes.insert(0, outcome(i, i != 0));
        }
        let eval = evaluate_short_window(&outcomes, &ShortWindowConfig::default());
        assert!(eval.short_rate_lower_bound.is_some());
        assert!(eval.downgraded);
    }

    #[test]
    fn apply_short_window_keeps_more_severe() {
        let mut outcomes: Vec<_> = (0..40).map(|i| outcome(60 + i * 30, true)).collect();
        for i in 0..20 {
            outcomes.insert(0, outcome(i, false));
        }
        let (status, _) = apply_short_window(
            PreflightStatus::Caution,
            &outcomes,
            &ShortWindowConfig::default(),
        );
        assert_eq!(status, PreflightStatus::Hold);
    }
}
