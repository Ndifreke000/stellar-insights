use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use anyhow::Context; // ✅ Added Context trait into scope

use crate::broadcast::broadcast_anchor_update;
use crate::error::{ApiError, ApiResult};
use crate::models::{AnchorDetailResponse, CreateAnchorRequest};
use crate::state::AppState;
use crate::cache::helpers::cached_query;
use crate::cache::{keys, CacheManager};
use crate::database::Database;
use crate::rpc::StellarRpcClient;
use crate::services::price_feed::PriceFeedClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnchorMetrics {
    pub anchor_id: Uuid,
    pub total_payments: u64,
    pub successful_payments: u64,
    pub failed_payments: u64,
    pub total_volume: f64,
}

#[derive(Debug, Serialize)]
pub struct ListAnchorsResponse {
    pub anchors: Vec<crate::models::Anchor>,
    pub total: usize,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct MuxedAnalyticsQuery {
    #[serde(default = "default_muxed_limit")]
    #[param(example = 20, minimum = 1, maximum = 100)]
    pub limit: i64,
}

const fn default_muxed_limit() -> i64 {
    20
}

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

pub async fn get_anchor_by_account(
    State(app_state): State<AppState>,
    Path(stellar_account): Path<String>,
) -> ApiResult<Json<crate::models::Anchor>> {
    let account_lookup = stellar_account.trim();
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

pub async fn create_anchor(
    State(app_state): State<AppState>,
    Json(req): Json<CreateAnchorRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    if req.name.is_empty() {
        return Err(ApiError::bad_request("INVALID_INPUT", "Name cannot be empty"));
    }

    if req.stellar_account.is_empty() {
        return Err(ApiError::bad_request("INVALID_INPUT", "Stellar account cannot be empty"));
    }

    let anchor = app_state.db.create_anchor(req).await?;
    broadcast_anchor_update(&app_state.ws_state, &anchor);

    Ok(Json(anchor))
}

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
        .update_anchor_metrics(crate::database::AnchorMetricsUpdate {
            anchor_id: id,
            total_transactions: req.total_transactions,
            successful_transactions: req.successful_transactions,
            failed_transactions: req.failed_transactions,
            avg_settlement_time_ms: req.avg_settlement_time_ms,
            volume_usd: req.volume_usd,
        })
        .await?;

    broadcast_anchor_update(&app_state.ws_state, &anchor);

    Ok(Json(anchor))
}

pub async fn get_anchor_assets(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<crate::models::Asset>>> {
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

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListAnchorsQuery {
    #[serde(default = "default_limit")]
    #[param(example = 50)]
    pub limit: i64,
    #[serde(default)]
    #[param(example = 0)]
    pub offset: i64,
}

const fn default_limit() -> i64 {
    50
}

async fn with_retry<F, Fut, T>(
    mut operation: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut backoff = initial_backoff;
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Retry failed without errors")))
}

pub async fn get_anchor_metrics_with_rpc(
    anchor_id: Uuid,
    rpc_client: Arc<StellarRpcClient>,
) -> anyhow::Result<AnchorMetrics> {
    with_retry(
        || async {
            rpc_client
                .fetch_anchor_metrics(anchor_id)
                .await
                .map_err(anyhow::Error::from) // ✅ Fixed Error Type!
        },
        4,
        Duration::from_millis(100),
    )
    .await
    .context("Failed to fetch anchor metrics from RPC")
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AnchorMetricsResponse {
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,
    #[schema(example = "MoneyGram Access")]
    pub name: String,
    #[schema(example = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")]
    pub stellar_account: String,
    pub reliability_score: f64,
    pub asset_coverage: usize,
    pub failure_rate: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AnchorsResponse {
    pub anchors: Vec<AnchorMetricsResponse>,
    pub total: usize,
}

pub async fn get_anchors(
    State((db, cache, rpc_client, _price_feed)): State<(
        Arc<Database>,
        Arc<CacheManager>,
        Arc<StellarRpcClient>,
        Arc<PriceFeedClient>,
    )>,
    Query(params): Query<ListAnchorsQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let cache_key = keys::anchor_list(params.limit, params.offset);

    let response = cached_query(
        &cache,
        &cache_key,
        cache.config.get_ttl("anchor"),
        || async {
            let anchors = db.list_anchors(params.limit, params.offset).await?;

            if anchors.is_empty() {
                return Ok(AnchorsResponse {
                    anchors: vec![],
                    total: 0,
                });
            }

            let anchor_ids: Vec<uuid::Uuid> = anchors
                .iter()
                .map(|a| uuid::Uuid::parse_str(&a.id).unwrap_or_else(|_| uuid::Uuid::nil()))
                .collect();

            let asset_map = db
                .get_assets_by_anchors(&anchor_ids)
                .await
                .unwrap_or_default();

            let mut anchor_responses = Vec::new();

            for anchor in &anchors { // ✅ Added borrowing `&` to fix moving error
                let assets = asset_map.get(&anchor.id).cloned().unwrap_or_default();

                let payments = match rpc_client
                    .fetch_all_account_payments(&anchor.stellar_account, Some(500))
                    .await
                {
                    Ok(p) => p,
                    Err(_) => vec![]
                };

                let (total_transactions, successful_transactions, failed_transactions) =
                    if payments.is_empty() {
                        (
                            anchor.total_transactions,
                            anchor.successful_transactions,
                            anchor.failed_transactions,
                        )
                    } else {
                        let total = payments.len() as i64;
                        (total, total, 0)
                    };

                let failure_rate = if total_transactions > 0 {
                    (failed_transactions as f64 / total_transactions as f64) * 100.0
                } else {
                    0.0
                };

                let reliability_score = if total_transactions > 0 {
                    (successful_transactions as f64 / total_transactions as f64) * 100.0
                } else {
                    anchor.reliability_score
                };

                let status = if reliability_score >= 99.0 {
                    "green".to_string()
                } else if reliability_score >= 95.0 {
                    "yellow".to_string()
                } else {
                    "red".to_string()
                };

                anchor_responses.push(AnchorMetricsResponse {
                    id: anchor.id.to_string(),
                    name: anchor.name.clone(),
                    stellar_account: anchor.stellar_account.clone(),
                    reliability_score,
                    asset_coverage: assets.len(),
                    failure_rate,
                    total_transactions,
                    successful_transactions,
                    failed_transactions,
                    status,
                });
            }

            Ok(AnchorsResponse {
                anchors: anchor_responses,
                total: anchors.len(), // ✅ Works perfectly now!
            })
        },
    )
    .await?;

    let ttl = cache.config.get_ttl("anchor");
    let response = crate::http_cache::cached_json_response(&headers, &cache_key, &response, ttl)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key = keys::anchor_list(50, 0);
        assert_eq!(key, "anchor:list:50:0");
    }

    #[test]
    fn test_anchor_metrics_response_creation() {
        let response = AnchorMetricsResponse {
            id: "123".to_string(),
            name: "Test Anchor".to_string(),
            stellar_account: "GA123".to_string(),
            reliability_score: 95.5,
            asset_coverage: 3,
            failure_rate: 5.0,
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            status: "green".to_string(),
        };

        assert_eq!(response.name, "Test Anchor");
        assert_eq!(response.reliability_score, 95.5);
    }
}