use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::{TrustlineMetrics, TrustlineSnapshot, TrustlineStat};
use crate::services::trustline_analyzer::TrustlineAnalyzer;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, message).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err.to_string())
    }
}

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
        .route(
            "/:asset_code/:asset_issuer/history",
            get(get_trustline_history),
        )
        .with_state(analyzer)
}

async fn get_trustline_metrics(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
) -> ApiResult<Json<TrustlineMetrics>> {
    let metrics = analyzer.get_metrics().await.unwrap_or(TrustlineMetrics {
        total_assets_tracked: 0,
        total_trustlines_across_network: 0,
        active_assets: 0,
    });
    Ok(Json(metrics))
}

async fn get_trustline_rankings(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
    Query(params): Query<RankingsParams>,
) -> ApiResult<Json<Vec<TrustlineStat>>> {
    let limit = params.limit.clamp(1, 200);
    let rankings = analyzer
        .get_trustline_rankings(limit)
        .await
        .unwrap_or_default();
    Ok(Json(rankings))
}

async fn get_trustline_history(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
    Path((asset_code, asset_issuer)): Path<(String, String)>,
    Query(params): Query<HistoryParams>,
) -> ApiResult<Json<Vec<TrustlineSnapshot>>> {
    let limit = params.limit.clamp(1, 365);
    let history = analyzer
        .get_asset_history(&asset_code, &asset_issuer, limit)
        .await
        .unwrap_or_default();
    Ok(Json(history))
}
