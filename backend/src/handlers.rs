use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::broadcast::{broadcast_anchor_update, broadcast_corridor_update};
use crate::error::{ApiError, ApiResult};
use crate::models::corridor::Corridor;
use crate::models::{AnchorDetailResponse, CreateAnchorRequest, CreateCorridorRequest};
use crate::services::analytics::{compute_corridor_metrics, CorridorTransaction};
use crate::state::AppState;


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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct ListCorridorsResponse {
    pub corridors: Vec<Corridor>,
    pub total: usize,
}

/// GET /api/anchors - List all anchors with their metrics
pub async fn list_anchors(
    State(app_state): State<AppState>,
    Query(params): Query<ListAnchorsQuery>,
) -> ApiResult<Json<ListAnchorsResponse>> {
    let anchors = app_state
        .db
        .list_anchors(params.limit, params.offset)
        .await?;
    let total = anchors.len();

    Ok(Json(ListAnchorsResponse { anchors, total }))
}

/// GET /api/anchors/:id - Get detailed anchor information
pub async fn get_anchor(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AnchorDetailResponse>> {
    let anchor_detail = app_state.db.get_anchor_detail(id).await?.ok_or_else(|| {
        let mut details = HashMap::new();
        details.insert("anchor_id".to_string(), serde_json::json!(id.to_string()));
        ApiError::not_found_with_details(
            "ANCHOR_NOT_FOUND",
            format!("Anchor with id {} not found", id),
            details,
        )
    })?;

    Ok(Json(anchor_detail))
}

/// GET /api/anchors/account/:stellar_account - Get anchor by Stellar account (G- or M-address)
pub async fn get_anchor_by_account(
    State(app_state): State<AppState>,
    Path(stellar_account): Path<String>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let account_lookup = stellar_account.trim();
    // If M-address, resolve to base account for anchor lookup (anchors are keyed by G-address)
    let lookup_key = if crate::muxed::is_muxed_address(account_lookup) {
        crate::muxed::parse_muxed_address(account_lookup)
            .and_then(|i| i.base_account)
            .unwrap_or_else(|| account_lookup.to_string())
    } else {
        account_lookup.to_string()
    };
    let anchor = app_state
        .db
        .get_anchor_by_stellar_account(&lookup_key)
        .await?
        .ok_or_else(|| {
            let mut details = HashMap::new();
            details.insert(
                "stellar_account".to_string(),
                serde_json::json!(account_lookup),
            );
            ApiError::not_found_with_details(
                "ANCHOR_NOT_FOUND",
                format!("Anchor with stellar account {} not found", account_lookup),
                details,
            )
        })?;

    Ok(Json(anchor))
}

/// GET /api/analytics/muxed - Muxed account usage analytics
#[derive(Debug, Deserialize)]
pub struct MuxedAnalyticsQuery {
    #[serde(default = "default_muxed_limit")]
    pub limit: i64,
}
fn default_muxed_limit() -> i64 {
    20
}

pub async fn get_muxed_analytics(
    State(app_state): State<AppState>,
    Query(params): Query<MuxedAnalyticsQuery>,
) -> ApiResult<Json<crate::models::MuxedAccountAnalytics>> {
    let limit = params.limit.clamp(1, 100);
    let analytics = app_state.db.get_muxed_analytics(limit).await?;
    Ok(Json(analytics))
}

/// POST /api/anchors - Create a new anchor
pub async fn create_anchor(
    State(app_state): State<AppState>,
    Json(req): Json<CreateAnchorRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    if req.name.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_INPUT",
            "Name cannot be empty",
        ));
    }

    if req.stellar_account.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_INPUT",
            "Stellar account cannot be empty",
        ));
    }

    let anchor = app_state.db.create_anchor(req).await?;

    // Broadcast the new anchor to WebSocket clients
    broadcast_anchor_update(&app_state.ws_state, &anchor);

    Ok(Json(anchor))
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
        let mut details = HashMap::new();
        details.insert("anchor_id".to_string(), serde_json::json!(id.to_string()));
        return Err(ApiError::not_found_with_details(
            "ANCHOR_NOT_FOUND",
            format!("Anchor with id {} not found", id),
            details,
        ));
    }

    let anchor = app_state
        .db
        .update_anchor_metrics(
            id,
            req.total_transactions,
            req.successful_transactions,
            req.failed_transactions,
            req.avg_settlement_time_ms,
            req.volume_usd,
        )
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
        let mut details = HashMap::new();
        details.insert("anchor_id".to_string(), serde_json::json!(id.to_string()));
        return Err(ApiError::not_found_with_details(
            "ANCHOR_NOT_FOUND",
            format!("Anchor with id {} not found", id),
            details,
        ));
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
        let mut details = HashMap::new();
        details.insert("anchor_id".to_string(), serde_json::json!(id.to_string()));
        return Err(ApiError::not_found_with_details(
            "ANCHOR_NOT_FOUND",
            format!("Anchor with id {} not found", id),
            details,
        ));
    }

    let asset = app_state
        .db
        .create_asset(id, req.asset_code, req.asset_issuer)
        .await?;

    Ok(Json(asset))
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "stellar-insights-backend",
        "version": env!("CARGO_PKG_VERSION"),
        "api": {
            "current_version": "v1",
            "supported_versions": ["v1"],
            "status": "active"
        }
    }))
}


/// Database pool metrics endpoint
pub async fn pool_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.db.pool_metrics();
    Json(metrics)
}

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

/// POST /api/corridors - Create a new corridor
pub async fn create_corridor(
    State(app_state): State<AppState>,
    Json(req): Json<CreateCorridorRequest>,
) -> ApiResult<Json<Corridor>> {
    if req.source_asset_code.is_empty() || req.dest_asset_code.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_INPUT",
            "Asset codes cannot be empty",
        ));
    }
    if req.source_asset_issuer.is_empty() || req.dest_asset_issuer.is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_INPUT",
            "Asset issuers cannot be empty",
        ));
    }
    let corridor = app_state.db.create_corridor(req).await?;

    // Broadcast the new corridor to WebSocket clients
    broadcast_corridor_update(&app_state.ws_state, &corridor);

    Ok(Json(corridor))
}

/// PUT /api/corridors/:id/metrics-from-transactions - Compute metrics from transactions and persist
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

pub async fn update_corridor_metrics_from_transactions(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCorridorMetricsFromTxns>,
) -> ApiResult<Json<Corridor>> {
    if app_state.db.get_corridor_by_id(id).await?.is_none() {
        let mut details = HashMap::new();
        details.insert("corridor_id".to_string(), serde_json::json!(id.to_string()));
        return Err(ApiError::not_found_with_details(
            "CORRIDOR_NOT_FOUND",
            format!("Corridor with id {} not found", id),
            details,
        ));
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
    let corridor = app_state.db.update_corridor_metrics(id, metrics).await?;

    // Broadcast the corridor update to WebSocket clients
    broadcast_corridor_update(&app_state.ws_state, &corridor);

    Ok(Json(corridor))
}

pub async fn ingestion_status(
    State(app_state): State<AppState>,
) -> ApiResult<Json<crate::ingestion::IngestionStatus>> {
    let status = app_state.ingestion.get_ingestion_status().await?;
    Ok(Json(status))
}
