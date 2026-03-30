use std::time::Instant;

use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use lazy_static::lazy_static;
use prometheus::{
    gather, register_counter, register_gauge, register_histogram, Counter, Encoder, Gauge,
    Histogram, Registry, TextEncoder,
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Encoder, Gauge,
    HistogramOpts, HistogramVec, Registry, TextEncoder,
    register_counter, register_gauge, register_histogram, Counter, Encoder, Gauge, Histogram,
    HistogramOpts, Registry, TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "endpoint", "status"]
    )
    .unwrap();

    pub static ref HTTP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ),
        &["method", "endpoint", "status"]
    )
    .unwrap();

    pub static ref RPC_CALLS_TOTAL: CounterVec = register_counter_vec!(
        "rpc_calls_total",
        "Total RPC calls",
        &["method", "status"]
    )
    .unwrap();

    pub static ref RPC_CALL_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        HistogramOpts::new(
            "rpc_call_duration_seconds",
            "RPC call duration in seconds"
        ),
        &["method", "status"]
    )
    .unwrap();

    pub static ref CACHE_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "cache_operations_total",
        "Cache operations by result",
        &["result"]
    )
    .unwrap();

    pub static ref ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "errors_total",
        "Total errors by type",
        &["error_type"]
    )
    .unwrap();

    pub static ref DB_QUERY_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        HistogramOpts::new(
            "db_query_duration_seconds",
            "Database query duration in seconds"
        ),
        &["query", "status"]
    )
    .unwrap();

    pub static ref BACKGROUND_JOBS_TOTAL: CounterVec = register_counter_vec!(
        "background_jobs_total",
        "Background jobs by name and status",
        &["job", "status"]
    )
    .unwrap();

    pub static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "active_connections",
        "Active websocket connections"
    )
    .unwrap();

    pub static ref CORRIDORS_TRACKED: Gauge = register_gauge!(
        "corridors_tracked",
        "Number of tracked corridors"
    )
    .unwrap();

    pub static ref HTTP_IN_FLIGHT_REQUESTS: Gauge = register_gauge!(
        "http_in_flight_requests",
        "In-flight HTTP requests"
    )
    .unwrap();

    pub static ref DB_POOL_SIZE: Gauge = register_gauge!(
        "db_pool_size",
        "Total database pool connections"
    )
    .unwrap();

    pub static ref DB_POOL_IDLE: Gauge = register_gauge!(
        "db_pool_idle",
        "Idle database pool connections"
    pub static ref HTTP_REQUESTS_TOTAL: Counter = register_counter!(
        "http_requests_total",
        "Total number of HTTP requests processed"
    )
    .unwrap();
    pub static ref HTTP_REQUEST_DURATION_SECONDS: Histogram = register_histogram!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .unwrap();
    pub static ref RPC_CALLS_TOTAL: Counter =
        register_counter!("rpc_calls_total", "Total number of RPC calls made").unwrap();
    pub static ref RPC_CALL_DURATION_SECONDS: Histogram = register_histogram!(
        "rpc_call_duration_seconds",
        "RPC call duration in seconds",
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    )
    .unwrap();
    pub static ref DB_QUERY_DURATION_SECONDS: Histogram = register_histogram!(
        "db_query_duration_seconds",
        "Database query duration in seconds",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .unwrap();
    pub static ref CACHE_OPERATIONS_TOTAL: Counter =
        register_counter!("cache_operations_total", "Total number of cache operations").unwrap();
    pub static ref ERRORS_TOTAL: Counter =
        register_counter!("errors_total", "Total number of errors encountered").unwrap();
    pub static ref BACKGROUND_JOBS_TOTAL: Counter = register_counter!(
        "background_jobs_total",
        "Total number of background jobs executed"
    )
    .unwrap();
    pub static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "active_connections",
        "Number of active websocket connections"
    )
    .unwrap();
    pub static ref CORRIDORS_TRACKED: Gauge =
        register_gauge!("corridors_tracked", "Number of tracked corridors").unwrap();
    pub static ref HTTP_IN_FLIGHT_REQUESTS: Gauge = register_gauge!(
        "http_in_flight_requests",
        "Number of in-flight HTTP requests"
    )
    .unwrap();
    pub static ref DB_POOL_SIZE: Gauge =
        register_gauge!("db_pool_size", "Total database pool connections").unwrap();
    pub static ref DB_POOL_IDLE: Gauge =
        register_gauge!("db_pool_idle", "Idle database pool connections").unwrap();
    pub static ref DB_POOL_ACTIVE: Gauge =
        register_gauge!("db_pool_active", "Active database pool connections").unwrap();
}

pub fn init_metrics() {
    // Explicitly initialize lazy_statics by accessing them
    let _ = &*REGISTRY;
}

pub fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = gather();
    let mut buffer = vec![];

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error",
        )
            .into_response();
    }

    pub static ref DB_POOL_ACTIVE: Gauge = register_gauge!(
        "db_pool_active",
        "Active database pool connections"
    (
        [("Content-Type", encoder.format_type())],
        Body::from(buffer),
    )
    .unwrap();
}

pub fn init_metrics() {
    // Metrics are registered via lazy_static and the register_* macros which use the global registry by default.
    // However, if we want to use the local REGISTRY, we should explicitly register them there.
    // For simplicity and since most Prometheus integrations expect the global registry,
    // we use the default registry.
}

pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}

pub async fn http_metrics_middleware(req: Request<Body>, _next: Next) -> Response {
    // Note: To keep labels simple for this task, we aren't using them in the global statics
    // unless we use MetricVec, but the requirement was basic aggregation first.
    // For a production app, we'd use int_counter_vec!

    HTTP_IN_FLIGHT_REQUESTS.inc();
    let start = Instant::now();
    let response = _next.run(req).await;
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    HTTP_IN_FLIGHT_REQUESTS.dec();

    HTTP_REQUESTS_TOTAL
        .with_label_values(&[&method, &endpoint, &status])
        .inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[&method, &endpoint, &status])
        .observe(duration);
    HTTP_IN_FLIGHT_REQUESTS.dec();

    HTTP_REQUESTS_TOTAL.inc();
    HTTP_REQUEST_DURATION_SECONDS.observe(duration);

    if response.status().is_server_error() {
        record_error("http_5xx");
    } else if response.status().is_client_error() {
        record_error("http_4xx");
    }

    response
}

pub fn record_rpc_call(method: &str, status: &str, duration_seconds: f64) {
    RPC_CALLS_TOTAL.with_label_values(&[method, status]).inc();
    RPC_CALL_DURATION_SECONDS
        .with_label_values(&[method, status])
        .observe(duration_seconds);
}

pub fn record_cache_lookup(hit: bool) {
    let result = if hit { "hit" } else { "miss" };
    CACHE_OPERATIONS_TOTAL.with_label_values(&[result]).inc();
}

pub fn record_error(error_type: &str) {
    ERRORS_TOTAL.with_label_values(&[error_type]).inc();
pub fn record_rpc_call(_method: &str, _status: &str, duration_seconds: f64) {
    RPC_CALLS_TOTAL.inc();
    RPC_CALL_DURATION_SECONDS.observe(duration_seconds);
}

pub fn record_cache_lookup(_hit: bool) {
    CACHE_OPERATIONS_TOTAL.inc();
}

pub fn record_error(_error_type: &str) {
    ERRORS_TOTAL.inc();
}

pub fn set_active_connections(count: i64) {
    ACTIVE_CONNECTIONS.set(count as f64);
}

pub fn observe_db_query(query: &str, status: &str, duration_seconds: f64) {
    DB_QUERY_DURATION_SECONDS
        .with_label_values(&[query, status])
        .observe(duration_seconds);
}

pub fn record_background_job(job: &str, status: &str) {
    BACKGROUND_JOBS_TOTAL.with_label_values(&[job, status]).inc();
pub fn observe_db_query(_query: &str, _status: &str, duration_seconds: f64) {
    DB_QUERY_DURATION_SECONDS.observe(duration_seconds);
}

pub fn record_background_job(_job: &str, _status: &str) {
    BACKGROUND_JOBS_TOTAL.inc();
}

pub fn set_corridors_tracked(count: i64) {
    CORRIDORS_TRACKED.set(count as f64);
}

pub fn set_pool_size(count: i64) {
    DB_POOL_SIZE.set(count as f64);
}

pub fn set_pool_idle(count: i64) {
    DB_POOL_IDLE.set(count as f64);
}

pub fn set_pool_active(count: i64) {
    DB_POOL_ACTIVE.set(count as f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn metrics_endpoint_contains_rpc_and_cache_metrics() {
        init_metrics();
        record_rpc_call("get_latest_ledger", "success", 0.42);
        record_cache_lookup(true);
        set_active_connections(3);

        let response = metrics_handler();
        let response = metrics_handler().await.into_response();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("rpc_calls_total"));
        assert!(text.contains("cache_operations_total"));
        assert!(text.contains("active_connections 3"));
    }

    #[tokio::test]
    async fn http_middleware_records_request_labels() {
        init_metrics();
        let before = HTTP_REQUESTS_TOTAL.get();

        let app = Router::new()
            .route("/ping", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn(http_metrics_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let metrics_response = metrics_handler();
        let metrics_response = metrics_handler().await.into_response();
        let body = to_bytes(metrics_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(HTTP_REQUESTS_TOTAL.get() >= before + 1.0);
        assert!(text.contains("http_requests_total"));
    }

    #[tokio::test]
    async fn metrics_handler_returns_prometheus_content_type() {
        init_metrics();

        let response = metrics_handler();
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        assert!(
            text.contains("http_requests_total{endpoint=\"/ping\",method=\"GET\",status=\"200\"}")
            content_type.contains("text/plain"),
            "Expected text/plain content type, got: {content_type}"
        );
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_route_is_scrapeable_via_router() {
        init_metrics();

        let app = Router::new().route("/metrics", get(|| async { metrics_handler() }));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .method("GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        // Verify standard Prometheus metric families are present
        assert!(text.contains("# HELP http_requests_total"));
        assert!(text.contains("# HELP"));
        assert!(text.contains("# TYPE"));
    }

    #[tokio::test]
    async fn metrics_handler_is_safe_for_concurrent_access() {
        init_metrics();

        let handles: Vec<_> = (0..10)
            .map(|_| tokio::spawn(async { metrics_handler() }))
            .collect();

        for handle in handles {
            let response = handle.await.unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
        }
    }
}
