use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::broadcast::{broadcast_anchor_update, broadcast_corridor_update};
use crate::error::{ApiError, ApiResult};
use crate::models::corridor::Corridor;
use crate::models::{CreateAnchorRequest, CreateCorridorRequest, ListCorridorsQuery, ListCorridorsResponse};
use crate::cache::CacheManager;
use crate::database::Database;
use crate::error::ApiResult;
use crate::rpc::StellarRpcClient;
use crate::state::AppState;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HealthChecks,
}

#[derive(Serialize, Debug, Clone)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub cache: ComponentHealth,
    pub rpc: ComponentHealth,
}

#[derive(Serialize, Debug, Clone)]
#[derive(Serialize)]
pub struct ComponentHealth {
    pub healthy: bool,
    pub response_time_ms: Option<u64>,
    pub message: Option<String>,
}

/// Check database health
async fn check_database(db: &Arc<Database>) -> ComponentHealth {
    let start = Instant::now();
    match sqlx::query("SELECT 1").fetch_one(db.pool()).await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Database connection failed: {}", e)),
        },
    }
}

/// Check cache health
async fn check_cache(cache: &Arc<CacheManager>) -> ComponentHealth {
    let start = Instant::now();
    match cache.ping().await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("Cache connection failed: {}", e)),
        },
    }
}

/// Check RPC health
async fn check_rpc(rpc: &Arc<StellarRpcClient>) -> ComponentHealth {
    let start = Instant::now();
    match rpc.check_health().await {
        Ok(_) => ComponentHealth {
            healthy: true,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: None,
        },
        Err(e) => ComponentHealth {
            healthy: false,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            message: Some(format!("RPC connection failed: {}", e)),
        },
    }
}

/// Detailed health check endpoint
pub async fn health_check(State(app_state): State<AppState>) -> Json<HealthStatus> {
    let db_health = check_database(&app_state.db).await;
    let cache_health = check_cache(&app_state.cache).await;
    let rpc_health = check_rpc(&app_state.rpc_client).await;

    let overall_status = if db_health.healthy && cache_health.healthy && rpc_health.healthy {
        "healthy"
    } else if db_health.healthy && cache_health.healthy {
        "degraded"
    } else {
        "unhealthy"
    };

    let start_epoch = app_state.server_start_time.load(Ordering::Relaxed);
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let uptime_seconds = now_epoch.saturating_sub(start_epoch);

    let health_status = HealthStatus {
        status: overall_status.to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        checks: HealthChecks {
            database: db_health,
            cache: cache_health,
            rpc: rpc_health,
        },
    };

    Json(health_status)
}

/// PUT /api/anchors/:id/metrics - Update anchor metrics
#[derive(Debug, Deserialize)]
pub struct UpdateMetricsRequest {
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
}

pub async fn update_anchor_metrics(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMetricsRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    // Verify anchor exists
    if app_state.db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let anchor = app_state
        .db
        .update_anchor_metrics(crate::database::AnchorMetricsUpdate {
            anchor_id: id,
            total_transactions: req.total_transactions,
            successful_transactions: req.successful_transactions,
            failed_transactions: req.failed_transactions,
            avg_settlement_time_ms: req.avg_settlement_time_ms,
            volume_usd: req.volume_usd,
        })
        .await?;

    // Broadcast the anchor update to WebSocket clients
    broadcast_anchor_update(&app_state.ws_state, &anchor);

    Ok(Json(anchor))
}

/// GET /api/anchors/:id/assets - Get assets for an anchor
pub async fn get_anchor_assets(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<crate::models::Asset>>> {
    // Verify anchor exists
    if app_state.db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let assets = app_state.db.get_assets_by_anchor(id).await?;

    Ok(Json(assets))
}

/// POST /api/anchors/:id/assets - Add asset to anchor
#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub asset_code: String,
    pub asset_issuer: String,
}

pub async fn create_anchor_asset(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateAssetRequest>,
) -> ApiResult<Json<crate::models::Asset>> {
    // Verify anchor exists
    if app_state.db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let asset = app_state
        .db
        .create_asset(id, req.asset_code, req.asset_issuer)
        .await?;

    Ok(Json(asset))
}

