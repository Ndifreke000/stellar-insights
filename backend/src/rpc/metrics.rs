//! Prometheus metrics for RPC error rates and circuit breaker state.

use lazy_static::lazy_static;
use prometheus::{register_int_counter_vec, register_int_gauge_vec, IntCounterVec, IntGaugeVec};

lazy_static! {
    static ref RPC_ERRORS: IntCounterVec = register_int_counter_vec!(
        "rpc_errors_total",
        "Total RPC errors by type and endpoint",
        &["error_type", "endpoint"]
    )
    .expect("rpc_errors_total metric");

    static ref CIRCUIT_BREAKER_STATE: IntGaugeVec = register_int_gauge_vec!(
        "circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=open, 2=half-open)",
        &["endpoint"]
    )
    .expect("circuit_breaker_state metric");
}

/// Record an RPC error for metrics.
pub fn record_rpc_error(error_type: &str, endpoint: &str) {
    RPC_ERRORS
        .with_label_values(&[error_type, endpoint])
        .inc();
}

/// Set circuit breaker state gauge (0=closed, 1=open, 2=half-open).
pub fn set_circuit_breaker_state(endpoint: &str, state: i64) {
    CIRCUIT_BREAKER_STATE
        .with_label_values(&[endpoint])
        .set(state);
}
