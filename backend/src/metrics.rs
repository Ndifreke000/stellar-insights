use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Process-wide metrics registry exposed in Prometheus text exposition format.
#[derive(Default)]
pub struct Metrics {
    requests_total: AtomicU64,
    errors_total: AtomicU64,
    request_duration_seconds_sum_micros: AtomicU64,
    request_duration_seconds_count: AtomicU64,
    db_pool_connections: AtomicU64,
    db_pool_idle_connections: AtomicU64,
    cache_hits_total: AtomicU64,
    cache_misses_total: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self, duration_secs: f64, is_error: bool) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.errors_total.fetch_add(1, Ordering::Relaxed);
        }
        self.request_duration_seconds_sum_micros
            .fetch_add((duration_secs * 1_000_000.0) as u64, Ordering::Relaxed);
        self.request_duration_seconds_count
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_db_pool(&self, connections: u64, idle: u64) {
        self.db_pool_connections.store(connections, Ordering::Relaxed);
        self.db_pool_idle_connections.store(idle, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Render the current metric values in Prometheus text exposition format.
    pub fn render(&self) -> String {
        let requests = self.requests_total.load(Ordering::Relaxed);
        let errors = self.errors_total.load(Ordering::Relaxed);
        let duration_sum_micros = self
            .request_duration_seconds_sum_micros
            .load(Ordering::Relaxed);
        let duration_count = self
            .request_duration_seconds_count
            .load(Ordering::Relaxed);
        let duration_sum = duration_sum_micros as f64 / 1_000_000.0;

        let mut out = String::new();
        out.push_str("# HELP http_requests_total Total number of HTTP requests handled.\n");
        out.push_str("# TYPE http_requests_total counter\n");
        out.push_str(&format!("http_requests_total {}\n", requests));

        out.push_str("# HELP http_request_errors_total Total number of HTTP requests that returned an error status.\n");
        out.push_str("# TYPE http_request_errors_total counter\n");
        out.push_str(&format!("http_request_errors_total {}\n", errors));

        out.push_str("# HELP http_request_duration_seconds_sum Total time spent handling HTTP requests.\n");
        out.push_str("# TYPE http_request_duration_seconds_sum counter\n");
        out.push_str(&format!(
            "http_request_duration_seconds_sum {}\n",
            duration_sum
        ));

        out.push_str("# HELP http_request_duration_seconds_count Total number of timed HTTP requests.\n");
        out.push_str("# TYPE http_request_duration_seconds_count counter\n");
        out.push_str(&format!(
            "http_request_duration_seconds_count {}\n",
            duration_count
        ));

        out.push_str("# HELP db_pool_connections Number of connections currently held by the database pool.\n");
        out.push_str("# TYPE db_pool_connections gauge\n");
        out.push_str(&format!(
            "db_pool_connections {}\n",
            self.db_pool_connections.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP db_pool_idle_connections Number of idle connections in the database pool.\n");
        out.push_str("# TYPE db_pool_idle_connections gauge\n");
        out.push_str(&format!(
            "db_pool_idle_connections {}\n",
            self.db_pool_idle_connections.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP cache_hits_total Total number of cache hits.\n");
        out.push_str("# TYPE cache_hits_total counter\n");
        out.push_str(&format!(
            "cache_hits_total {}\n",
            self.cache_hits_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP cache_misses_total Total number of cache misses.\n");
        out.push_str("# TYPE cache_misses_total counter\n");
        out.push_str(&format!(
            "cache_misses_total {}\n",
            self.cache_misses_total.load(Ordering::Relaxed)
        ));

        out
    }
}

/// Axum handler that serves the Prometheus metrics endpoint.
pub async fn metrics_handler(State(metrics): State<Arc<Metrics>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics.render(),
    )
}

/// Middleware that records request count, latency, and error status for every request.
pub async fn track_metrics(
    State(metrics): State<Arc<Metrics>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed = start.elapsed().as_secs_f64();
    let is_error = response.status().is_client_error() || response.status().is_server_error();
    metrics.record_request(elapsed, is_error);
    response
}
