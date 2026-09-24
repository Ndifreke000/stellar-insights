/// Concurrency-limiting middleware to prevent 500 errors under high load.
///
/// Tracks in-flight requests via an atomic counter. When the limit is reached,
/// new requests receive a `503 Service Unavailable` with a `Retry-After` header
/// instead of queuing indefinitely and exhausting resources.
use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Shared concurrency-limit state.
#[derive(Clone)]
pub struct ConcurrencyLimitState {
    in_flight: Arc<AtomicUsize>,
    max_in_flight: usize,
}

impl ConcurrencyLimitState {
    pub fn new(max_in_flight: usize) -> Self {
        Self {
            in_flight: Arc::new(AtomicUsize::new(0)),
            max_in_flight,
        }
    }

    /// Load from `MAX_IN_FLIGHT_REQUESTS` env var; defaults to 500.
    pub fn from_env() -> Self {
        let max = std::env::var("MAX_IN_FLIGHT_REQUESTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500_usize);
        Self::new(max)
    }

    pub fn current(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }
}

/// Axum middleware function — call via `middleware::from_fn_with_state`.
pub async fn concurrency_limit_middleware(
    axum::extract::State(state): axum::extract::State<ConcurrencyLimitState>,
    req: Request,
    next: Next,
) -> Response {
    let prev = state.in_flight.fetch_add(1, Ordering::Relaxed);
    if prev >= state.max_in_flight {
        state.in_flight.fetch_sub(1, Ordering::Relaxed);
        tracing::warn!(
            in_flight = prev,
            max = state.max_in_flight,
            "Concurrency limit reached — rejecting request"
        );
        crate::observability::metrics::ERRORS_TOTAL.inc();
        let body = Json(json!({
            "error": {
                "code": "CONCURRENCY_LIMIT_EXCEEDED",
                "message": "Server is under heavy load. Please retry shortly."
            }
        }));
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::RETRY_AFTER, HeaderValue::from_static("5"))],
            body,
        )
            .into_response();
    }

    let response = next.run(req).await;
    state.in_flight.fetch_sub(1, Ordering::Relaxed);
    response
}

/// Panic-recovery middleware — converts handler panics into 500 responses
/// instead of dropping the connection, which avoids confusing client errors.
pub async fn panic_recovery_middleware(req: Request<Body>, next: Next) -> Response {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;

    match AssertUnwindSafe(next.run(req)).catch_unwind().await {
        Ok(response) => response,
        Err(_panic) => {
            tracing::error!("Handler panicked — returning 500");
            crate::observability::metrics::ERRORS_TOTAL.inc();
            let body = Json(json!({
                "error": {
                    "code": "INTERNAL_ERROR",
                    "message": "An unexpected error occurred."
                }
            }));
            (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
        }
    }
}
