use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// use anyhow::Context;

use crate::broadcast::broadcast_anchor_update;
use crate::cache::helpers::cached_query;
use crate::cache::keys;
use crate::cache::CacheManager;
use crate::database::Database;
use crate::error::{ApiError, ApiResult};
use crate::models::{AnchorDetailResponse, CreateAnchorRequest};
use crate::rpc::circuit_breaker::rpc_circuit_breaker;
use crate::rpc::circuit_breaker::{rpc_circuit_breaker, CircuitBreaker, CircuitBreakerConfig};
use crate::rpc::error::{with_retry, RetryConfig, RpcError};
use crate::rpc::StellarRpcClient;
use crate::services::price_feed::PriceFeedClient;
use crate::state::AppState;
use tracing::warn;
use crate::rpc::error::{RetryConfig, with_retry, RpcError};
use tracing::warn;
use tracing::{error, info, warn};


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

/// GET /api/analytics/muxed - Muxed account usage analytics
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

/// GET /api/anchors/:id - Get detailed anchor information
#[utoipa::path(
    get,
    path = "/api/anchors/{id}",
    params(
        ("id" = String, Path, description = "Anchor UUID")
    ),
    responses(
        (status = 200, description = "Anchor details retrieved successfully"),
        (status = 404, description = "Anchor not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Anchors"
)]
#[tracing::instrument(skip(app_state), fields(anchor_id = %id))]
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
#[utoipa::path(
    get,
    path = "/api/anchors/account/{stellar_account}",
    params(
        ("stellar_account" = String, Path, description = "Anchor Stellar account address (G... or M...)")
    ),
    responses(
        (status = 200, description = "Anchor retrieved successfully"),
        (status = 404, description = "Anchor not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Anchors"
)]
#[tracing::instrument(skip(app_state), fields(stellar_account = %stellar_account))]
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

#[utoipa::path(
    get,
    path = "/api/analytics/muxed",
    params(MuxedAnalyticsQuery),
    responses(
        (status = 200, description = "Muxed account analytics retrieved successfully"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Anchors"
)]
#[tracing::instrument(skip(app_state), fields(limit = params.limit))]
pub async fn get_muxed_analytics(
    State(app_state): State<AppState>,
    Query(params): Query<MuxedAnalyticsQuery>,
) -> ApiResult<Json<crate::models::MuxedAccountAnalytics>> {
    let limit = params.limit.clamp(1, 100);
    let analytics = app_state.db.get_muxed_analytics(limit).await?;
    Ok(Json(analytics))
}

/// POST /api/anchors - Create a new anchor
#[tracing::instrument(skip(app_state, req), fields(anchor_name = %req.name))]
pub async fn create_anchor(
    State(app_state): State<AppState>,
    Json(req): Json<CreateAnchorRequest>,
) -> ApiResult<Json<crate::models::Anchor>> {
    // Struct-level field validation (lengths)
    crate::validation::validate_request(&req)?;

    // Business logic: stellar account must start with 'G'
    crate::validation::validate_stellar_account(&req.stellar_account)?;

    let anchor = app_state.db.create_anchor(req).await?;

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

#[tracing::instrument(skip(app_state, req), fields(anchor_id = %id))]
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
#[tracing::instrument(skip(app_state), fields(anchor_id = %id))]
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
#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateAssetRequest {
    #[validate(length(
        min = 1,
        max = 12,
        message = "Asset code must be between 1 and 12 characters"
    ))]
    pub asset_code: String,

    #[validate(length(
        min = 1,
        max = 56,
        message = "Asset issuer must be at most 56 characters"
    ))]
    pub asset_issuer: String,
}

#[tracing::instrument(skip(app_state, req), fields(anchor_id = %id, asset_code = %req.asset_code))]
pub async fn create_anchor_asset(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateAssetRequest>,
) -> ApiResult<Json<crate::models::Asset>> {
    // Field validation
    crate::validation::validate_request(&req)?;

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

use crate::cache::helpers::cached_query;
use crate::cache::keys;
use crate::database::Database;
use crate::rpc::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    error::{RpcError},
    circuit_breaker::{rpc_circuit_breaker, CircuitBreaker},
    error::{with_retry, RetryConfig, RpcError},
    StellarRpcClient,
};
use crate::services::price_feed::PriceFeedClient;
use std::time::Duration;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListAnchorsQuery {
    /// Maximum number of results to return (default: 50)
    #[serde(default = "default_limit")]
    #[param(example = 50)]
    pub limit: i64,
    /// Pagination offset (default: 0)
    #[serde(default)]
    #[param(example = 0)]
    pub offset: i64,
}

const fn default_limit() -> i64 {
    50
}

