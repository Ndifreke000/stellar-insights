use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};

#[derive(Default)]
struct DurationSeries {
    count: u64,
    sum: f64,
}

#[derive(Default)]
struct MetricsState {
    http_requests_total: Mutex<HashMap<String, u64>>,
    http_request_duration_seconds: Mutex<HashMap<String, DurationSeries>>,
    rpc_calls_total: Mutex<HashMap<String, u64>>,
    rpc_call_duration_seconds: Mutex<HashMap<String, DurationSeries>>,
    cache_operations_total: Mutex<HashMap<String, u64>>,
    errors_total: Mutex<HashMap<String, u64>>,
    db_query_duration_seconds: Mutex<HashMap<String, DurationSeries>>,
    background_jobs_total: Mutex<HashMap<String, u64>>,
    active_connections: AtomicI64,
    corridors_tracked: AtomicI64,
    http_in_flight_requests: AtomicI64,
}

static METRICS: OnceLock<MetricsState> = OnceLock::new();

fn state() -> &'static MetricsState {
    METRICS.get_or_init(MetricsState::default)
}

fn make_key(labels: &[(&str, &str)]) -> String {
    labels
        .iter()
        .map(|(k, v)| format!("{}={}", k, v.replace('|', "_")))
        .collect::<Vec<_>>()
        .join("|")
}

