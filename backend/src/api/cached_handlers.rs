/// Cached API handlers with Redis caching layer
/// Implements cache-aside pattern with proper invalidation

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, warn};

use crate::cache::{RedisCache, CacheKey};
use crate::database::Database;
use crate::handlers::{ApiError, ApiResult};
use crate::models::corridor::Corridor;
use crate::models::{AnchorDetailResponse, CreateAnchorRequest, CreateCorridorRequest};

// TTL constants (in seconds)
const CORRIDOR_METRICS_TTL: usize = 300;      // 5 minutes
const ANCHOR_DATA_TTL: usize = 600;           // 10 minutes
#[allow(dead_code)]
const DASHBOARD_STATS_TTL: usize = 60;        // 1 minute (ready for dashboard integration)

#[derive(Debug, Deserialize)]
pub struct ListAnchorsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListAnchorsResponse {
    pub anchors: Vec<crate::models::Anchor>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListCorridorsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListCorridorsResponse {
    pub corridors: Vec<Corridor>,
    pub total: usize,
}

/// GET /api/anchors - List all anchors with caching
pub async fn list_anchors_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Query(params): Query<ListAnchorsQuery>,
) -> ApiResult<Json<ListAnchorsResponse>> {
    let cache_key = CacheKey::anchor_list(params.limit, params.offset);

    // Try to get from cache
    if let Ok(Some(cached)) = cache.get::<ListAnchorsResponse>(&cache_key).await {
        debug!("Returning cached anchor list");
        return Ok(Json(cached));
    }

    // Cache miss - fetch from database
    let anchors = db.list_anchors(params.limit, params.offset).await?;
    let total = anchors.len();

    let response = ListAnchorsResponse { anchors, total };

    // Store in cache
    if let Err(e) = cache.set(&cache_key, &response, ANCHOR_DATA_TTL).await {
        warn!("Failed to cache anchor list: {}", e);
        // Don't fail the request if caching fails
    }

    Ok(Json(response))
}

/// GET /api/anchors/:id - Get detailed anchor information with caching
pub async fn get_anchor_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AnchorDetailResponse>> {
    let cache_key = CacheKey::anchor_detail(&id.to_string());

    // Try to get from cache
    if let Ok(Some(cached)) = cache.get::<AnchorDetailResponse>(&cache_key).await {
        debug!("Returning cached anchor detail");
        return Ok(Json(cached));
    }

    // Cache miss - fetch from database
    let anchor_detail = db
        .get_anchor_detail(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Anchor with id {} not found", id)))?;

    // Store in cache
    if let Err(e) = cache.set(&cache_key, &anchor_detail, ANCHOR_DATA_TTL).await {
        warn!("Failed to cache anchor detail: {}", e);
    }

    Ok(Json(anchor_detail))
}

/// GET /api/anchors/account/:stellar_account - Get anchor by Stellar account with caching
pub async fn get_anchor_by_account_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Path(stellar_account): Path<String>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let cache_key = format!("anchor:account:{}", stellar_account);

    // Try to get from cache
    if let Ok(Some(cached)) = cache.get::<crate::models::Anchor>(&cache_key).await {
        debug!("Returning cached anchor by account");
        return Ok(Json(cached));
    }

    // Cache miss - fetch from database
    let anchor = db
        .get_anchor_by_stellar_account(&stellar_account)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Anchor with stellar account {} not found",
                stellar_account
            ))
        })?;

    // Store in cache
    if let Err(e) = cache.set(&cache_key, &anchor, ANCHOR_DATA_TTL).await {
        warn!("Failed to cache anchor by account: {}", e);
    }

    Ok(Json(anchor))
}

/// POST /api/anchors - Create a new anchor (invalidates cache)
pub async fn create_anchor_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Json(req): Json<CreateAnchorRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    if req.name.is_empty() {
        return Err(ApiError::BadRequest("Name cannot be empty".to_string()));
    }

    if req.stellar_account.is_empty() {
        return Err(ApiError::BadRequest(
            "Stellar account cannot be empty".to_string(),
        ));
    }

    let anchor = db.create_anchor(req).await?;

    // Invalidate anchor list cache
    if let Err(e) = cache.delete_pattern(&CacheKey::anchor_pattern()).await {
        warn!("Failed to invalidate anchor cache: {}", e);
    }

    Ok(Json(anchor))
}

/// PUT /api/anchors/:id/metrics - Update anchor metrics (invalidates cache)
#[derive(Debug, Deserialize)]
pub struct UpdateMetricsRequest {
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
}

pub async fn update_anchor_metrics_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMetricsRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    // Verify anchor exists
    if db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let anchor = db
        .update_anchor_metrics(
            id,
            req.total_transactions,
            req.successful_transactions,
            req.failed_transactions,
            req.avg_settlement_time_ms,
            req.volume_usd,
        )
        .await?;

    // Invalidate anchor caches
    if let Err(e) = cache.delete_pattern(&CacheKey::anchor_pattern()).await {
        warn!("Failed to invalidate anchor cache: {}", e);
    }
    if let Err(e) = cache.delete_pattern(&CacheKey::dashboard_pattern()).await {
        warn!("Failed to invalidate dashboard cache: {}", e);
    }

    Ok(Json(anchor))
}

