/// Service Level Objectives (SLOs) for Stellar Insights API
///
/// Targets:
///   - p95 response time  < 500 ms  (approximated as mean latency budget)
///   - Availability       >= 99.9 %
///   - Error rate         < 1 %
///
/// `SloEvaluator::evaluate()` reads the Prometheus text output from
/// `metrics_handler` and derives compliance gauges, then appends them to
/// the next `/metrics` scrape via `append_slo_metrics`.
///
/// Call `spawn_slo_evaluator()` once at startup alongside `init_metrics()`.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

// ── SLO targets ───────────────────────────────────────────────────────────────

/// Mean latency budget in seconds (proxy for p95 when histogram unavailable).
pub const SLO_P95_LATENCY_S: f64 = 0.500;

/// Minimum availability ratio (successful / total).
pub const SLO_AVAILABILITY: f64 = 0.999;

/// Maximum error ratio (5xx / total).
pub const SLO_MAX_ERROR_RATE: f64 = 0.010;

/// How often the evaluator re-checks compliance.
pub const EVAL_INTERVAL: Duration = Duration::from_secs(60);

// ── Shared SLO state (atomic bit-packed f64 via u64) ─────────────────────────

/// Stores the latest SLO gauge values so `metrics_handler` can append them.
pub struct SloState {
    /// 1.0 = met, 0.0 = breached — packed as u64 bits.
    pub p95_compliance: AtomicU64,
    pub availability_compliance: AtomicU64,
    pub error_rate_compliance: AtomicU64,
    /// Current measured values.
    pub p95_value: AtomicU64,
    pub availability_value: AtomicU64,
    pub error_rate_value: AtomicU64,
}

impl SloState {
    const fn new() -> Self {
        // Default: all SLOs met, values at ideal state.
        Self {
            p95_compliance: AtomicU64::new(0x3FF0000000000000u64), // 1.0_f64
            availability_compliance: AtomicU64::new(0x3FF0000000000000u64),
            error_rate_compliance: AtomicU64::new(0x3FF0000000000000u64),
            p95_value: AtomicU64::new(0),
            availability_value: AtomicU64::new(0x3FF0000000000000u64), // 1.0
            error_rate_value: AtomicU64::new(0),
        }
    }

    fn set_f64(atomic: &AtomicU64, v: f64) {
        atomic.store(v.to_bits(), Ordering::Relaxed);
    }

    fn get_f64(atomic: &AtomicU64) -> f64 {
        f64::from_bits(atomic.load(Ordering::Relaxed))
    }
}

static SLO_STATE: OnceLock<SloState> = OnceLock::new();

fn slo_state() -> &'static SloState {
    SLO_STATE.get_or_init(SloState::new)
}

pub fn init_slo_metrics() {
    let _ = slo_state();
}

// ── SLO evaluator ─────────────────────────────────────────────────────────────

pub struct SloEvaluator;

