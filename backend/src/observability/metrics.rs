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
    Histogram, HistogramOpts, Opts, Registry, TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref HTTP_REQUESTS_TOTAL: Counter = register_counter!(
        "http_requests_total",
        "Total number of HTTP requests processed"
    )
    .expect("Failed to register http_requests_total counter");
    pub static ref HTTP_REQUEST_DURATION_SECONDS: Histogram =
        register_histogram!(HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds with p50/p95/p99 buckets"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]))
        )
        .expect("Failed to register http_request_duration_seconds histogram");
    pub static ref HTTP_REQUEST_DURATION_BY_ENDPOINT: Histogram = register_histogram!(
        HistogramOpts::new(
            "http_request_duration_by_endpoint_seconds",
            "HTTP request duration in seconds per endpoint"
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0])
        .label_names(vec!["method", "endpoint"])
    )
    .expect("Failed to register http_request_duration_by_endpoint_seconds histogram");
    pub static ref RPC_CALLS_TOTAL: Counter =
        register_counter!("rpc_calls_total", "Total number of RPC calls made")
            .expect("Failed to register rpc_calls_total counter");
    pub static ref RPC_CALL_DURATION_SECONDS: Histogram = register_histogram!(HistogramOpts::new(
        "rpc_call_duration_seconds",
        "RPC call duration in seconds"
    ))
    .expect("Failed to register rpc_call_duration_seconds histogram");
    pub static ref DB_QUERY_DURATION_SECONDS: Histogram = register_histogram!(HistogramOpts::new(
        "db_query_duration_seconds",
        "Database query duration in seconds"
    ))
    .expect("Failed to register db_query_duration_seconds histogram");
    pub static ref CACHE_OPERATIONS_TOTAL: Counter =
        register_counter!("cache_operations_total", "Total number of cache operations")
            .expect("Failed to register cache_operations_total counter");
    pub static ref CACHE_HITS_TOTAL: Counter =
        register_counter!("cache_hits_total", "Total number of cache hits")
            .expect("Failed to register cache_hits_total counter");
    pub static ref CACHE_MISSES_TOTAL: Counter =
        register_counter!("cache_misses_total", "Total number of cache misses")
            .expect("Failed to register cache_misses_total counter");
    pub static ref ERRORS_TOTAL: Counter =
        register_counter!("errors_total", "Total number of errors encountered")
            .expect("Failed to register errors_total counter");
    pub static ref HTTP_ERRORS_TOTAL: Counter = register_counter!(Opts::new(
        "http_errors_total",
        "Total number of HTTP errors by status code"
    )
    .label_names(vec!["status_code", "method", "path"]))
    .expect("Failed to register http_errors_total counter");
    pub static ref DB_ERRORS_TOTAL: Counter = register_counter!(Opts::new(
        "db_errors_total",
        "Total number of database errors by type"
    )
    .label_names(vec!["error_type", "query_type"]))
    .expect("Failed to register db_errors_total counter");
    pub static ref RPC_ERRORS_TOTAL: Counter = register_counter!(Opts::new(
        "rpc_errors_total",
        "Total number of RPC errors by method"
    )
    .label_names(vec!["method", "error_type"]))
    .expect("Failed to register rpc_errors_total counter");
    pub static ref BACKGROUND_JOBS_TOTAL: Counter = register_counter!(
        "background_jobs_total",
        "Total number of background jobs executed"
    )
    .expect("Failed to register background_jobs_total counter");
    pub static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "active_connections",
        "Number of active websocket connections"
    )
    .expect("Failed to register active_connections gauge");
    pub static ref CORRIDORS_TRACKED: Gauge =
        register_gauge!("corridors_tracked", "Number of tracked corridors")
            .expect("Failed to register corridors_tracked gauge");
    pub static ref HTTP_IN_FLIGHT_REQUESTS: Gauge = register_gauge!(
        "http_in_flight_requests",
        "Number of in-flight HTTP requests"
    )
    .expect("Failed to register http_in_flight_requests gauge");
    pub static ref DB_POOL_SIZE: Gauge =
        register_gauge!("db_pool_size", "Total database pool connections")
            .expect("Failed to register db_pool_size gauge");
    pub static ref DB_POOL_IDLE: Gauge =
        register_gauge!("db_pool_idle", "Idle database pool connections")
            .expect("Failed to register db_pool_idle gauge");
    pub static ref DB_POOL_ACTIVE: Gauge =
        register_gauge!("db_pool_active", "Active database pool connections")
            .expect("Failed to register db_pool_active gauge");
    pub static ref HTTP_REQUEST_SLO_VIOLATIONS: Counter = register_counter!(Opts::new(
        "http_request_slo_violations_total",
        "Total number of HTTP requests exceeding SLO targets"
    )
    .label_names(vec!["endpoint", "slo_target_ms"]))
    .expect("Failed to register http_request_slo_violations_total counter");
    pub static ref HTTP_RESPONSES_COMPRESSED_TOTAL: Counter = register_counter!(
        "http_responses_compressed_total",
        "Total number of HTTP responses sent with compression (Content-Encoding set)"
    )
    .expect("Failed to register http_responses_compressed_total counter");
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

    (
        [("Content-Type", encoder.format_type())],
        Body::from(buffer),
    )
        .into_response()
}