/// GET /api/anchors/:id/assets - Get assets issued by anchor with caching
pub async fn get_anchor_assets_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<crate::models::Asset>>> {
    let cache_key = CacheKey::anchor_assets(&id.to_string());

    // Try to get from cache
    if let Ok(Some(cached)) = cache.get::<Vec<crate::models::Asset>>(&cache_key).await {
        debug!("Returning cached anchor assets");
        return Ok(Json(cached));
    }

    // Verify anchor exists
    if db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let assets = db.get_assets_by_anchor(id).await?;

    // Store in cache
    if let Err(e) = cache.set(&cache_key, &assets, ANCHOR_DATA_TTL).await {
        warn!("Failed to cache anchor assets: {}", e);
    }

    Ok(Json(assets))
}

/// POST /api/anchors/:id/assets - Add asset to anchor (invalidates cache)
#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub asset_code: String,
    pub asset_issuer: String,
}

pub async fn create_anchor_asset_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateAssetRequest>,
) -> ApiResult<Json<crate::models::Asset>> {
    // Verify anchor exists
    if db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let asset = db
        .create_asset(id, req.asset_code, req.asset_issuer)
        .await?;

    // Invalidate anchor caches
    if let Err(e) = cache.delete(&CacheKey::anchor_assets(&id.to_string())).await {
        warn!("Failed to invalidate anchor assets cache: {}", e);
    }
    if let Err(e) = cache.delete(&CacheKey::anchor_detail(&id.to_string())).await {
        warn!("Failed to invalidate anchor detail cache: {}", e);
    }

    Ok(Json(asset))
}

/// GET /api/corridors - List corridors with caching
pub async fn list_corridors_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<ListCorridorsResponse>> {
    let cache_key = CacheKey::corridor_list(params.limit, params.offset, "default");

    // Try to get from cache
    if let Ok(Some(cached)) = cache.get::<ListCorridorsResponse>(&cache_key).await {
        debug!("Returning cached corridor list");
        return Ok(Json(cached));
    }

    // Cache miss - fetch from database
    let corridors = db.list_corridors(params.limit, params.offset).await?;
    let total = corridors.len();

    let response = ListCorridorsResponse { corridors, total };

    // Store in cache
    if let Err(e) = cache.set(&cache_key, &response, CORRIDOR_METRICS_TTL).await {
        warn!("Failed to cache corridor list: {}", e);
    }

    Ok(Json(response))
}

/// POST /api/corridors - Create a corridor (invalidates cache)
pub async fn create_corridor_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Json(req): Json<CreateCorridorRequest>,
) -> ApiResult<Json<Corridor>> {
    if req.source_asset_code.is_empty() || req.dest_asset_code.is_empty() {
        return Err(ApiError::BadRequest(
            "Asset codes cannot be empty".to_string(),
        ));
    }
    if req.source_asset_issuer.is_empty() || req.dest_asset_issuer.is_empty() {
        return Err(ApiError::BadRequest(
            "Asset issuers cannot be empty".to_string(),
        ));
    }

    let corridor = db.create_corridor(req).await?;

    // Invalidate corridor caches
    if let Err(e) = cache.delete_pattern(&CacheKey::corridor_pattern()).await {
        warn!("Failed to invalidate corridor cache: {}", e);
    }

    Ok(Json(corridor))
}

/// Cache metrics endpoint
#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub redis_connected: bool,
    pub metrics: crate::cache::metrics::CacheMetricsSummary,
}

pub async fn get_cache_stats(
    State((_, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
) -> Json<CacheStatsResponse> {
    Json(CacheStatsResponse {
        redis_connected: cache.is_redis_connected().await,
        metrics: cache.metrics(),
    })
}

/// Clear cache endpoint (admin only in production)
pub async fn clear_cache(
    State((_, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
) -> ApiResult<Json<serde_json::Value>> {
    cache.clear_all().await?;
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Cache cleared"
    })))
}


/// PUT /api/corridors/:id/metrics-from-transactions - Update corridor metrics (invalidates cache)
#[derive(Debug, Deserialize)]
pub struct UpdateCorridorMetricsFromTxns {
    pub transactions: Vec<CorridorTransactionDto>,
}

#[derive(Debug, Deserialize)]
pub struct CorridorTransactionDto {
    pub successful: bool,
    pub settlement_latency_ms: Option<i32>,
    pub amount_usd: f64,
}

pub async fn update_corridor_metrics_from_transactions_cached(
    State((db, cache)): State<(Arc<Database>, Arc<RedisCache>)>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCorridorMetricsFromTxns>,
) -> ApiResult<Json<Corridor>> {
    use crate::services::analytics::compute_corridor_metrics;
    use crate::services::analytics::CorridorTransaction;

    if db.get_corridor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Corridor with id {} not found",
            id
        )));
    }

    let txs: Vec<CorridorTransaction> = req
        .transactions
        .into_iter()
        .map(|t| CorridorTransaction {
            successful: t.successful,
            settlement_latency_ms: t.settlement_latency_ms,
            amount_usd: t.amount_usd,
        })
        .collect();

    let metrics = compute_corridor_metrics(&txs, None, 1.0);
    let corridor = db.update_corridor_metrics(id, metrics).await?;

    // Invalidate corridor caches
    if let Err(e) = cache.delete_pattern(&CacheKey::corridor_pattern()).await {
        warn!("Failed to invalidate corridor cache: {}", e);
    }
    if let Err(e) = cache.delete_pattern(&CacheKey::dashboard_pattern()).await {
        warn!("Failed to invalidate dashboard cache: {}", e);
    }

    Ok(Json(corridor))
}
