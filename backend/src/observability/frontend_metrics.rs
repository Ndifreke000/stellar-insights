//! Frontend real user monitoring (RUM) ingestion.
//!
//! `POST /api/metrics/frontend` accepts batches of Web Vitals, page load and API latency
//! metrics plus client errors from the browser. They are exported to Prometheus and kept
//! in a bounded in-memory window so `GET /api/metrics/frontend` can serve a p50/p75/p95
//! summary with performance budget status.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use axum::{http::StatusCode, response::IntoResponse, Json};
use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry, HistogramVec,
    IntCounterVec,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::metrics::REGISTRY;

const MAX_BATCH: usize = 500;
const WINDOW_PER_METRIC: usize = 1000;
const MAX_TRACKED_METRICS: usize = 100;

lazy_static! {
    pub static ref FRONTEND_METRIC_VALUES: HistogramVec = register_histogram_vec_with_registry!(
        "frontend_metric_value",
        "Frontend RUM metric values (ms for timings, unitless for CLS)",
        &["metric"],
        vec![
            0.01, 0.05, 0.1, 0.25, 50.0, 100.0, 200.0, 500.0, 1000.0, 1800.0, 2500.0, 4000.0,
            8000.0
        ],
        REGISTRY
    )
    .unwrap();
    pub static ref FRONTEND_BUDGET_VIOLATIONS: IntCounterVec =
        register_int_counter_vec_with_registry!(
            "frontend_budget_violations_total",
            "Frontend metric samples exceeding their performance budget",
            &["metric"],
            REGISTRY
        )
        .unwrap();
    pub static ref FRONTEND_ERRORS_TOTAL: IntCounterVec = register_int_counter_vec_with_registry!(
        "frontend_errors_total",
        "Client-side errors reported by the frontend",
        &["kind"],
        REGISTRY
    )
    .unwrap();
}

