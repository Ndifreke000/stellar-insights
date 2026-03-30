use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::cache::helpers::cached_query;
use crate::cache::{keys, CacheManager};

#[derive(Serialize, Deserialize, Clone)]
pub struct MetricsOverview {
    pub total_volume: f64,
    pub total_transactions: u64,
    pub active_users: u64,
    pub average_transaction_value: f64,
    pub corridor_count: u32,
}

/// Handler for GET /api/metrics/overview (cached with 1 min TTL)
#[utoipa::path(
    get,
    path = "/api/metrics/overview",
    responses(
        (status = 200, description = "Metrics overview", body = MetricsOverview),
        (status = 500, description = "Internal server error")
    ),
    tag = "Metrics"
)]
pub async fn metrics_overview(
    State(cache): State<Arc<CacheManager>>,
    headers: HeaderMap,
) -> Response {
    let cache_key = keys::metrics_overview();

    let overview = cached_query(
        &cache,
        &cache_key,
        cache.config.get_ttl("dashboard"),
        || async {
            // Placeholder: Replace with real data aggregation logic
            Ok(MetricsOverview {
                total_volume: 1_234_567.89,
                total_transactions: 98_765,
                active_users: 4321,
                average_transaction_value: 28.56,
                corridor_count: 12,
            })
        },
    )
    .await
    .unwrap_or(MetricsOverview {
        total_volume: 0.0,
        total_transactions: 0,
        active_users: 0,
        average_transaction_value: 0.0,
        corridor_count: 0,
    });

    let ttl = cache.config.get_ttl("dashboard");
    match crate::http_cache::cached_json_response(&headers, &cache_key, &overview, ttl) {
        Ok(response) => response,
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub fn routes(cache: Arc<CacheManager>) -> Router {
    Router::new()
        .route("/api/metrics/overview", get(metrics_overview))
        .with_state(cache)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_overview_structure() {
        let overview = MetricsOverview {
            total_volume: 1000.0,
            total_transactions: 100,
            active_users: 50,
            average_transaction_value: 10.0,
            corridor_count: 5,
        };

        assert_eq!(overview.total_volume, 1000.0);
        assert_eq!(overview.total_transactions, 100);
        assert_eq!(overview.corridor_count, 5);
    }
}

/// GET /api/admin/pool-metrics - Return current database pool metrics
pub fn get_pool_metrics(State(app_state): State<AppState>) -> Json<crate::database::PoolMetrics> {
    Json(app_state.db.pool_metrics())
}

/// GET /metrics/pool - Pool metrics as JSON (async variant)
pub async fn pool_metrics(
    State(state): State<AppState>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    Json(state.db.pool_metrics()).into_response()
}

#[cfg(test)]
fn render_pool_metrics_prometheus(metrics: &crate::database::PoolMetrics) -> String {
    format!(
        "# HELP stellar_insights_db_pool_size Database pool size\n\
# TYPE stellar_insights_db_pool_size gauge\n\
stellar_insights_db_pool_size {}\n\
# HELP stellar_insights_db_pool_idle Database pool idle connections\n\
# TYPE stellar_insights_db_pool_idle gauge\n\
stellar_insights_db_pool_idle {}\n\
# HELP stellar_insights_db_pool_active Database pool active connections\n\
# TYPE stellar_insights_db_pool_active gauge\n\
stellar_insights_db_pool_active {}\n",
        metrics.size, metrics.idle, metrics.active
    )
}

#[cfg(test)]
mod pool_metrics_tests {
    use super::*;

    #[test]
    fn test_render_pool_metrics_prometheus() {
        let metrics = crate::database::PoolMetrics::new(12, 3, 9);
        let rendered = render_pool_metrics_prometheus(&metrics);

        assert!(rendered.contains("stellar_insights_db_pool_size 12"));
        assert!(rendered.contains("stellar_insights_db_pool_idle 3"));
        assert!(rendered.contains("stellar_insights_db_pool_active 9"));
        assert!(rendered.contains("# TYPE stellar_insights_db_pool_size gauge"));
    }
}
