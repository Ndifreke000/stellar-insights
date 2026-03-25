use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::broadcast::broadcast_anchor_update;
use crate::error::{ApiError, ApiResult};
use crate::models::{AnchorDetailResponse, CreateAnchorRequest};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListAnchorsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

const fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct ListAnchorsResponse {
    pub anchors: Vec<crate::models::Anchor>,
    pub total: usize,
}

/// GET /api/analytics/muxed - Muxed account usage analytics
#[derive(Debug, Deserialize)]
pub struct MuxedAnalyticsQuery {
    #[serde(default = "default_muxed_limit")]
    pub limit: i64,
}

const fn default_muxed_limit() -> i64 {
    20
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
            format!("Anchor with id {id} not found"),
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
                format!("Anchor with stellar account {account_lookup} not found"),
                details,
            )
        })?;

    Ok(Json(anchor))
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
            format!("Anchor with id {id} not found"),
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
            format!("Anchor with id {id} not found"),
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
            format!("Anchor with id {id} not found"),
            details,
        ));
    }

    let asset = app_state
        .db
        .create_asset(id, req.asset_code, req.asset_issuer)
        .await?;

    Ok(Json(asset))
}

