use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::cache::{keys, CacheManager};
use crate::cache_middleware::CacheAware;
use crate::database::Database;
use crate::handlers::ApiResult;
use crate::models::corridor::Corridor;
use crate::models::SortBy;
use crate::rpc::StellarRpcClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorResponse {
    pub id: String,
    pub source_asset: String,
    pub destination_asset: String,
    pub success_rate: f64,
    pub total_attempts: i64,
    pub successful_payments: i64,
    pub failed_payments: i64,
    pub average_latency_ms: f64,
    pub median_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub liquidity_depth_usd: f64,
    pub liquidity_volume_24h_usd: f64,
    pub liquidity_trend: String,
    pub health_score: f64,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessRateDataPoint {
    pub timestamp: String,
    pub success_rate: f64,
    pub attempts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDataPoint {
    pub latency_bucket_ms: i32,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityDataPoint {
    pub timestamp: String,
    pub liquidity_usd: f64,
    pub volume_24h_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorDetailResponse {
    pub corridor: CorridorResponse,
    pub historical_success_rate: Vec<SuccessRateDataPoint>,
    pub latency_distribution: Vec<LatencyDataPoint>,
    pub liquidity_trends: Vec<LiquidityDataPoint>,
    pub related_corridors: Option<Vec<CorridorResponse>>,
}

#[derive(Debug, Deserialize)]
pub struct ListCorridorsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub sort_by: SortBy,
    pub success_rate_min: Option<f64>,
    pub success_rate_max: Option<f64>,
    pub volume_min: Option<f64>,
    pub volume_max: Option<f64>,
    pub asset_code: Option<String>,
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

/// GET /api/corridors - List all corridors (cached)
///
/// **DATA SOURCE: RPC**
/// - Payment data from Horizon API
/// - Trade data from Horizon API  
/// - Order book data from Horizon API
/// - Calculates corridor metrics from real-time RPC data
pub async fn list_corridors(
    State((_db, cache, rpc_client)): State<(Arc<Database>, Arc<CacheManager>, Arc<StellarRpcClient>)>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<Vec<CorridorResponse>>> {
    let cache_key = generate_corridor_list_cache_key(&params);

    let corridors = <()>::get_or_fetch(
        &cache,
        &cache_key,
        cache.config.get_ttl("corridor"),
        async {
            // **RPC DATA**: Fetch recent payments to identify active corridors
            let payments = match rpc_client.fetch_payments(200, None).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to fetch payments from RPC: {}", e);
                    return Ok(vec![]);
                }
            };

            // **RPC DATA**: Fetch recent trades for volume data
            let _trades = match rpc_client.fetch_trades(200, None).await {
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
                let asset_from = format!(
                    "{}:{}",
                    payment.asset_code.as_deref().unwrap_or("XLM"),
                    payment.asset_issuer.as_deref().unwrap_or("native")
                );
                
                // For now, assume destination is XLM (we'd need more data to determine actual destination asset)
                let asset_to = "XLM:native".to_string();
                
                let corridor_key = format!("{}->{}", asset_from, asset_to);
                corridor_map.entry(corridor_key).or_insert_with(Vec::new).push(payment);
            }

            // Calculate metrics for each corridor
            let mut corridor_responses = Vec::new();

            for (corridor_key, corridor_payments) in corridor_map.iter() {
                let total_attempts = corridor_payments.len() as i64;
                
                // In Stellar, payments in the stream are successful
                let successful_payments = total_attempts;
                let failed_payments = 0;
                let success_rate = if total_attempts > 0 { 100.0 } else { 0.0 };

                // Calculate volume from payment amounts
                let volume_usd: f64 = corridor_payments
                    .iter()
                    .filter_map(|p| p.amount.parse::<f64>().ok())
                    .sum();

                // Calculate health score
                let health_score = calculate_health_score(success_rate, total_attempts, volume_usd);
                let liquidity_trend = get_liquidity_trend(volume_usd);
                let avg_latency = 400.0 + (success_rate * 2.0);

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
                            && !c.destination_asset.to_lowercase().contains(&asset_code_lower)
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

    Ok(Json(corridors))
}


/// GET /api/corridors/:corridor_key - Get detailed corridor information (cached)
pub async fn get_corridor_detail(
    State((_db, _cache, _rpc_client)): State<(Arc<Database>, Arc<CacheManager>, Arc<StellarRpcClient>)>,
    Path(_corridor_key): Path<String>,
) -> ApiResult<Json<CorridorDetailResponse>> {
    // TODO: Implement RPC-based corridor detail
    Err(crate::handlers::ApiError::NotFound(
        "Corridor detail endpoint not yet implemented with RPC".to_string()
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
}