pub(crate) fn rpc_circuit_breaker() -> Arc<CircuitBreaker> {
    static CIRCUIT_BREAKER: OnceLock<Arc<CircuitBreaker>> = OnceLock::new();
    CIRCUIT_BREAKER
        .get_or_init(|| {
            Arc::new(CircuitBreaker::new(
                CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 2,
                    timeout_duration: Duration::from_secs(30),
                    half_open_max_calls: 3,
                },
                "horizon",
            ))
        })
        .clone()
}

pub fn rpc_circuit_breaker_instance() -> Arc<CircuitBreaker> {
    rpc_circuit_breaker()
}

// Add retry helper
async fn with_retry<F, Fut, T>(
    mut operation: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> Result<T, RpcError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, RpcError>>,
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
                    backoff *= 2;  // Exponential backoff
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}
// Shared circuit breaker is now managed in crate::rpc::circuit_breaker

pub async fn get_anchor_metrics_with_rpc(
    anchor_id: Uuid,
    rpc_client: Arc<StellarRpcClient>,
) -> anyhow::Result<AnchorMetrics> {
    let circuit_breaker = rpc_circuit_breaker();
    let metrics: AnchorMetrics = with_retry(
        || async {
            rpc_client
                .fetch_anchor_metrics(anchor_id)
                .map_err(|e| RpcError::categorize(&e.to_string()))
        },
        RetryConfig::default(),
        circuit_breaker,
    )
    .await
    .map_err(|e| match e {
        RpcError::CircuitBreakerOpen => {
            anyhow::anyhow!("Circuit breaker open - RPC service unavailable")
        }
        other => anyhow::anyhow!(other.to_string()),
    })?;
                .await
                .map_err(|e| RpcError::categorize(&e.to_string()))
        },
        3,
        Duration::from_secs(1),
    )
    .await;

    let metrics = match result {
        Ok(metrics) => metrics,
        Err(RpcError::CircuitBreakerOpen) => {
            return Err(anyhow::anyhow!("Circuit breaker open - RPC service unavailable"));
        }
        Err(err) => return Err(anyhow::anyhow!(err.to_string())),
    };
                .context("RPC call failed")
        })
        .await
        .map_err(|e| match e {
            failsafe::Error::Rejected => {
                anyhow::anyhow!("Circuit breaker open - RPC service unavailable")
            }
            failsafe::Error::Inner(err) => err,
        })?;

    Ok(metrics)
}

