use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::services::price_feed::PriceFeedClient;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetPriceQuery {
    /// Stellar asset identifier (e.g., "XLM:native", "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")
    #[param(example = "XLM:native")]
    pub asset: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetPricesQuery {
    /// Comma-separated list of Stellar asset identifiers
    #[param(example = "XLM:native,USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")]
    pub assets: String,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ConvertQuery {
    /// Stellar asset identifier
    #[param(example = "XLM:native")]
    pub asset: String,
    /// Amount to convert
    #[param(example = 100.0)]
    pub amount: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PriceResponse {
    /// Stellar asset identifier
    #[schema(example = "XLM:native")]
    pub asset: String,
    /// Price in USD
    #[schema(example = 0.12)]
    pub price_usd: f64,
    /// Timestamp of the response
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PricesResponse {
    /// Map of asset to price in USD
    pub prices: std::collections::HashMap<String, f64>,
    /// Timestamp of the response
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConvertResponse {
    /// Stellar asset identifier
    #[schema(example = "XLM:native")]
    pub asset: String,
    /// Original amount
    #[schema(example = 100.0)]
    pub amount: f64,
    /// Converted amount in USD
    #[schema(example = 12.0)]
    pub amount_usd: f64,
    /// Price used for conversion
    #[schema(example = 0.12)]
    pub price_usd: f64,
    /// Timestamp of the response
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheStatsResponse {
    /// Total number of cached prices
    #[schema(example = 10)]
    pub total_cached: usize,
    /// Number of fresh (non-expired) cached prices
    #[schema(example = 8)]
    pub fresh_cached: usize,
    /// Timestamp of the response
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Get price for a single asset
///
/// Returns the current USD price for a Stellar asset.
///
/// **DATA SOURCE: CoinGecko API**
#[utoipa::path(
    get,
    path = "/api/prices",
    params(GetPriceQuery),
    responses(
        (status = 200, description = "Price retrieved successfully", body = PriceResponse),
        (status = 400, description = "Invalid asset identifier"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prices"
)]
pub async fn get_price(
    State(price_feed): State<Arc<PriceFeedClient>>,
    Query(params): Query<GetPriceQuery>,
) -> impl IntoResponse {
    match price_feed.get_price(&params.asset).await {
        Ok(price) => {
            let response = PriceResponse {
                asset: params.asset,
                price_usd: price,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let error = ErrorResponse {
                error: format!("Failed to fetch price: {}", e),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// Get prices for multiple assets
///
/// Returns the current USD prices for multiple Stellar assets.
///
/// **DATA SOURCE: CoinGecko API**
#[utoipa::path(
    get,
    path = "/api/prices/batch",
    params(GetPricesQuery),
    responses(
        (status = 200, description = "Prices retrieved successfully", body = PricesResponse),
        (status = 400, description = "Invalid asset identifiers"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prices"
)]
pub async fn get_prices(
    State(price_feed): State<Arc<PriceFeedClient>>,
    Query(params): Query<GetPricesQuery>,
) -> impl IntoResponse {
    let assets: Vec<String> = params
        .assets
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if assets.is_empty() {
        let error = ErrorResponse {
            error: "No assets provided".to_string(),
        };
        return (StatusCode::BAD_REQUEST, Json(error)).into_response();
    }

    let prices = price_feed.get_prices(&assets).await;

    let response = PricesResponse {
        prices,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

/// Convert asset amount to USD
///
/// Converts an amount of a Stellar asset to USD using current prices.
///
/// **DATA SOURCE: CoinGecko API**
#[utoipa::path(
    get,
    path = "/api/prices/convert",
    params(ConvertQuery),
    responses(
        (status = 200, description = "Conversion successful", body = ConvertResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prices"
)]
pub async fn convert_to_usd(
    State(price_feed): State<Arc<PriceFeedClient>>,
    Query(params): Query<ConvertQuery>,
) -> impl IntoResponse {
    match price_feed
        .convert_to_usd(&params.asset, params.amount)
        .await
    {
        Ok(amount_usd) => {
            let price_usd = amount_usd / params.amount;
            let response = ConvertResponse {
                asset: params.asset,
                amount: params.amount,
                amount_usd,
                price_usd,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            let error = ErrorResponse {
                error: format!("Failed to convert: {}", e),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error)).into_response()
        }
    }
}

/// Get cache statistics
///
/// Returns statistics about the price cache.
#[utoipa::path(
    get,
    path = "/api/prices/cache-stats",
    responses(
        (status = 200, description = "Cache stats retrieved successfully", body = CacheStatsResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Prices"
)]
pub async fn get_cache_stats(State(price_feed): State<Arc<PriceFeedClient>>) -> impl IntoResponse {
    let (total, fresh) = price_feed.cache_stats().await;
    let response = CacheStatsResponse {
        total_cached: total,
        fresh_cached: fresh,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    (StatusCode::OK, Json(response)).into_response()
}

/// Create price feed routes
pub fn routes(price_feed: Arc<PriceFeedClient>) -> Router {
    Router::new()
        .route("/", get(get_price))
        .route("/batch", get(get_prices))
        .route("/convert", get(convert_to_usd))
        .route("/cache-stats", get(get_cache_stats))
        .with_state(price_feed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assets() {
        let assets_str = "XLM:native,USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
        let assets: Vec<String> = assets_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0], "XLM:native");
    }
}