pub async fn http_metrics_middleware(req: Request<Body>, next: Next) -> Response {
    HTTP_IN_FLIGHT_REQUESTS.inc();
    let start = Instant::now();
    let method = req.method().to_string();
    let uri = req.uri().to_string();
    
    // Normalize endpoint path (remove IDs to group similar endpoints)
    let endpoint = normalize_endpoint(&uri);
    
    let response = next.run(req).await;
    let duration = start.elapsed().as_secs_f64();
    HTTP_IN_FLIGHT_REQUESTS.dec();

    HTTP_REQUEST_DURATION_SECONDS.observe(duration);
    HTTP_REQUEST_DURATION_BY_ENDPOINT
        .with_label_values(&[&method, &endpoint])
        .observe(duration);

    // Check SLO violations
    check_slo_violation(&endpoint, duration * 1000.0);

    if response.headers().contains_key(axum::http::header::CONTENT_ENCODING) {
        HTTP_RESPONSES_COMPRESSED_TOTAL.inc();
    }

    if response.status().is_server_error() {
        record_http_error(response.status().as_u16(), &method, &uri);
        record_error("http_5xx");
    } else if response.status().is_client_error() {
        record_http_error(response.status().as_u16(), &method, &uri);
        record_error("http_4xx");
    }

    response
}

/// Normalize endpoint paths by replacing IDs with placeholders
/// e.g., /api/v1/anchors/123 -> /api/v1/anchors/:id
fn normalize_endpoint(uri: &str) -> String {
    let path = uri.split('?').next().unwrap_or(uri);
    let parts: Vec<&str> = path.split('/').collect();
    
    let normalized = parts
        .iter()
        .map(|part| {
            // Replace UUID-like and numeric IDs with placeholders
            if part.len() > 20 || (part.len() > 0 && part.chars().all(|c| c.is_numeric())) {
                ":id"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    
    if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    }
}

pub fn record_rpc_call(_method: &str, _status: &str, duration_seconds: f64) {
    RPC_CALLS_TOTAL.inc();
    RPC_CALL_DURATION_SECONDS.observe(duration_seconds);
}

pub fn record_cache_lookup(hit: bool) {
    CACHE_OPERATIONS_TOTAL.inc();
    if hit {
        CACHE_HITS_TOTAL.inc();
    } else {
        CACHE_MISSES_TOTAL.inc();
    }
}

pub fn record_cache_hit() {
    CACHE_OPERATIONS_TOTAL.inc();
    CACHE_HITS_TOTAL.inc();
}

pub fn record_cache_miss() {
    CACHE_OPERATIONS_TOTAL.inc();
    CACHE_MISSES_TOTAL.inc();
}

pub fn record_error(error_type: &str) {
    ERRORS_TOTAL.inc();
}

pub fn record_http_error(status_code: u16, method: &str, path: &str) {
    HTTP_ERRORS_TOTAL
        .with_label_values(&[&status_code.to_string(), method, path])
        .inc();
    ERRORS_TOTAL.inc();
}

pub fn record_db_error(error_type: &str, query_type: &str) {
    DB_ERRORS_TOTAL
        .with_label_values(&[error_type, query_type])
        .inc();
    ERRORS_TOTAL.inc();
}

pub fn record_rpc_error(method: &str, error_type: &str) {
    RPC_ERRORS_TOTAL
        .with_label_values(&[method, error_type])
        .inc();
    ERRORS_TOTAL.inc();
}

pub fn set_active_connections(count: i64) {
    ACTIVE_CONNECTIONS.set(count as f64);
}

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

/// Check if request duration violates SLO (p95 < 500ms)
pub fn check_slo_violation(endpoint: &str, duration_ms: f64) {
    const SLO_TARGET_MS: f64 = 500.0;
    if duration_ms > SLO_TARGET_MS {
        HTTP_REQUEST_SLO_VIOLATIONS
            .with_label_values(&[endpoint, "500"])
            .inc();
    }
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
    async fn metrics_endpoint_contains_cache_hit_miss_metrics() {
        init_metrics();
        record_cache_hit();
        record_cache_miss();
        record_cache_lookup(true);
        record_cache_lookup(false);

        let response = metrics_handler();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("cache_hits_total"));
        assert!(text.contains("cache_misses_total"));
        assert!(text.contains("cache_operations_total"));
    }

    #[tokio::test]
    async fn metrics_endpoint_contains_detailed_error_metrics() {
        init_metrics();
        record_http_error(404, "GET", "/api/v1/anchors");
        record_db_error("timeout", "SELECT");
        record_rpc_error("get_latest_ledger", "network_error");

        let response = metrics_handler();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("http_errors_total"));
        assert!(text.contains("db_errors_total"));
        assert!(text.contains("rpc_errors_total"));
        assert!(text.contains("status_code=\"404\""));
        assert!(text.contains("error_type=\"timeout\""));
        assert!(text.contains("method=\"get_latest_ledger\""));
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