pub async fn get_anchor_metrics_with_fallback(
    anchor_id: Uuid,
    rpc_client: Arc<StellarRpcClient>,
    cache: Arc<CacheManager>,
) -> anyhow::Result<AnchorMetrics> {
    match get_anchor_metrics_with_rpc(anchor_id, rpc_client).await {
        Ok(metrics) => Ok(metrics),
        Err(e) if e.to_string().contains("Circuit breaker open") => {
            warn!("Circuit breaker open, using cached data");
            cache
                .get::<AnchorMetrics>(&format!("anchor_metrics:{}", anchor_id))
                .await?
                .ok_or_else(|| anyhow::anyhow!("No cached data available"))
        }
        Err(e) => Err(e),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AnchorMetricsResponse {
    /// Unique identifier for the anchor
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,
    /// Name of the anchor
    #[schema(example = "MoneyGram Access")]
    pub name: String,
    /// Stellar account address
    #[schema(example = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")]
    pub stellar_account: String,
    /// Reliability score (0-100)
    #[schema(example = 99.5)]
    pub reliability_score: f64,
    /// Number of assets supported
    #[schema(example = 5)]
    pub asset_coverage: usize,
    /// Failure rate percentage
    #[schema(example = 0.5)]
    pub failure_rate: f64,
    /// Total number of transactions
    #[schema(example = 10000)]
    pub total_transactions: i64,
    /// Number of successful transactions
    #[schema(example = 9950)]
    pub successful_transactions: i64,
    /// Number of failed transactions
    #[schema(example = 50)]
    pub failed_transactions: i64,
    /// Health status (green, yellow, red)
    #[schema(example = "green")]
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AnchorsResponse {
    /// List of anchors with their metrics
    pub anchors: Vec<AnchorMetricsResponse>,
    /// Total number of anchors
    #[schema(example = 25)]
    pub total: usize,
}

/// List all anchors with key metrics
///
/// Returns a paginated list of all anchors with their performance metrics.
/// Data is cached for improved performance.
///
/// **DATA SOURCE: RPC + Database**
/// - Anchor metadata (name, account) from database
/// - Transaction metrics calculated from RPC payment data
#[utoipa::path(
    get,
    path = "/api/anchors",
    params(ListAnchorsQuery),
    responses(
        (status = 200, description = "List of anchors retrieved successfully", body = AnchorsResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Anchors"
)]
#[tracing::instrument(skip(db, cache, rpc_client, _price_feed, params, headers), fields(limit = params.limit, offset = params.offset))]
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
            // Get anchor metadata from database (names, accounts, etc.)
            let anchors: Vec<crate::models::Anchor> =
                db.list_anchors(params.limit, params.offset).await?;

            if anchors.is_empty() {
                return Ok(AnchorsResponse {
                    anchors: vec![],
                    total: 0,
                });
            }

            // OPTIMIZATION: Batch fetch all assets for these anchors (1 query instead of N)
            let anchor_ids: Vec<uuid::Uuid> = anchors
                .iter()
                .map(|a| uuid::Uuid::parse_str(&a.id).unwrap_or_else(|_| uuid::Uuid::nil()))
                .collect();

            let asset_map: HashMap<String, Vec<crate::models::Asset>> = db
                .get_assets_by_anchors(&anchor_ids)
                .await
                .unwrap_or_default();

            let circuit_breaker = rpc_circuit_breaker();
            let mut anchor_responses = Vec::new();

            // Process anchors with pre-fetched data
            for anchor in anchors {
                // Get pre-fetched assets (no additional query needed)
                let assets = asset_map.get(&anchor.id).cloned().unwrap_or_default();

                // **RPC DATA**: Fetch real-time payment data for this anchor with pagination
                // Wrapped in circuit breaker as requested in Issue #671
                let payments: Vec<crate::rpc::Payment> = with_retry(
                    || async {
                        rpc_client
                            .fetch_all_account_payments(&anchor.stellar_account, Some(500))
                            .await
                            .map_err(|e| RpcError::categorize(&e.to_string()))
                    },
                    RetryConfig::default(),
                    circuit_breaker.clone(),
                )
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "Failed to fetch payments for anchor {}: {}",
                        anchor.stellar_account,
                        e
                    );
                    vec![]
                });

                // Calculate metrics from RPC payment data
                let (total_transactions, successful_transactions, failed_transactions) =
                    if payments.is_empty() {
                        (
                            anchor.total_transactions,
                            anchor.successful_transactions,
                            anchor.failed_transactions,
                        )
                    } else {
                        let total = payments.len() as i64;
                        // In Stellar, if a payment appears in the ledger, it was successful
                        // Failed payments don't appear in the payment stream
                        let successful = total;
                        let failed = 0;
                        (total, successful, failed)
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

                let anchor_response = AnchorMetricsResponse {
                    id: anchor.id.to_string(),
                    name: anchor.name,
                    stellar_account: anchor.stellar_account,
                    reliability_score,
                    asset_coverage: assets.len(),
                    failure_rate,
                    total_transactions,
                    successful_transactions,
                    failed_transactions,
                    status,
                };

                anchor_responses.push(anchor_response);
            }

            let total = anchor_responses.len();

            Ok(AnchorsResponse {
                anchors: anchor_responses,
                total,
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
    use crate::cache::{CacheConfig, CacheManager};
    use crate::rpc::circuit_breaker::rpc_circuit_breaker;
    use crate::rpc::StellarRpcClient;
    use failsafe::CircuitBreaker as _;

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(false));
        let anchor_id = Uuid::new_v4();

        let circuit_breaker = rpc_circuit_breaker();
        while matches!(
            circuit_breaker.call(|| Err::<(), anyhow::Error>(anyhow::anyhow!("forced failure"))),
            Err(failsafe::Error::Inner(_))
        ) {}

        for _ in 0..5 {
            let _ = get_anchor_metrics_with_rpc(anchor_id, rpc_client.clone()).await;
        }

        let result = get_anchor_metrics_with_rpc(anchor_id, rpc_client.clone()).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circuit breaker open"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_fallback() {
        let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(false));
        let cache = Arc::new(CacheManager::new_in_memory_for_tests(CacheConfig::default()));
        let anchor_id = Uuid::new_v4();

        // Pre-fill cache
        let metrics = AnchorMetrics {
            anchor_id,
            total_payments: 100,
            successful_payments: 95,
            failed_payments: 5,
            total_volume: 1000.0,
        };
        cache
            .set(&format!("anchor_metrics:{}", anchor_id), &metrics, 60)
            .await
            .unwrap();

        // Trigger circuit breaker
        for _ in 0..6 {
            let _ = get_anchor_metrics_with_rpc(anchor_id, rpc_client.clone()).await;
        }

        // Verify fallback works
        let result = get_anchor_metrics_with_fallback(anchor_id, rpc_client, cache).await;
        assert!(result.is_ok());
        let returned_metrics = result.unwrap();
        assert_eq!(returned_metrics.total_payments, 100);
    }

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
        assert_eq!(response.asset_coverage, 3);
    }
}
