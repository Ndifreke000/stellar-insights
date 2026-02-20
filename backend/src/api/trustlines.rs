use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::{TrustlineMetrics, TrustlineSnapshot, TrustlineStat};
use crate::services::trustline_analyzer::TrustlineAnalyzer;

#[derive(Deserialize)]
pub struct RankingsParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct HistoryParams {
    #[serde(default = "default_history_limit")]
    limit: i64,
}

fn default_history_limit() -> i64 {
    30 // 30 days
}

pub fn routes(analyzer: Arc<TrustlineAnalyzer>) -> Router {
    Router::new()
        .route("/stats", get(get_trustline_metrics))
        .route("/rankings", get(get_trustline_rankings))
        .route("/:asset_code/:asset_issuer/history", get(get_trustline_history))
        .with_state(analyzer)
}

async fn get_trustline_metrics(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
) -> Json<TrustlineMetrics> {
    let metrics = analyzer.get_metrics().await.unwrap_or(TrustlineMetrics {
        total_assets_tracked: 0,
        total_trustlines_across_network: 0,
        active_assets: 0,
    });
    Json(metrics)
}

async fn get_trustline_rankings(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
    Query(params): Query<RankingsParams>,
) -> Json<Vec<TrustlineStat>> {
    let limit = params.limit.clamp(1, 200);
    let rankings = analyzer
        .get_trustline_rankings(limit)
        .await
        .unwrap_or_default();
    Json(rankings)
}

async fn get_trustline_history(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
    Path((asset_code, asset_issuer)): Path<(String, String)>,
    Query(params): Query<HistoryParams>,
) -> Json<Vec<TrustlineSnapshot>> {
    let limit = params.limit.clamp(1, 365);
    let history = analyzer
        .get_asset_history(&asset_code, &asset_issuer, limit)
        .await
        .unwrap_or_default();
    Json(history)
}
