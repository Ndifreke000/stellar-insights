use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::{LiquidityPool, LiquidityPoolSnapshot, LiquidityPoolStats};
use crate::services::liquidity_pool_analyzer::LiquidityPoolAnalyzer;

#[derive(Deserialize)]
pub struct RankingsParams {
    #[serde(default = "default_sort")]
    sort_by: String,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_sort() -> String {
    "apy".to_string()
}

const fn default_limit() -> i64 {
    20
}

#[derive(Deserialize)]
pub struct SnapshotParams {
    #[serde(default = "default_snapshot_limit")]
    limit: i64,
}

const fn default_snapshot_limit() -> i64 {
    100
}

pub fn routes(analyzer: Arc<LiquidityPoolAnalyzer>) -> Router {
    Router::new()
        .route("/", get(list_pools))
        .route("/stats", get(get_pool_stats))
        .route("/rankings", get(get_pool_rankings))
        .route("/:pool_id", get(get_pool_detail))
        .route("/:pool_id/snapshots", get(get_pool_snapshots))
        .with_state(analyzer)
}

/// GET /api/liquidity-pools/ - List all liquidity pools
#[utoipa::path(
    get,
    path = "/api/liquidity-pools/",
    responses(
        (status = 200, description = "List of liquidity pools", body = Vec<LiquidityPool>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Liquidity Pools"
)]
async fn list_pools(
    State(analyzer): State<Arc<LiquidityPoolAnalyzer>>,
) -> Json<Vec<LiquidityPool>> {
    let pools = analyzer.get_all_pools().await.unwrap_or_default();
    Json(pools)
}

/// GET /api/liquidity-pools/stats - Get aggregated liquidity pool statistics
#[utoipa::path(
    get,
    path = "/api/liquidity-pools/stats",
    responses(
        (status = 200, description = "Liquidity pool statistics", body = LiquidityPoolStats),
        (status = 500, description = "Internal server error")
    ),
    tag = "Liquidity Pools"
)]
async fn get_pool_stats(
    State(analyzer): State<Arc<LiquidityPoolAnalyzer>>,
) -> Json<LiquidityPoolStats> {
    let stats = analyzer
        .get_pool_stats()
        .await
        .unwrap_or(LiquidityPoolStats {
            total_pools: 0,
            total_liquidity_usd: 0.0,
            avg_pool_size_usd: 0.0,
            total_value_locked_usd: 0.0,
            total_volume_24h_usd: 0.0,
            total_fees_24h_usd: 0.0,
            avg_apy: 0.0,
            avg_impermanent_loss: 0.0,
        });
    Json(stats)
}

/// GET /api/liquidity-pools/rankings - Get liquidity pool rankings
#[utoipa::path(
    get,
    path = "/api/liquidity-pools/rankings",
    params(
        ("sort_by" = Option<String>, Query, description = "Sort field (e.g., 'apy', default 'apy')"),
        ("limit" = Option<i64>, Query, description = "Maximum number of pools to return (1-100, default 20)")
    ),
    responses(
        (status = 200, description = "Liquidity pool rankings", body = Vec<LiquidityPool>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Liquidity Pools"
)]
async fn get_pool_rankings(
    State(analyzer): State<Arc<LiquidityPoolAnalyzer>>,
    Query(params): Query<RankingsParams>,
) -> Json<Vec<LiquidityPool>> {
    let limit = params.limit.clamp(1, 100);
    let pools = analyzer
        .get_pool_rankings(&params.sort_by, limit)
        .await
        .unwrap_or_default();
    Json(pools)
}

#[derive(serde::Serialize)]
struct PoolDetailResponse {
    pool: LiquidityPool,
    snapshots: Vec<LiquidityPoolSnapshot>,
}

/// GET /api/liquidity-pools/{pool_id} - Get liquidity pool details
#[utoipa::path(
    get,
    path = "/api/liquidity-pools/{pool_id}",
    params(
        ("pool_id" = String, Path, description = "Liquidity pool ID")
    ),
    responses(
        (status = 200, description = "Liquidity pool details", body = PoolDetailResponse),
        (status = 404, description = "Liquidity pool not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Liquidity Pools"
)]
async fn get_pool_detail(
    State(analyzer): State<Arc<LiquidityPoolAnalyzer>>,
    Path(pool_id): Path<String>,
) -> Result<Json<PoolDetailResponse>, axum::http::StatusCode> {
    match analyzer.get_pool_detail(&pool_id).await {
        Ok((pool, snapshots)) => Ok(Json(PoolDetailResponse { pool, snapshots })),
        Err(_) => Err(axum::http::StatusCode::NOT_FOUND),
    }
}

/// GET /api/liquidity-pools/{pool_id}/snapshots - Get liquidity pool snapshots
#[utoipa::path(
    get,
    path = "/api/liquidity-pools/{pool_id}/snapshots",
    params(
        ("pool_id" = String, Path, description = "Liquidity pool ID"),
        ("limit" = Option<i64>, Query, description = "Maximum number of snapshots to return (1-500, default 100)")
    ),
    responses(
        (status = 200, description = "Liquidity pool snapshots", body = Vec<LiquidityPoolSnapshot>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Liquidity Pools"
)]
async fn get_pool_snapshots(
    State(analyzer): State<Arc<LiquidityPoolAnalyzer>>,
    Path(pool_id): Path<String>,
    Query(params): Query<SnapshotParams>,
) -> Json<Vec<LiquidityPoolSnapshot>> {
    let limit = params.limit.clamp(1, 500);
    let snapshots = analyzer
        .get_pool_snapshots(&pool_id, limit)
        .await
        .unwrap_or_default();
    Json(snapshots)
}