impl SloEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate all SLOs from the current Prometheus text output.
    pub async fn evaluate(&self) {
        use axum::body::to_bytes;
        use crate::observability::metrics::metrics_handler;

        let response = metrics_handler().await;
        let Ok(bytes) = to_bytes(response.into_body(), 1024 * 1024).await else {
            return;
        };
        let text = String::from_utf8_lossy(&bytes);

        let (total_requests, total_duration_sum, total_duration_count, errors) =
            parse_metrics(&text);

        self.eval_latency(total_duration_sum, total_duration_count);
        self.eval_availability(total_requests, errors);
        self.eval_error_rate(total_requests, errors);
    }

    fn eval_latency(&self, sum: f64, count: f64) {
        let mean = if count > 0.0 { sum / count } else { 0.0 };
        let met = mean <= SLO_P95_LATENCY_S;
        let state = slo_state();
        SloState::set_f64(&state.p95_compliance, if met { 1.0 } else { 0.0 });
        SloState::set_f64(&state.p95_value, mean);
        if !met {
            tracing::warn!(
                slo = "p95_latency",
                current_s = mean,
                target_s = SLO_P95_LATENCY_S,
                "SLO BREACHED: mean latency exceeds target"
            );
        }
    }

    fn eval_availability(&self, total: f64, errors: f64) {
        let availability = if total > 0.0 {
            (total - errors) / total
        } else {
            1.0
        };
        let met = availability >= SLO_AVAILABILITY;
        let state = slo_state();
        SloState::set_f64(&state.availability_compliance, if met { 1.0 } else { 0.0 });
        SloState::set_f64(&state.availability_value, availability);
        if !met {
            tracing::warn!(
                slo = "availability",
                current = availability,
                target = SLO_AVAILABILITY,
                "SLO BREACHED: availability below target"
            );
        }
    }

    fn eval_error_rate(&self, total: f64, errors: f64) {
        let rate = if total > 0.0 { errors / total } else { 0.0 };
        let met = rate < SLO_MAX_ERROR_RATE;
        let state = slo_state();
        SloState::set_f64(&state.error_rate_compliance, if met { 1.0 } else { 0.0 });
        SloState::set_f64(&state.error_rate_value, rate);
        if !met {
            tracing::warn!(
                slo = "error_rate",
                current = rate,
                target = SLO_MAX_ERROR_RATE,
                "SLO BREACHED: error rate exceeds target"
            );
        }
    }
}

impl Default for SloEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Append SLO gauge lines to an existing Prometheus text output.
pub fn append_slo_metrics(out: &mut String) {
    let s = slo_state();

    out.push_str("# HELP slo_compliance 1 if the SLO is currently met, 0 if breached\n");
    out.push_str("# TYPE slo_compliance gauge\n");
    out.push_str(&format!(
        "slo_compliance{{slo=\"p95_latency\"}} {}\n",
        SloState::get_f64(&s.p95_compliance)
    ));
    out.push_str(&format!(
        "slo_compliance{{slo=\"availability\"}} {}\n",
        SloState::get_f64(&s.availability_compliance)
    ));
    out.push_str(&format!(
        "slo_compliance{{slo=\"error_rate\"}} {}\n",
        SloState::get_f64(&s.error_rate_compliance)
    ));

    out.push_str("# HELP slo_current_value Current measured value for each SLO metric\n");
    out.push_str("# TYPE slo_current_value gauge\n");
    out.push_str(&format!(
        "slo_current_value{{slo=\"p95_latency_seconds\"}} {}\n",
        SloState::get_f64(&s.p95_value)
    ));
    out.push_str(&format!(
        "slo_current_value{{slo=\"availability_ratio\"}} {}\n",
        SloState::get_f64(&s.availability_value)
    ));
    out.push_str(&format!(
        "slo_current_value{{slo=\"error_rate_ratio\"}} {}\n",
        SloState::get_f64(&s.error_rate_value)
    ));
}

/// Spawn a background Tokio task that evaluates SLOs on a fixed interval.
pub fn spawn_slo_evaluator() {
    tokio::spawn(async move {
        init_slo_metrics();
        let evaluator = SloEvaluator::new();
        let mut interval = tokio::time::interval(EVAL_INTERVAL);
        loop {
            interval.tick().await;
            evaluator.evaluate().await;
            tracing::debug!("SLO evaluation complete");
        }
    });
}

// ── Parse helpers ─────────────────────────────────────────────────────────────

/// Extract (total_requests, duration_sum, duration_count, errors) from
/// the Prometheus text format produced by `metrics_handler`.
fn parse_metrics(text: &str) -> (f64, f64, f64, f64) {
    let mut total_requests = 0.0f64;
    let mut duration_sum = 0.0f64;
    let mut duration_count = 0.0f64;
    let mut errors = 0.0f64;

    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        if let Some(v) = parse_metric_value(line, "http_requests_total") {
            total_requests += v;
        }
        if let Some(v) = parse_metric_value(line, "http_request_duration_seconds_sum") {
            duration_sum += v;
        }
        if let Some(v) = parse_metric_value(line, "http_request_duration_seconds_count") {
            duration_count += v;
        }
        if let Some(v) = parse_metric_value(line, "errors_total") {
            errors += v;
        }
    }

    (total_requests, duration_sum, duration_count, errors)
}

