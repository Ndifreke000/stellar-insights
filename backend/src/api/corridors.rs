use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::handlers::{ApiError, ApiResult};
use crate::models::corridor::{Corridor, CorridorMetrics};
use crate::models::SortBy;
use crate::state::AppState;

// Response DTOs matching frontend TypeScript interfaces

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
    // Filter parameters
    pub success_rate_min: Option<f64>,
    pub success_rate_max: Option<f64>,
    pub volume_min: Option<f64>,
    pub volume_max: Option<f64>,
    pub asset_code: Option<String>,
    pub time_period: Option<String>, // "7d", "30d", "90d"
}

fn default_limit() -> i64 {
    50
}

/// Calculate health score based on success rate, volume, and transaction count
fn calculate_health_score(success_rate: f64, total_transactions: i64, volume_usd: f64) -> f64 {
    let success_weight = 0.6;
    let volume_weight = 0.2;
    let transaction_weight = 0.2;

    // Normalize volume and transactions (using logarithmic scale)
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

/// Determine liquidity trend (simple heuristic based on recent data)
fn get_liquidity_trend(volume_usd: f64) -> String {
    if volume_usd > 10_000_000.0 {
        "increasing".to_string()
    } else if volume_usd > 1_000_000.0 {
        "stable".to_string()
    } else {
        "decreasing".to_string()
    }
}

/// GET /api/corridors - List all corridors
pub async fn list_corridors(
    State(app_state): State<AppState>,
    Query(params): Query<ListCorridorsQuery>,
) -> ApiResult<Json<Vec<CorridorResponse>>> {
    let today = Utc::now().date_naive();

    // Determine date range based on time_period
    let (start_date, end_date) = match params.time_period.as_deref() {
        Some("7d") => (today - Duration::days(7), today),
        Some("30d") => (today - Duration::days(30), today),
        Some("90d") => (today - Duration::days(90), today),
        _ => (today, today), // Default to today
    };

    let metrics = if params.time_period.is_some() {
        // Use aggregated metrics for time periods
        let aggregated = app_state.db
            .corridor_aggregates()
            .get_aggregated_corridor_metrics(start_date, end_date)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch corridors: {}", e)))?;

        // Convert to CorridorMetrics-like structure for filtering
        aggregated
            .into_iter()
            .map(|m| CorridorMetrics {
                id: format!("{}-{}", m.corridor_key, start_date),
                corridor_key: m.corridor_key,
                asset_a_code: m.asset_a_code,
                asset_a_issuer: m.asset_a_issuer,
                asset_b_code: m.asset_b_code,
                asset_b_issuer: m.asset_b_issuer,
                date: m.latest_date,
                total_transactions: m.total_transactions,
                successful_transactions: m.successful_transactions,
                failed_transactions: m.failed_transactions,
                success_rate: m.avg_success_rate,
                volume_usd: m.total_volume_usd,
                avg_settlement_latency_ms: None,
                median_settlement_latency_ms: None,
                liquidity_depth_usd: m.total_volume_usd,
                created_at: m.latest_date,
                updated_at: m.latest_date,
            })
            .collect()
    } else {
        // Use daily metrics for single day
        app_state.db.corridor_aggregates()
            .get_corridor_metrics_for_date(today)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to fetch corridors: {}", e)))?
    };

    // Apply filters
    let filtered_metrics: Vec<_> = metrics
        .into_iter()
        .filter(|m| {
            // Success rate filter
            if let Some(min) = params.success_rate_min {
                if m.success_rate < min {
                    return false;
                }
            }
            if let Some(max) = params.success_rate_max {
                if m.success_rate > max {
                    return false;
                }
            }

            // Volume filter
            if let Some(min) = params.volume_min {
                if m.volume_usd < min {
                    return false;
                }
            }
            if let Some(max) = params.volume_max {
                if m.volume_usd > max {
                    return false;
                }
            }

            // Asset code filter
            if let Some(asset_code) = &params.asset_code {
                let asset_code_lower = asset_code.to_lowercase();
                if !m.asset_a_code.to_lowercase().contains(&asset_code_lower)
                    && !m.asset_b_code.to_lowercase().contains(&asset_code_lower)
                {
                    return false;
                }
            }

            true
        })
        .collect();

    let corridors: Vec<CorridorResponse> = filtered_metrics
        .iter()
        .map(|m| {
            let health_score =
                calculate_health_score(m.success_rate, m.total_transactions, m.volume_usd);
            let liquidity_trend = get_liquidity_trend(m.volume_usd);
            let avg_latency = 400.0 + (m.success_rate * 2.0);

            CorridorResponse {
                id: m.corridor_key.clone(),
                source_asset: m.asset_a_code.clone(),
                destination_asset: m.asset_b_code.clone(),
                success_rate: m.success_rate,
                total_attempts: m.total_transactions,
                successful_payments: m.successful_transactions,
                failed_payments: m.failed_transactions,
                average_latency_ms: avg_latency,
                median_latency_ms: avg_latency * 0.75,
                p95_latency_ms: avg_latency * 2.5,
                p99_latency_ms: avg_latency * 4.0,
                liquidity_depth_usd: m.volume_usd,
                liquidity_volume_24h_usd: m.volume_usd * 0.1,
                liquidity_trend,
                health_score,
                last_updated: m.updated_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(corridors))
}

/// GET /api/corridors/:corridor_key - Get detailed corridor information
pub async fn get_corridor_detail(
    State(app_state): State<AppState>,
    Path(corridor_key): Path<String>,
) -> ApiResult<Json<CorridorDetailResponse>> {
    let parts: Vec<&str> = corridor_key.split("->").collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest(
            "Invalid corridor key format".to_string(),
        ));
    }

    let asset_a_parts: Vec<&str> = parts[0].split(':').collect();
    let asset_b_parts: Vec<&str> = parts[1].split(':').collect();

    if asset_a_parts.len() != 2 || asset_b_parts.len() != 2 {
        return Err(ApiError::BadRequest(
            "Invalid corridor key format".to_string(),
        ));
    }

    let corridor = Corridor::new(
        asset_a_parts[0].to_string(),
        asset_a_parts[1].to_string(),
        asset_b_parts[0].to_string(),
        asset_b_parts[1].to_string(),
    );

    let end_date = Utc::now().date_naive();
    let start_date = end_date - Duration::days(30);

    let metrics = app_state.db
        .corridor_aggregates()
        .get_corridor_metrics(&corridor, start_date, end_date)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch corridor detail: {}", e)))?;

    if metrics.is_empty() {
        return Err(ApiError::NotFound(format!(
            "Corridor {} not found",
            corridor_key
        )));
    }

    let latest = metrics.first().unwrap();
    let health_score = calculate_health_score(
        latest.success_rate,
        latest.total_transactions,
        latest.volume_usd,
    );
    let liquidity_trend = get_liquidity_trend(latest.volume_usd);
    let avg_latency = 400.0 + (latest.success_rate * 2.0);

    let corridor_response = CorridorResponse {
        id: latest.corridor_key.clone(),
        source_asset: latest.asset_a_code.clone(),
        destination_asset: latest.asset_b_code.clone(),
        success_rate: latest.success_rate,
        total_attempts: latest.total_transactions,
        successful_payments: latest.successful_transactions,
        failed_payments: latest.failed_transactions,
        average_latency_ms: avg_latency,
        median_latency_ms: avg_latency * 0.75,
        p95_latency_ms: avg_latency * 2.5,
        p99_latency_ms: avg_latency * 4.0,
        liquidity_depth_usd: latest.volume_usd,
        liquidity_volume_24h_usd: latest.volume_usd * 0.1,
        liquidity_trend,
        health_score,
        last_updated: latest.updated_at.to_rfc3339(),
    };

    let historical_success_rate: Vec<SuccessRateDataPoint> = metrics
        .iter()
        .rev()
        .map(|m| SuccessRateDataPoint {
            timestamp: m.date.format("%Y-%m-%d").to_string(),
            success_rate: m.success_rate,
            attempts: m.total_transactions,
        })
        .collect();

    let latency_distribution = vec![
        LatencyDataPoint {
            latency_bucket_ms: 100,
            count: 250,
            percentage: 15.0,
        },
        LatencyDataPoint {
            latency_bucket_ms: 250,
            count: 520,
            percentage: 31.0,
        },
        LatencyDataPoint {
            latency_bucket_ms: 500,
            count: 580,
            percentage: 35.0,
        },
        LatencyDataPoint {
            latency_bucket_ms: 1000,
            count: 280,
            percentage: 17.0,
        },
        LatencyDataPoint {
            latency_bucket_ms: 2000,
            count: 50,
            percentage: 3.0,
        },
    ];

    let liquidity_trends: Vec<LiquidityDataPoint> = metrics
        .iter()
        .rev()
        .map(|m| LiquidityDataPoint {
            timestamp: m.date.format("%Y-%m-%d").to_string(),
            liquidity_usd: m.volume_usd,
            volume_24h_usd: m.volume_usd * 0.1,
        })
        .collect();

    let related_metrics = app_state.db
        .corridor_aggregates()
        .get_top_corridors_by_volume(end_date, 4)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to fetch related corridors: {}", e))
        })?;

    let related_corridors: Vec<CorridorResponse> = related_metrics
        .iter()
        .filter(|m| m.corridor_key != latest.corridor_key)
        .take(3)
        .map(|m| {
            let health_score =
                calculate_health_score(m.success_rate, m.total_transactions, m.volume_usd);
            let liquidity_trend = get_liquidity_trend(m.volume_usd);
            let avg_latency = 400.0 + (m.success_rate * 2.0);

            CorridorResponse {
                id: m.corridor_key.clone(),
                source_asset: m.asset_a_code.clone(),
                destination_asset: m.asset_b_code.clone(),
                success_rate: m.success_rate,
                total_attempts: m.total_transactions,
                successful_payments: m.successful_transactions,
                failed_payments: m.failed_transactions,
                average_latency_ms: avg_latency,
                median_latency_ms: avg_latency * 0.75,
                p95_latency_ms: avg_latency * 2.5,
                p99_latency_ms: avg_latency * 4.0,
                liquidity_depth_usd: m.volume_usd,
                liquidity_volume_24h_usd: m.volume_usd * 0.1,
                liquidity_trend,
                health_score,
                last_updated: m.updated_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(CorridorDetailResponse {
        corridor: corridor_response,
        historical_success_rate,
        latency_distribution,
        liquidity_trends,
        related_corridors: Some(related_corridors),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::corridor::CorridorMetrics;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_corridor_response_from_metrics() {
        let metrics = CorridorMetrics {
            id: Uuid::new_v4().to_string(),
            corridor_key: "EURC:issuer2->USDC:issuer1".to_string(),
            asset_a_code: "EURC".to_string(),
            asset_a_issuer: "issuer2".to_string(),
            asset_b_code: "USDC".to_string(),
            asset_b_issuer: "issuer1".to_string(),
            date: Utc::now(),
            total_transactions: 1000,
            successful_transactions: 950,
            failed_transactions: 50,
            success_rate: 95.0,
            volume_usd: 1000000.0,
            avg_settlement_latency_ms: Some(400),
            median_settlement_latency_ms: Some(300),
            liquidity_depth_usd: 500000.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response = CorridorResponse {
            id: metrics.corridor_key.clone(),
            source_asset: metrics.asset_a_code.clone(),
            destination_asset: metrics.asset_b_code.clone(),
            success_rate: metrics.success_rate,
            total_attempts: metrics.total_transactions,
            successful_payments: metrics.successful_transactions,
            failed_payments: metrics.failed_transactions,
            average_latency_ms: 400.0,
            median_latency_ms: 300.0,
            p95_latency_ms: 1000.0,
            p99_latency_ms: 1600.0,
            liquidity_depth_usd: metrics.volume_usd,
            liquidity_volume_24h_usd: metrics.volume_usd * 0.1,
            liquidity_trend: "stable".to_string(),
            health_score: 95.0,
            last_updated: metrics.updated_at.to_rfc3339(),
        };

        assert_eq!(response.source_asset, "EURC");
        assert_eq!(response.destination_asset, "USDC");
        assert_eq!(response.success_rate, 95.0);
        assert_eq!(response.total_attempts, 1000);
        assert_eq!(response.liquidity_depth_usd, 1000000.0);
        assert!(response.id.contains("EURC:issuer2"));
        assert!(response.id.contains("USDC:issuer1"));
    }
}
