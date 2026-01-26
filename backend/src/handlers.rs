use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::database::Database;
use crate::models::corridor::Corridor;
use crate::models::{AnchorDetailResponse, CreateAnchorRequest, CreateCorridorRequest};
use crate::services::analytics::{compute_corridor_metrics, CorridorTransaction};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

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
    State(db): State<Arc<Database>>,
    Query(params): Query<ListAnchorsQuery>,
) -> ApiResult<Json<ListAnchorsResponse>> {
    let anchors = db.list_anchors(params.limit, params.offset).await?;
    let total = anchors.len();

    Ok(Json(ListAnchorsResponse { anchors, total }))
}

/// GET /api/anchors/:id - Get detailed anchor information
pub async fn get_anchor(
    State(db): State<Arc<Database>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AnchorDetailResponse>> {
    let anchor_detail = db
        .get_anchor_detail(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Anchor with id {} not found", id)))?;

    Ok(Json(anchor_detail))
}

/// GET /api/anchors/account/:stellar_account - Get anchor by Stellar account
pub async fn get_anchor_by_account(
    State(db): State<Arc<Database>>,
    Path(stellar_account): Path<String>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let anchor = db
        .get_anchor_by_stellar_account(&stellar_account)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Anchor with stellar account {} not found",
                stellar_account
            ))
        })?;

    Ok(Json(anchor))
}

/// POST /api/anchors - Create a new anchor
pub async fn create_anchor(
    State(db): State<Arc<Database>>,
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
    State(db): State<Arc<Database>>,
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

    Ok(Json(anchor))
}

/// GET /api/anchors/:id/assets - Get assets issued by anchor
pub async fn get_anchor_assets(
    State(db): State<Arc<Database>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<crate::models::Asset>>> {
    // Verify anchor exists
    if db.get_anchor_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Anchor with id {} not found",
            id
        )));
    }

    let assets = db.get_assets_by_anchor(id).await?;

    Ok(Json(assets))
}

/// POST /api/anchors/:id/assets - Add asset to anchor
#[derive(Debug, Deserialize)]
pub struct CreateAssetRequest {
    pub asset_code: String,
    pub asset_issuer: String,
}

pub async fn create_anchor_asset(
    State(db): State<Arc<Database>>,
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

    Ok(Json(asset))
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "stellar-insights-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// GET /api/corridors - List corridors with metrics
pub async fn list_corridors(
    State(db): State<Arc<Database>>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<ListCorridorsResponse>> {
    let corridors = db.list_corridors(params.limit, params.offset).await?;
    let total = corridors.len();
    Ok(Json(ListCorridorsResponse { corridors, total }))
}

/// POST /api/corridors - Create a corridor (idempotent on asset pair)
pub async fn create_corridor(
    State(db): State<Arc<Database>>,
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
    State(db): State<Arc<Database>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCorridorMetricsFromTxns>,
) -> ApiResult<Json<Corridor>> {
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
    Ok(Json(corridor))
}