fn key_to_prom_labels(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }

    let labels = key
        .split('|')
        .filter_map(|part| {
            let mut chunks = part.splitn(2, '=');
            let label = chunks.next()?;
            let value = chunks.next().unwrap_or_default().replace('"', "\\\"");
            Some(format!(r#"{label}="{value}""#))
        })
        .collect::<Vec<_>>()
        .join(",");

    format!("{{{labels}}}")
}

fn inc_counter(map: &Mutex<HashMap<String, u64>>, key: String) {
    if let Ok(mut guard) = map.lock() {
        *guard.entry(key).or_insert(0) += 1;
    }
}

fn observe_duration(map: &Mutex<HashMap<String, DurationSeries>>, key: String, seconds: f64) {
    if let Ok(mut guard) = map.lock() {
        let entry = guard.entry(key).or_default();
        entry.count += 1;
        entry.sum += seconds;
    }
}

fn snapshot_counters(map: &Mutex<HashMap<String, u64>>) -> Vec<(String, u64)> {
    map.lock()
        .map(|guard| guard.iter().map(|(k, v)| (k.clone(), *v)).collect())
        .unwrap_or_default()
}

fn snapshot_durations(
    map: &Mutex<HashMap<String, DurationSeries>>,
) -> Vec<(String, DurationSeries)> {
    map.lock()
        .map(|guard| {
            guard
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        DurationSeries {
                            count: v.count,
                            sum: v.sum,
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn init_metrics() {
    let _ = state();
}

pub async fn metrics_handler() -> Response {
    let metrics = state();
    let mut out = String::new();

    out.push_str("# HELP http_requests_total Total HTTP requests\n");
    out.push_str("# TYPE http_requests_total counter\n");
    for (key, value) in snapshot_counters(&metrics.http_requests_total) {
        out.push_str(&format!(
            "http_requests_total{} {}\n",
            key_to_prom_labels(&key),
            value
        ));
    }

    out.push_str("# HELP http_request_duration_seconds HTTP request duration in seconds\n");
    out.push_str("# TYPE http_request_duration_seconds summary\n");
    for (key, series) in snapshot_durations(&metrics.http_request_duration_seconds) {
        let labels = key_to_prom_labels(&key);
        out.push_str(&format!(
            "http_request_duration_seconds_count{} {}\n",
            labels, series.count
        ));
        out.push_str(&format!(
            "http_request_duration_seconds_sum{} {}\n",
            labels, series.sum
        ));
    }

    out.push_str("# HELP rpc_calls_total Total RPC calls\n");
    out.push_str("# TYPE rpc_calls_total counter\n");
    for (key, value) in snapshot_counters(&metrics.rpc_calls_total) {
        out.push_str(&format!(
            "rpc_calls_total{} {}\n",
            key_to_prom_labels(&key),
            value
        ));
    }

    out.push_str("# HELP rpc_call_duration_seconds RPC call duration in seconds\n");
    out.push_str("# TYPE rpc_call_duration_seconds summary\n");
    for (key, series) in snapshot_durations(&metrics.rpc_call_duration_seconds) {
        let labels = key_to_prom_labels(&key);
        out.push_str(&format!(
            "rpc_call_duration_seconds_count{} {}\n",
            labels, series.count
        ));
        out.push_str(&format!(
            "rpc_call_duration_seconds_sum{} {}\n",
            labels, series.sum
        ));
    }

    out.push_str("# HELP cache_operations_total Cache operations by result\n");
    out.push_str("# TYPE cache_operations_total counter\n");
    for (key, value) in snapshot_counters(&metrics.cache_operations_total) {
        out.push_str(&format!(
            "cache_operations_total{} {}\n",
            key_to_prom_labels(&key),
            value
        ));
    }

    out.push_str("# HELP errors_total Total errors by type\n");
    out.push_str("# TYPE errors_total counter\n");
    for (key, value) in snapshot_counters(&metrics.errors_total) {
        out.push_str(&format!(
            "errors_total{} {}\n",
            key_to_prom_labels(&key),
            value
        ));
    }

    out.push_str("# HELP db_query_duration_seconds Database query duration in seconds\n");
    out.push_str("# TYPE db_query_duration_seconds summary\n");
    for (key, series) in snapshot_durations(&metrics.db_query_duration_seconds) {
        let labels = key_to_prom_labels(&key);
        out.push_str(&format!(
            "db_query_duration_seconds_count{} {}\n",
            labels, series.count
        ));
        out.push_str(&format!(
            "db_query_duration_seconds_sum{} {}\n",
            labels, series.sum
        ));
    }

    out.push_str("# HELP background_jobs_total Background jobs by name and status\n");
    out.push_str("# TYPE background_jobs_total counter\n");
    for (key, value) in snapshot_counters(&metrics.background_jobs_total) {
        out.push_str(&format!(
            "background_jobs_total{} {}\n",
            key_to_prom_labels(&key),
            value
        ));
    }

    out.push_str("# HELP active_connections Active websocket connections\n");
    out.push_str("# TYPE active_connections gauge\n");
    out.push_str(&format!(
        "active_connections {}\n",
        metrics.active_connections.load(Ordering::Relaxed)
    ));

    out.push_str("# HELP corridors_tracked Number of tracked corridors\n");
    out.push_str("# TYPE corridors_tracked gauge\n");
    out.push_str(&format!(
        "corridors_tracked {}\n",
        metrics.corridors_tracked.load(Ordering::Relaxed)
    ));

    out.push_str("# HELP http_in_flight_requests In-flight HTTP requests\n");
    out.push_str("# TYPE http_in_flight_requests gauge\n");
    out.push_str(&format!(
        "http_in_flight_requests {}\n",
        metrics.http_in_flight_requests.load(Ordering::Relaxed)
    ));

    (
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        out,
    )
        .into_response()
}

pub async fn http_metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().as_str().to_string();
    let endpoint = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    state()
        .http_in_flight_requests
        .fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();
    state()
        .http_in_flight_requests
        .fetch_sub(1, Ordering::Relaxed);

    let key = make_key(&[
        ("method", method.as_str()),
        ("endpoint", endpoint.as_str()),
        ("status", status.as_str()),
    ]);
    inc_counter(&state().http_requests_total, key.clone());
    observe_duration(&state().http_request_duration_seconds, key, duration);

    if response.status().is_server_error() {
        record_error("http_5xx");
    } else if response.status().is_client_error() {
        record_error("http_4xx");
    }

    response
}

pub fn record_rpc_call(method: &str, status: &str, duration_seconds: f64) {
    let key = make_key(&[("method", method), ("status", status)]);
    inc_counter(&state().rpc_calls_total, key.clone());
    observe_duration(&state().rpc_call_duration_seconds, key, duration_seconds);
}

pub fn record_cache_lookup(hit: bool) {
    let result = if hit { "hit" } else { "miss" };
    inc_counter(
        &state().cache_operations_total,
        make_key(&[("result", result)]),
    );
}

pub fn record_error(error_type: &str) {
    inc_counter(
        &state().errors_total,
        make_key(&[("error_type", error_type)]),
    );
}

pub fn set_active_connections(count: i64) {
    state().active_connections.store(count, Ordering::Relaxed);
}

pub fn observe_db_query(query: &str, status: &str, duration_seconds: f64) {
    observe_duration(
        &state().db_query_duration_seconds,
        make_key(&[("query", query), ("status", status)]),
        duration_seconds,
    );
}

pub fn record_background_job(job: &str, status: &str) {
    inc_counter(
        &state().background_jobs_total,
        make_key(&[("job", job), ("status", status)]),
    );
}

pub fn set_corridors_tracked(count: i64) {
    state().corridors_tracked.store(count, Ordering::Relaxed);
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

        let response = metrics_handler().await;
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(text.contains("rpc_calls_total{method=\"get_latest_ledger\",status=\"success\"}"));
        assert!(text.contains("cache_operations_total{result=\"hit\"}"));
        assert!(text.contains("active_connections 3"));
    }

    #[tokio::test]
    async fn http_middleware_records_request_labels() {
        init_metrics();

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

        let metrics_response = metrics_handler().await;
        let body = to_bytes(metrics_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            text.contains("http_requests_total{method=\"GET\",endpoint=\"/ping\",status=\"200\"}")
        );
    }
}