fn parse_metric_value(line: &str, name: &str) -> Option<f64> {
    let rest = if line.starts_with(name) {
        &line[name.len()..]
    } else {
        return None;
    };
    // rest is either " <value>" or "{...} <value>"
    let value_str = if rest.starts_with('{') {
        rest.rsplit_once('}')?.1.trim()
    } else {
        rest.trim()
    };
    value_str.parse().ok()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slo_constants_are_sensible() {
        assert!(SLO_P95_LATENCY_S > 0.0 && SLO_P95_LATENCY_S <= 1.0);
        assert!(SLO_AVAILABILITY > 0.99 && SLO_AVAILABILITY <= 1.0);
        assert!(SLO_MAX_ERROR_RATE > 0.0 && SLO_MAX_ERROR_RATE < 0.05);
        assert!(EVAL_INTERVAL.as_secs() > 0);
    }

    #[test]
    fn parse_metrics_empty() {
        let (req, sum, count, err) = parse_metrics("");
        assert_eq!(req, 0.0);
        assert_eq!(sum, 0.0);
        assert_eq!(count, 0.0);
        assert_eq!(err, 0.0);
    }

    #[test]
    fn parse_metrics_with_labels() {
        let text = r#"# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",endpoint="/api/anchors",status="200"} 42
http_request_duration_seconds_sum{method="GET",endpoint="/api/anchors",status="200"} 4.2
http_request_duration_seconds_count{method="GET",endpoint="/api/anchors",status="200"} 42
errors_total{error_type="http_5xx"} 1
"#;
        let (req, sum, count, err) = parse_metrics(text);
        assert_eq!(req, 42.0);
        assert!((sum - 4.2).abs() < 1e-9);
        assert_eq!(count, 42.0);
        assert_eq!(err, 1.0);
    }

    #[test]
    fn slo_state_defaults_to_met() {
        init_slo_metrics();
        let s = slo_state();
        assert_eq!(SloState::get_f64(&s.p95_compliance), 1.0);
        assert_eq!(SloState::get_f64(&s.availability_compliance), 1.0);
        assert_eq!(SloState::get_f64(&s.error_rate_compliance), 1.0);
    }

    #[test]
    fn append_slo_metrics_contains_required_lines() {
        init_slo_metrics();
        let mut out = String::new();
        append_slo_metrics(&mut out);
        assert!(out.contains("slo_compliance{slo=\"p95_latency\"}"));
        assert!(out.contains("slo_compliance{slo=\"availability\"}"));
        assert!(out.contains("slo_compliance{slo=\"error_rate\"}"));
        assert!(out.contains("slo_current_value{slo=\"p95_latency_seconds\"}"));
        assert!(out.contains("slo_current_value{slo=\"availability_ratio\"}"));
        assert!(out.contains("slo_current_value{slo=\"error_rate_ratio\"}"));
    }

    #[test]
    fn evaluator_marks_error_rate_breached() {
        init_slo_metrics();
        let ev = SloEvaluator::new();
        // 10 errors out of 20 requests = 50% error rate → breached
        ev.eval_error_rate(20.0, 10.0);
        assert_eq!(
            SloState::get_f64(&slo_state().error_rate_compliance),
            0.0
        );
    }

    #[test]
    fn evaluator_marks_availability_breached() {
        init_slo_metrics();
        let ev = SloEvaluator::new();
        // 5 errors out of 10 = 50% availability → breached
        ev.eval_availability(10.0, 5.0);
        assert_eq!(
            SloState::get_f64(&slo_state().availability_compliance),
            0.0
        );
    }
}
