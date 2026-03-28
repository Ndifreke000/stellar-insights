use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::cache::helpers::cached_query;
use crate::cache::{keys, CacheManager};

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedMetricsSummary {
    pub cache_hit: bool,
    pub total_volume: f64,
    pub total_transactions: u64,
    pub corridor_count: u32,
}

pub async fn cached_metrics_summary(State(cache): State<Arc<CacheManager>>) -> impl IntoResponse {
    let cache_key = keys::metrics_overview();
    let ttl = cache.config.get_ttl("dashboard");

    let summary = cached_query(&cache, &cache_key, ttl, || async {
        Ok(CachedMetricsSummary {
            cache_hit: false,
            total_volume: 0.0,
            total_transactions: 0,
            corridor_count: 0,
        })
    })
    .await
    .unwrap_or(CachedMetricsSummary {
        cache_hit: false,
        total_volume: 0.0,
        total_transactions: 0,
        corridor_count: 0,
    });

    Json(summary)
}

pub fn routes(cache: Arc<CacheManager>) -> Router {
    Router::new()
        .route("/api/metrics/cached", get(cached_metrics_summary))
        .with_state(cache)
}
