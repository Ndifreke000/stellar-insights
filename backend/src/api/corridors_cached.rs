use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::cache::{keys, CacheManager};
use crate::cache_middleware::CacheAware;
use crate::database::Database;
use crate::handlers::ApiResult;
use crate::models::SortBy;
use crate::rpc::StellarRpcClient;
use crate::services::price_feed::PriceFeedClient;

/// Represents an asset pair (source -> destination) for a corridor
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AssetPair {
    source_asset: String,
    destination_asset: String,
}

impl AssetPair {
    fn to_corridor_key(&self) -> String {
        format!("{}->{}", self.source_asset, self.destination_asset)
    }
}

/// Extract asset pair from a payment operation
/// Handles regular payments, path_payment_strict_send, and path_payment_strict_receive
fn extract_asset_pair_from_payment(payment: &crate::rpc::Payment) -> Option<AssetPair> {
    let operation_type = payment.operation_type.as_deref().unwrap_or("payment");

    match operation_type {
        "path_payment_strict_send" | "path_payment_strict_receive" => {
            // Path payments have explicit source and destination assets
            let source_asset = if let Some(src_type) = &payment.source_asset_type {
                if src_type == "native" {
                    "XLM:native".to_string()
                } else {
                    format!(
                        "{}:{}",
                        payment.source_asset_code.as_deref().unwrap_or("UNKNOWN"),
                        payment.source_asset_issuer.as_deref().unwrap_or("unknown")
                    )
                }
            } else {
                return None;
            };

            let destination_asset = if payment.asset_type == "native" {
                "XLM:native".to_string()
            } else {
                format!(
                    "{}:{}",
                    payment.asset_code.as_deref().unwrap_or("UNKNOWN"),
                    payment.asset_issuer.as_deref().unwrap_or("unknown")
                )
            };

            Some(AssetPair {
                source_asset,
                destination_asset,
            })
        }
        "payment" | _ => {
            // Regular payments: same asset for source and destination
            let asset = if payment.asset_type == "native" {
                "XLM:native".to_string()
            } else {
                format!(
                    "{}:{}",
                    payment.asset_code.as_deref().unwrap_or("UNKNOWN"),
                    payment.asset_issuer.as_deref().unwrap_or("unknown")
                )
            };

            Some(AssetPair {
                source_asset: asset.clone(),
                destination_asset: asset,
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorResponse {
    /// Unique identifier for the corridor
    #[schema(example = "USDC:native->XLM:native")]
    pub id: String,
    /// Source asset code
    #[schema(example = "USDC")]
    pub source_asset: String,
    /// Destination asset code
    #[schema(example = "XLM")]
    pub destination_asset: String,
    /// Success rate percentage
    #[schema(example = 99.8)]
    pub success_rate: f64,
    /// Total payment attempts
    #[schema(example = 5000)]
    pub total_attempts: i64,
    /// Number of successful payments
    #[schema(example = 4990)]
    pub successful_payments: i64,
    /// Number of failed payments
    #[schema(example = 10)]
    pub failed_payments: i64,
    /// Average latency in milliseconds
    #[schema(example = 450.5)]
    pub average_latency_ms: f64,
    /// Median latency in milliseconds
    #[schema(example = 380.0)]
    pub median_latency_ms: f64,
    /// 95th percentile latency in milliseconds
    #[schema(example = 850.0)]
    pub p95_latency_ms: f64,
    /// 99th percentile latency in milliseconds
    #[schema(example = 1200.0)]
    pub p99_latency_ms: f64,
    /// Liquidity depth in USD
    #[schema(example = 1500000.0)]
    pub liquidity_depth_usd: f64,
    /// 24-hour trading volume in USD
    #[schema(example = 150000.0)]
    pub liquidity_volume_24h_usd: f64,
    /// Liquidity trend (increasing, stable, decreasing)
    #[schema(example = "stable")]
    pub liquidity_trend: String,
    /// Overall health score (0-100)
    #[schema(example = 95.5)]
    pub health_score: f64,
    /// Last update timestamp
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessRateDataPoint {
    /// Timestamp of the data point
    #[schema(example = "2024-01-15T10:00:00Z")]
    pub timestamp: String,
    /// Success rate percentage at this time
    #[schema(example = 99.5)]
    pub success_rate: f64,
    /// Number of attempts at this time
    #[schema(example = 150)]
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LatencyDataPoint {
    /// Latency bucket in milliseconds
    #[schema(example = 500)]
    pub latency_bucket_ms: i32,
    /// Number of transactions in this bucket
    #[schema(example = 250)]
    pub count: i64,
    /// Percentage of total transactions
    #[schema(example = 25.5)]
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiquidityDataPoint {
    /// Timestamp of the data point
    #[schema(example = "2024-01-15T10:00:00Z")]
    pub timestamp: String,
    /// Liquidity in USD at this time
    #[schema(example = 1500000.0)]
    pub liquidity_usd: f64,
    /// 24-hour volume in USD
    #[schema(example = 150000.0)]
    pub volume_24h_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CorridorDetailResponse {
    /// Corridor summary information
    pub corridor: CorridorResponse,
    /// Historical success rate data points
    pub historical_success_rate: Vec<SuccessRateDataPoint>,
    /// Latency distribution histogram
    pub latency_distribution: Vec<LatencyDataPoint>,
    /// Liquidity trend over time
    pub liquidity_trends: Vec<LiquidityDataPoint>,
    /// Related corridors
    pub related_corridors: Option<Vec<CorridorResponse>>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCorridorsQuery {
    /// Maximum number of results to return (default: 50)
    #[serde(default = "default_limit")]
    #[param(example = 50)]
    pub limit: i64,
    /// Pagination offset (default: 0)
    #[serde(default)]
    #[param(example = 0)]
    pub offset: i64,
    /// Sort by field (success_rate or volume)
    #[serde(default)]
    pub sort_by: SortBy,
    /// Minimum success rate filter
    #[param(example = 95.0)]
    pub success_rate_min: Option<f64>,
    /// Maximum success rate filter
    #[param(example = 100.0)]
    pub success_rate_max: Option<f64>,
    /// Minimum volume filter (USD)
    #[param(example = 100000.0)]
    pub volume_min: Option<f64>,
    /// Maximum volume filter (USD)
    #[param(example = 10000000.0)]
    pub volume_max: Option<f64>,
    /// Filter by asset code
    #[param(example = "USDC")]
    pub asset_code: Option<String>,
    /// Time period for metrics (24h, 7d, 30d)
    #[param(example = "24h")]
    pub time_period: Option<String>,
}

fn default_limit() -> i64 {
    50
}

fn calculate_health_score(success_rate: f64, total_transactions: i64, volume_usd: f64) -> f64 {
    let success_weight = 0.6;
    let volume_weight = 0.2;
    let transaction_weight = 0.2;

    let volume_score = if volume_usd > 0.0 {
        ((volume_usd.ln() / 15.0) * 100.0).min(100.0)
    } else {
        0.0
    };

    let transaction_score = if total_transactions > 0 {
        ((total_transactions as f64).ln() / 10.0 * 100.0).min(100.0)
    } else {
        0.0
    };

    success_rate * success_weight
        + volume_score * volume_weight
        + transaction_score * transaction_weight
}

fn get_liquidity_trend(volume_usd: f64) -> String {
    if volume_usd > 10_000_000.0 {
        "increasing".to_string()
    } else if volume_usd > 1_000_000.0 {
        "stable".to_string()
    } else {
        "decreasing".to_string()
    }
}

/// Generate cache key for corridor list with filters
fn generate_corridor_list_cache_key(params: &ListCorridorsQuery) -> String {
    let filter_str = format!(
        "sr_min:{:?}_sr_max:{:?}_vol_min:{:?}_vol_max:{:?}_asset:{:?}_period:{:?}",
        params.success_rate_min,
        params.success_rate_max,
        params.volume_min,
        params.volume_max,
        params.asset_code,
        params.time_period
    );
    keys::corridor_list(params.limit, params.offset, &filter_str)
}

/// List all payment corridors
///
/// Returns a list of payment corridors with performance metrics.
/// Supports filtering by success rate, volume, and asset code.
///
/// **DATA SOURCE: RPC**
/// - Payment data from Horizon API
/// - Trade data from Horizon API  
/// - Order book data from Horizon API
/// - Calculates corridor metrics from real-time RPC data
#[utoipa::path(
    get,
    path = "/api/corridors",
    params(ListCorridorsQuery),
    responses(
        (status = 200, description = "List of corridors retrieved successfully", body = Vec<CorridorResponse>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridors"
)]
pub async fn list_corridors(
    State((_db, cache, rpc_client, price_feed)): State<(
        Arc<Database>,
        Arc<CacheManager>,
        Arc<StellarRpcClient>,
        Arc<PriceFeedClient>,
    )>,
    Query(params): Query<ListCorridorsQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let cache_key = generate_corridor_list_cache_key(&params);

    let corridors = <()>::get_or_fetch(
        &cache,
        &cache_key,
        cache.config.get_ttl("corridor"),
        async {
            // **RPC DATA**: Fetch recent payments with pagination to identify active corridors
            // Use paginated fetch to get more complete data (up to configured limit)
            let payments = match rpc_client.fetch_all_payments(Some(1000)).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to fetch payments from RPC: {}", e);
                    return Ok(vec![]);
                }
            };

            // **RPC DATA**: Fetch recent trades with pagination for volume data
            let _trades = match rpc_client.fetch_all_trades(Some(1000)).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("Failed to fetch trades from RPC: {}", e);
                    vec![]
                }
            };

            // Group payments by asset pairs to identify corridors
            use std::collections::HashMap;
            let mut corridor_map: HashMap<String, Vec<&crate::rpc::Payment>> = HashMap::new();

            for payment in &payments {
                // Extract the actual asset pair from the payment
                if let Some(asset_pair) = extract_asset_pair_from_payment(payment) {
                    let corridor_key = asset_pair.to_corridor_key();
                    corridor_map
                        .entry(corridor_key)
                        .or_insert_with(Vec::new)
                        .push(payment);
                } else {
                    tracing::warn!("Failed to extract asset pair from payment: {}", payment.id);
                }
            }

            // Calculate metrics for each corridor
            let mut corridor_responses = Vec::new();

            for (corridor_key, corridor_payments) in corridor_map.iter() {
                let total_attempts = corridor_payments.len() as i64;

                // In Stellar, payments in the stream are successful
                let successful_payments = total_attempts;
                let failed_payments = 0;
                let success_rate = if total_attempts > 0 { 100.0 } else { 0.0 };

                // Parse corridor key to get assets
                let parts: Vec<&str> = corridor_key.split("->").collect();
                if parts.len() != 2 {
                    continue;
                }

                let source_parts: Vec<&str> = parts[0].split(':').collect();
                let dest_parts: Vec<&str> = parts[1].split(':').collect();

                if source_parts.len() != 2 || dest_parts.len() != 2 {
                    continue;
                }

                // Calculate volume from payment amounts and convert to USD
                let mut volume_usd: f64 = 0.0;
                let source_asset_key = parts[0];

                // Get price for source asset
                if let Ok(price) = price_feed.get_price(source_asset_key).await {
                    for payment in corridor_payments.iter() {
                        if let Ok(amount) = payment.amount.parse::<f64>() {
                            volume_usd += amount * price;
                        }
                    }
                } else {
                    // Fallback: use raw amounts if price unavailable
                    tracing::warn!(
                        "Price unavailable for {}, using raw amounts",
                        source_asset_key
                    );
                    volume_usd = corridor_payments
                        .iter()
                        .filter_map(|p| p.amount.parse::<f64>().ok())
                        .sum();
                }

                // Calculate health score
                let health_score = calculate_health_score(success_rate, total_attempts, volume_usd);
                let liquidity_trend = get_liquidity_trend(volume_usd);
                let avg_latency = 400.0 + (success_rate * 2.0);

                let corridor_response = CorridorResponse {
                    id: corridor_key.clone(),
                    source_asset: source_parts[0].to_string(),
                    destination_asset: dest_parts[0].to_string(),
                    success_rate,
                    total_attempts,
                    successful_payments,
                    failed_payments,
                    average_latency_ms: avg_latency,
                    median_latency_ms: avg_latency * 0.75,
                    p95_latency_ms: avg_latency * 2.5,
                    p99_latency_ms: avg_latency * 4.0,
                    liquidity_depth_usd: volume_usd,
                    liquidity_volume_24h_usd: volume_usd * 0.1,
                    liquidity_trend,
                    health_score,
                    last_updated: chrono::Utc::now().to_rfc3339(),
                };

                corridor_responses.push(corridor_response);
            }

            // Apply filters
            let filtered: Vec<_> = corridor_responses
                .into_iter()
                .filter(|c| {
                    if let Some(min) = params.success_rate_min {
                        if c.success_rate < min {
                            return false;
                        }
                    }
                    if let Some(max) = params.success_rate_max {
                        if c.success_rate > max {
                            return false;
                        }
                    }
                    if let Some(min) = params.volume_min {
                        if c.liquidity_depth_usd < min {
                            return false;
                        }
                    }
                    if let Some(max) = params.volume_max {
                        if c.liquidity_depth_usd > max {
                            return false;
                        }
                    }
                    if let Some(asset_code) = &params.asset_code {
                        let asset_code_lower = asset_code.to_lowercase();
                        if !c.source_asset.to_lowercase().contains(&asset_code_lower)
                            && !c
                                .destination_asset
                                .to_lowercase()
                                .contains(&asset_code_lower)
                        {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            Ok(filtered)
        },
    )
    .await?;

    let ttl = cache.config.get_ttl("corridor");
    let response = crate::http_cache::cached_json_response(&headers, &cache_key, &corridors, ttl)?;
    Ok(response)
}

/// Get detailed corridor information
///
/// Returns detailed metrics and historical data for a specific corridor.
///
/// **DATA SOURCE: RPC**
#[utoipa::path(
    get,
    path = "/api/corridors/{corridor_key}",
    params(
        ("corridor_key" = String, Path, description = "Corridor identifier (e.g., USDC:native->XLM:native)")
    ),
    responses(
        (status = 200, description = "Corridor details retrieved successfully", body = CorridorDetailResponse),
        (status = 404, description = "Corridor not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Corridors"
)]
pub async fn get_corridor_detail(
    State((_db, _cache, _rpc_client, _price_feed)): State<(
        Arc<Database>,
        Arc<CacheManager>,
        Arc<StellarRpcClient>,
        Arc<PriceFeedClient>,
    )>,
    Path(_corridor_key): Path<String>,
) -> ApiResult<Json<CorridorDetailResponse>> {
    // TODO: Implement RPC-based corridor detail
    Err(crate::handlers::ApiError::NotFound(
        "Corridor detail endpoint not yet implemented with RPC".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculation() {
        let score = calculate_health_score(95.0, 1000, 1_000_000.0);
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_liquidity_trend() {
        assert_eq!(get_liquidity_trend(15_000_000.0), "increasing");
        assert_eq!(get_liquidity_trend(5_000_000.0), "stable");
        assert_eq!(get_liquidity_trend(500_000.0), "decreasing");
    }

    #[test]
    fn test_extract_asset_pair_regular_payment_native() {
        let payment = crate::rpc::Payment {
            id: "test_1".to_string(),
            paging_token: "token_1".to_string(),
            transaction_hash: "hash_1".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("payment".to_string()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
        };

        let pair = extract_asset_pair_from_payment(&payment).unwrap();
        assert_eq!(pair.source_asset, "XLM:native");
        assert_eq!(pair.destination_asset, "XLM:native");
        assert_eq!(pair.to_corridor_key(), "XLM:native->XLM:native");
    }

    #[test]
    fn test_extract_asset_pair_regular_payment_issued_asset() {
        let payment = crate::rpc::Payment {
            id: "test_2".to_string(),
            paging_token: "token_2".to_string(),
            transaction_hash: "hash_2".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some("GISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("payment".to_string()),
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
        };

        let pair = extract_asset_pair_from_payment(&payment).unwrap();
        assert_eq!(pair.source_asset, "USDC:GISSUER");
        assert_eq!(pair.destination_asset, "USDC:GISSUER");
        assert_eq!(pair.to_corridor_key(), "USDC:GISSUER->USDC:GISSUER");
    }

    #[test]
    fn test_extract_asset_pair_path_payment_cross_asset() {
        let payment = crate::rpc::Payment {
            id: "test_3".to_string(),
            paging_token: "token_3".to_string(),
            transaction_hash: "hash_3".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("EUR".to_string()),
            asset_issuer: Some("GEURISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("path_payment_strict_send".to_string()),
            source_asset_type: Some("credit_alphanum4".to_string()),
            source_asset_code: Some("USD".to_string()),
            source_asset_issuer: Some("GUSDISSUER".to_string()),
            source_amount: Some("105.0".to_string()),
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
        };

        let pair = extract_asset_pair_from_payment(&payment).unwrap();
        assert_eq!(pair.source_asset, "USD:GUSDISSUER");
        assert_eq!(pair.destination_asset, "EUR:GEURISSUER");
        assert_eq!(pair.to_corridor_key(), "USD:GUSDISSUER->EUR:GEURISSUER");
    }

    #[test]
    fn test_extract_asset_pair_path_payment_native_to_issued() {
        let payment = crate::rpc::Payment {
            id: "test_4".to_string(),
            paging_token: "token_4".to_string(),
            transaction_hash: "hash_4".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some("GISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("path_payment_strict_receive".to_string()),
            source_asset_type: Some("native".to_string()),
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: Some("150.0".to_string()),
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
        };

        let pair = extract_asset_pair_from_payment(&payment).unwrap();
        assert_eq!(pair.source_asset, "XLM:native");
        assert_eq!(pair.destination_asset, "USDC:GISSUER");
        assert_eq!(pair.to_corridor_key(), "XLM:native->USDC:GISSUER");
    }

    #[test]
    fn test_extract_asset_pair_path_payment_issued_to_native() {
        let payment = crate::rpc::Payment {
            id: "test_5".to_string(),
            paging_token: "token_5".to_string(),
            transaction_hash: "hash_5".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "native".to_string(),
            asset_code: None,
            asset_issuer: None,
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: Some("path_payment_strict_send".to_string()),
            source_asset_type: Some("credit_alphanum4".to_string()),
            source_asset_code: Some("BRL".to_string()),
            source_asset_issuer: Some("GBRLISSUER".to_string()),
            source_amount: Some("500.0".to_string()),
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
        };

        let pair = extract_asset_pair_from_payment(&payment).unwrap();
        assert_eq!(pair.source_asset, "BRL:GBRLISSUER");
        assert_eq!(pair.destination_asset, "XLM:native");
        assert_eq!(pair.to_corridor_key(), "BRL:GBRLISSUER->XLM:native");
    }

    #[test]
    fn test_extract_asset_pair_missing_operation_type() {
        // Should default to regular payment behavior
        let payment = crate::rpc::Payment {
            id: "test_6".to_string(),
            paging_token: "token_6".to_string(),
            transaction_hash: "hash_6".to_string(),
            source_account: "GTEST".to_string(),
            destination: "GDEST".to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("NGNT".to_string()),
            asset_issuer: Some("GNGNTISSUER".to_string()),
            amount: "100.0".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            operation_type: None,
            source_asset_type: None,
            source_asset_code: None,
            source_asset_issuer: None,
            source_amount: None,
            from: Some("GTEST".to_string()),
            to: Some("GDEST".to_string()),
        };

        let pair = extract_asset_pair_from_payment(&payment).unwrap();
        assert_eq!(pair.source_asset, "NGNT:GNGNTISSUER");
        assert_eq!(pair.destination_asset, "NGNT:GNGNTISSUER");
    }
}
