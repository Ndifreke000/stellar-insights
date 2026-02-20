use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::services::account_merge_detector::{
    AccountMergeDetector, AccountMergeEvent, AccountMergeStats, DestinationAccountPattern,
};

#[derive(Deserialize)]
pub struct RecentMergesParams {
    #[serde(default = "default_recent_limit")]
    limit: i64,
}

fn default_recent_limit() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct DestinationParams {
    #[serde(default = "default_destination_limit")]
    limit: i64,
}

fn default_destination_limit() -> i64 {
    20
}

pub fn routes(detector: Arc<AccountMergeDetector>) -> Router {
    Router::new()
        .route("/stats", get(get_account_merge_stats))
        .route("/recent", get(get_recent_account_merges))
        .route("/destinations", get(get_destination_patterns))
        .with_state(detector)
}

async fn get_account_merge_stats(
    State(detector): State<Arc<AccountMergeDetector>>,
) -> Json<AccountMergeStats> {
    let stats = detector
        .get_merge_stats()
        .await
        .unwrap_or(AccountMergeStats {
            total_merges: 0,
            total_merged_balance: 0.0,
            unique_sources: 0,
            unique_destinations: 0,
        });

    Json(stats)
}

async fn get_recent_account_merges(
    State(detector): State<Arc<AccountMergeDetector>>,
    Query(params): Query<RecentMergesParams>,
) -> Json<Vec<AccountMergeEvent>> {
    let limit = params.limit.clamp(1, 200);
    let merges = detector.get_recent_merges(limit).await.unwrap_or_default();
    Json(merges)
}

async fn get_destination_patterns(
    State(detector): State<Arc<AccountMergeDetector>>,
    Query(params): Query<DestinationParams>,
) -> Json<Vec<DestinationAccountPattern>> {
    let limit = params.limit.clamp(1, 100);
    let patterns = detector
        .get_destination_patterns(limit)
        .await
        .unwrap_or_default();
    Json(patterns)
}
