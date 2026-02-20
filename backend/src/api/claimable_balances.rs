use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::{ClaimableBalance, ClaimableBalanceAnalytics};
use crate::services::claimable_balance_tracker::ClaimableBalanceTracker;

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default)]
    claimed: Option<bool>,
    #[serde(default)]
    asset_code: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct ExpiringParams {
    #[serde(default = "default_days")]
    days: i64,
}

fn default_days() -> i64 {
    30
}

pub fn routes(tracker: Arc<ClaimableBalanceTracker>) -> Router {
    Router::new()
        .route("/", get(list_balances))
        .route("/analytics", get(get_analytics))
        .route("/expiring", get(get_expiring))
        .route("/:id", get(get_balance))
        .with_state(tracker)
}

async fn list_balances(
    State(tracker): State<Arc<ClaimableBalanceTracker>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ClaimableBalance>>, StatusCode> {
    tracker
        .list_balances(
            params.claimed,
            params.asset_code.as_deref(),
            params.limit,
            params.offset,
        )
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_balance(
    State(tracker): State<Arc<ClaimableBalanceTracker>>,
    Path(id): Path<String>,
) -> Result<Json<ClaimableBalance>, StatusCode> {
    match tracker.get_balance(&id).await {
        Ok(Some(balance)) => Ok(Json(balance)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_expiring(
    State(tracker): State<Arc<ClaimableBalanceTracker>>,
    Query(params): Query<ExpiringParams>,
) -> Result<Json<Vec<ClaimableBalance>>, StatusCode> {
    let days = params.days.clamp(1, 365);
    tracker
        .get_expiring_soon(days)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_analytics(
    State(tracker): State<Arc<ClaimableBalanceTracker>>,
) -> Result<Json<ClaimableBalanceAnalytics>, StatusCode> {
    tracker
        .get_analytics()
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