/// Performance budgets (Core Web Vitals "good" thresholds and API latency target).
#[must_use]
pub fn budget_for(metric: &str) -> Option<f64> {
    match metric {
        "web-vitals-lcp" => Some(2500.0),
        "web-vitals-fid" => Some(100.0),
        "web-vitals-inp" => Some(200.0),
        "web-vitals-cls" => Some(0.1),
        "web-vitals-fcp" => Some(1800.0),
        "web-vitals-ttfb" => Some(800.0),
        "page-load-time" => Some(3000.0),
        "api-response-time" => Some(1000.0),
        _ => None,
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FrontendMetric {
    pub name: String,
    pub value: f64,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FrontendError {
    pub message: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({
    "metrics": [
        {"name": "web-vitals-lcp", "value": 1830.5, "path": "/en/corridors", "metadata": {"rating": "good"}},
        {"name": "api-response-time", "value": 212.0, "path": "/en/anchors", "metadata": {"endpoint": "/anchors", "status": 200}}
    ],
    "errors": []
}))]
pub struct FrontendMetricsBatch {
    #[serde(default)]
    pub metrics: Vec<FrontendMetric>,
    #[serde(default)]
    pub errors: Vec<FrontendError>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontendMetricSummary {
    pub name: String,
    pub count: usize,
    pub p50: f64,
    pub p75: f64,
    pub p95: f64,
    pub budget: Option<f64>,
    /// `good` when p75 is within budget, `poor` otherwise, `n/a` without a budget
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontendMetricsSummary {
    pub metrics: Vec<FrontendMetricSummary>,
    /// Per-page p75 for page-level metrics (LCP, page load)
    pub slowest_pages: Vec<PageSummary>,
    pub error_count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PageSummary {
    pub path: String,
    pub metric: String,
    pub p75: f64,
    pub count: usize,
}

#[derive(Default)]
struct Store {
    by_metric: HashMap<String, VecDeque<(String, f64)>>,
    errors: u64,
}

fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::default()))
}

fn sanitize_name(name: &str) -> Option<String> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then(|| name.to_ascii_lowercase())
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Ingest a batch of frontend metrics and errors
#[utoipa::path(
    post,
    path = "/api/metrics/frontend",
    request_body = FrontendMetricsBatch,
    responses(
        (status = 202, description = "Batch accepted"),
        (status = 400, description = "Batch too large")
    ),
    tag = "Metrics"
)]
pub async fn ingest_frontend_metrics(Json(batch): Json<FrontendMetricsBatch>) -> impl IntoResponse {
    if batch.metrics.len() + batch.errors.len() > MAX_BATCH {
        return (StatusCode::BAD_REQUEST, "batch too large").into_response();
    }

    let Ok(mut store) = store().lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    for m in batch.metrics {
        let Some(name) = sanitize_name(&m.name) else {
            continue;
        };
        if !m.value.is_finite() || m.value < 0.0 {
            continue;
        }
        if !store.by_metric.contains_key(&name) && store.by_metric.len() >= MAX_TRACKED_METRICS {
            continue;
        }

        FRONTEND_METRIC_VALUES
            .with_label_values(&[name.as_str()])
            .observe(m.value);
        if budget_for(&name).is_some_and(|b| m.value > b) {
            FRONTEND_BUDGET_VIOLATIONS.with_label_values(&[name.as_str()]).inc();
        }

        let path: String = m
            .path
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        let window = store.by_metric.entry(name).or_default();
        if window.len() >= WINDOW_PER_METRIC {
            window.pop_front();
        }
        window.push_back((path, m.value));
    }

    for e in &batch.errors {
        let kind = e
            .metadata
            .as_ref()
            .and_then(|m| m.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("runtime");
        let kind = if kind == "promise_rejection" { kind } else { "runtime" };
        FRONTEND_ERRORS_TOTAL.with_label_values(&[kind]).inc();
        tracing::warn!(message = %e.message, path = ?e.path, "Frontend error reported");
    }
    store.errors += batch.errors.len() as u64;

    StatusCode::ACCEPTED.into_response()
}

/// Summary of frontend performance (p50/p75/p95 per metric, budget status, slowest pages)
#[utoipa::path(
    get,
    path = "/api/metrics/frontend",
    responses((status = 200, description = "Frontend performance summary", body = FrontendMetricsSummary)),
    tag = "Metrics"
)]
pub async fn frontend_metrics_summary() -> impl IntoResponse {
    let Ok(store) = store().lock() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let mut metrics: Vec<FrontendMetricSummary> = store
        .by_metric
        .iter()
        .map(|(name, window)| {
            let mut values: Vec<f64> = window.iter().map(|(_, v)| *v).collect();
            values.sort_by(f64::total_cmp);
            let p75 = percentile(&values, 75.0);
            let budget = budget_for(name);
            FrontendMetricSummary {
                name: name.clone(),
                count: values.len(),
                p50: percentile(&values, 50.0),
                p75,
                p95: percentile(&values, 95.0),
                budget,
                status: match budget {
                    Some(b) if p75 <= b => "good",
                    Some(_) => "poor",
                    None => "n/a",
                }
                .to_string(),
            }
        })
        .collect();
    metrics.sort_by(|a, b| a.name.cmp(&b.name));

    let mut slowest_pages = Vec::new();
    for metric in ["web-vitals-lcp", "page-load-time"] {
        let Some(window) = store.by_metric.get(metric) else {
            continue;
        };
        let mut by_path: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for (path, v) in window {
            by_path.entry(path.as_str()).or_default().push(*v);
        }
        for (path, mut values) in by_path {
            values.sort_by(f64::total_cmp);
            slowest_pages.push(PageSummary {
                path: path.to_string(),
                metric: metric.to_string(),
                p75: percentile(&values, 75.0),
                count: values.len(),
            });
        }
    }
    slowest_pages.sort_by(|a, b| b.p75.total_cmp(&a.p75));
    slowest_pages.truncate(10);

    Json(FrontendMetricsSummary {
        metrics,
        slowest_pages,
        error_count: store.errors,
    })
    .into_response()
}