/// GET /api/anchors/:id - Get anchor by ID
pub async fn get_anchor(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let anchor = app_state.db.get_anchor_by_id(id).await?
        .ok_or_else(|| ApiError::NotFound(format!("Anchor {} not found", id)))?;
    Ok(Json(anchor))
}

/// GET /api/anchors/account/:stellar_account - Get anchor by Stellar account
pub async fn get_anchor_by_account(
    State(app_state): State<AppState>,
    Path(stellar_account): Path<String>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let anchor = app_state.db.get_anchor_by_stellar_account(&stellar_account).await?
        .ok_or_else(|| ApiError::NotFound(format!("Anchor for account {} not found", stellar_account)))?;
    Ok(Json(anchor))
}

/// GET /api/analytics/muxed - Get muxed account analytics
pub async fn get_muxed_analytics(
    State(app_state): State<AppState>,
    Query(params): Query<crate::models::ListCorridorsQuery>,
) -> ApiResult<Json<crate::models::MuxedAccountAnalytics>> {
    let limit = params.limit.unwrap_or(10);
    let analytics = app_state.db.get_muxed_analytics(limit).await?;
    Ok(Json(analytics))
}



/// GET /api/admin/pool-metrics - Return current database pool metrics
pub fn get_pool_metrics(
    State(app_state): State<AppState>,
) -> Json<crate::database::PoolMetrics> {
    Json(app_state.db.pool_metrics())
}

/// GET /metrics - Prometheus metrics endpoint (all registered metrics via global registry)
pub async fn get_prometheus_metrics() -> impl IntoResponse {
    crate::observability::metrics::metrics_handler()
}

#[cfg(test)]
fn render_pool_metrics_prometheus(metrics: &crate::database::PoolMetrics) -> String {
    format!(
        "# HELP stellar_insights_db_pool_size Database pool size\n\
# TYPE stellar_insights_db_pool_size gauge\n\
stellar_insights_db_pool_size {}\n\
# HELP stellar_insights_db_pool_idle Database pool idle connections\n\
# TYPE stellar_insights_db_pool_idle gauge\n\
stellar_insights_db_pool_idle {}\n\
# HELP stellar_insights_db_pool_active Database pool active connections\n\
# TYPE stellar_insights_db_pool_active gauge\n\
stellar_insights_db_pool_active {}\n",
        metrics.size, metrics.idle, metrics.active
    )
}

/// Database pool metrics endpoint
pub async fn pool_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.db.pool_metrics();
    Json(metrics)
/// GET /api/corridors - List all corridors
pub async fn list_corridors(
    State(app_state): State<AppState>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<ListCorridorsResponse>> {
    let corridors = app_state
        .db
        .list_corridors(params.limit, params.offset)
        .await?;
    let total = corridors.len();
    Ok(Json(ListCorridorsResponse { corridors, total }))
}

/// POST /api/anchors - Create a new anchor
pub async fn create_anchor(
    State(app_state): State<AppState>,
    Json(req): Json<CreateAnchorRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let anchor = app_state.db.create_anchor(req).await?;
    Ok(Json(anchor))
}

/// POST /api/corridors - Create a new corridor
pub async fn create_corridor(
    State(app_state): State<AppState>,
    Json(req): Json<CreateCorridorRequest>,
) -> ApiResult<Json<Corridor>> {
    let corridor = app_state.db.create_corridor(req).await?;
    // Broadcast the new corridor to WebSocket clients
    broadcast_corridor_update(&app_state.ws_state, &corridor);
    Ok(Json(corridor))
}

/// PUT /api/corridors/:id/metrics-from-transactions - Placeholder for updating metrics from batch transactions
pub async fn update_corridor_metrics_from_transactions(
    State(_app_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Implementation for processing transaction batch logic goes here
    Ok(Json(serde_json::json!({ "status": "not_implemented" })))
}

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_pool_metrics_prometheus() {
        let metrics = crate::database::PoolMetrics::new(12, 3, 9);
        let rendered = render_pool_metrics_prometheus(&metrics);

        assert!(rendered.contains("stellar_insights_db_pool_size 12"));
        assert!(rendered.contains("stellar_insights_db_pool_idle 3"));
        assert!(rendered.contains("stellar_insights_db_pool_active 9"));
        assert!(rendered.contains("# TYPE stellar_insights_db_pool_size gauge"));
    }
}
