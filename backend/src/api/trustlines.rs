//! Trustline analytics API handlers.
//!
//! #1868 N+1 audit: handlers delegate to single analyzer queries
//! (`get_metrics`, `get_trustline_rankings`, `get_asset_history`) with no
//! per-item `.await` loops. Sync-time upserts run inside one transaction.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::{TrustlineCohortMatrix, TrustlineMetrics, TrustlineSnapshot, TrustlineStat};
use crate::services::trustline_analyzer::TrustlineAnalyzer;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, message).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

#[derive(Deserialize)]
pub struct RankingsParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

const fn default_limit() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct HistoryParams {
    #[serde(default = "default_history_limit")]
    limit: i64,
}

const fn default_history_limit() -> i64 {
    30 // 30 days
}

#[derive(Deserialize)]
pub struct CohortParams {
    #[serde(default = "default_cohort_weeks")]
    weeks: i64,
}

const fn default_cohort_weeks() -> i64 {
    12
}

pub fn routes(analyzer: Arc<TrustlineAnalyzer>) -> Router {
    Router::new()
        .route("/stats", get(get_trustline_metrics))
        .route("/rankings", get(get_trustline_rankings))
        .route(
            "/{asset_code}/{asset_issuer}/history",
            get(get_trustline_history),
        )
        .route(
            "/{asset_code}/{asset_issuer}/cohorts",
            get(get_trustline_cohorts),
        )
        .with_state(analyzer)
}

/// GET /api/trustlines/stats - Get trustline metrics
#[utoipa::path(
    get,
    path = "/api/trustlines/stats",
    responses(
        (status = 200, description = "Trustline metrics", body = TrustlineMetrics),
        (status = 500, description = "Internal server error")
    ),
    tag = "Trustlines"
)]
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

/// GET /api/trustlines/rankings - Get trustline rankings
#[utoipa::path(
    get,
    path = "/api/trustlines/rankings",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of rankings to return (1-200, default 50)")
    ),
    responses(
        (status = 200, description = "Trustline rankings", body = Vec<TrustlineStat>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Trustlines"
)]
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

/// GET /api/trustlines/{asset_code}/{asset_issuer}/history - Get trustline history for an asset
#[utoipa::path(
    get,
    path = "/api/trustlines/{asset_code}/{asset_issuer}/history",
    params(
        ("asset_code" = String, Path, description = "Asset code (e.g., 'USDC')"),
        ("asset_issuer" = String, Path, description = "Asset issuer account ID"),
        ("limit" = Option<i64>, Query, description = "Maximum number of history entries to return (1-365, default 30)")
    ),
    responses(
        (status = 200, description = "Trustline history for asset", body = Vec<TrustlineSnapshot>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Trustlines"
)]
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

/// GET /api/trustlines/{asset_code}/{asset_issuer}/cohorts - Get trustline cohort retention matrix
#[utoipa::path(
    get,
    path = "/api/trustlines/{asset_code}/{asset_issuer}/cohorts",
    params(
        ("asset_code" = String, Path, description = "Asset code (e.g., 'USDC')"),
        ("asset_issuer" = String, Path, description = "Asset issuer account ID"),
        ("weeks" = Option<i64>, Query, description = "Number of cohort weeks to return (1-52, default 12)")
    ),
    responses(
        (status = 200, description = "Trustline cohort retention matrix", body = TrustlineCohortMatrix),
        (status = 500, description = "Internal server error")
    ),
    tag = "Trustlines"
)]
async fn get_trustline_cohorts(
    State(analyzer): State<Arc<TrustlineAnalyzer>>,
    Path((asset_code, asset_issuer)): Path<(String, String)>,
    Query(params): Query<CohortParams>,
) -> ApiResult<Json<TrustlineCohortMatrix>> {
    let weeks = params.weeks.clamp(1, 52);
    let matrix = analyzer
        .get_cohort_retention(&asset_code, &asset_issuer, weeks)
        .await
        .unwrap_or(TrustlineCohortMatrix {
            asset_code,
            asset_issuer,
            total_tracked_accounts: 0,
            spam_accounts_excluded: 0,
            cohorts: Vec::new(),
        });
    Ok(Json(matrix))
}
