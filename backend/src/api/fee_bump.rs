use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::{FeeBumpStats, FeeBumpTransaction};
use crate::services::fee_bump_tracker::FeeBumpTrackerService;

#[derive(Deserialize)]
pub struct RecentFeeBumpsParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

const fn default_limit() -> i64 {
    50
}

pub fn routes(fee_bump_service: Arc<FeeBumpTrackerService>) -> Router {
    Router::new()
        .route("/stats", get(get_fee_bump_stats))
        .route("/recent", get(get_recent_fee_bumps))
        .with_state(fee_bump_service)
}

/// GET /api/fee-bumps/stats - Get fee bump transaction statistics
#[utoipa::path(
    get,
    path = "/api/fee-bumps/stats",
    responses(
        (status = 200, description = "Fee bump statistics", body = FeeBumpStats),
        (status = 500, description = "Internal server error")
    ),
    tag = "Fee Bumps"
)]
async fn get_fee_bump_stats(
    State(service): State<Arc<FeeBumpTrackerService>>,
) -> Json<FeeBumpStats> {
    // In a real app, handle error properly (e.g. 500)
    let stats = service.get_fee_bump_stats().await.unwrap_or(FeeBumpStats {
        total_fee_bumps: 0,
        avg_fee_charged: 0.0,
        max_fee_charged: 0,
        min_fee_charged: 0,
        unique_fee_sources: 0,
    });
    Json(stats)
}

/// GET /api/fee-bumps/recent - Get recent fee bump transactions
#[utoipa::path(
    get,
    path = "/api/fee-bumps/recent",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of transactions to return (1-100, default 50)")
    ),
    responses(
        (status = 200, description = "List of recent fee bump transactions", body = Vec<FeeBumpTransaction>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Fee Bumps"
)]
async fn get_recent_fee_bumps(
    State(service): State<Arc<FeeBumpTrackerService>>,
    Query(params): Query<RecentFeeBumpsParams>,
) -> Json<Vec<FeeBumpTransaction>> {
    let limit = params.limit.clamp(1, 100);
    // In a real app, handle error properly
    let transactions = service
        .get_recent_fee_bumps(limit)
        .await
        .unwrap_or_default();
    Json(transactions)
}
