use axum::{routing::get, Json, Router, State};
use serde::Serialize;
use std::sync::Arc;

use crate::cache::{keys, CacheManager};
use crate::cache_middleware::CacheAware;

#[derive(Serialize, Clone)]
pub struct MetricsOverview {
    pub total_volume: f64,
    pub total_transactions: u64,
    pub active_users: u64,
    pub average_transaction_value: f64,
    pub corridor_count: u32,
}

/// Handler for GET /api/metrics/overview (cached with 1 min TTL)
pub async fn metrics_overview(
    State(cache): State<Arc<CacheManager>>,
) -> Json<MetricsOverview> {
    let cache_key = keys::metrics_overview();

    let overview = <()>::get_or_fetch(
        &cache,
        &cache_key,
        cache.config.dashboard_stats_ttl,
        async {
            // Placeholder: Replace with real data aggregation logic
            Ok(MetricsOverview {
                total_volume: 1234567.89,
                total_transactions: 98765,
                active_users: 4321,
                average_transaction_value: 28.56,
                corridor_count: 12,
            })
        },
    )
    .await
    .unwrap_or_else(|_| MetricsOverview {
        total_volume: 0.0,
        total_transactions: 0,
        active_users: 0,
        average_transaction_value: 0.0,
        corridor_count: 0,
    });

    Json(overview)
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
