use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::http_cache::cached_json_response;
use crate::services::price_feed::PriceFeedClient;

const DEFAULT_CACHE_TTL_SECONDS: usize = 60;
const USDC_ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentRoute {
    StellarDex,
    AnchorDirect,
    LiquidityPool,
}

impl PaymentRoute {
    fn default_routes() -> Vec<Self> {
        vec![Self::StellarDex, Self::AnchorDirect, Self::LiquidityPool]
    }

    fn as_key(&self) -> &'static str {
        match self {
            Self::StellarDex => "stellar_dex",
            Self::AnchorDirect => "anchor_direct",
            Self::LiquidityPool => "liquidity_pool",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::StellarDex => "Stellar DEX",
            Self::AnchorDirect => "Anchor Direct",
            Self::LiquidityPool => "Liquidity Pool",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CostCalculationRequest {
    #[schema(example = "USDC")]
    pub source_currency: String,
    #[schema(example = "NGN")]
    pub destination_currency: String,
    #[schema(example = 1000.0)]
    pub source_amount: f64,
    #[schema(example = 1550000.0)]
    pub destination_amount: Option<f64>,
    pub routes: Option<Vec<PaymentRoute>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RouteCostBreakdown {
    pub exchange_rate_mid: f64,
    pub effective_rate: f64,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub spread_cost_source: f64,
    pub service_fee_source: f64,
    pub network_fee_source: f64,
    pub slippage_cost_source: f64,
    pub total_fees_source: f64,
    pub total_fees_destination: f64,
    pub estimated_destination_amount: f64,
    pub destination_shortfall: Option<f64>,
    pub additional_source_required: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RouteEstimate {
    pub route: PaymentRoute,
    pub route_name: String,
    pub breakdown: RouteCostBreakdown,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CostCalculationResponse {
    pub source_currency: String,
    pub destination_currency: String,
    pub source_amount: f64,
    pub destination_amount: Option<f64>,
    pub source_usd_rate: f64,
    pub destination_usd_rate: f64,
    pub mid_market_rate: f64,
    pub best_route: RouteEstimate,
    pub routes: Vec<RouteEstimate>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Copy)]
struct RouteFees {
    spread_bps: f64,
    service_fee_bps: f64,
    network_fee_source: f64,
    slippage_base_bps: f64,
    slippage_per_10k_bps: f64,
}

impl RouteFees {
    fn for_route(route: PaymentRoute) -> Self {
        match route {
            PaymentRoute::StellarDex => Self {
                spread_bps: 35.0,
                service_fee_bps: 15.0,
                network_fee_source: 0.12,
                slippage_base_bps: 8.0,
                slippage_per_10k_bps: 2.0,
            },
            PaymentRoute::AnchorDirect => Self {
                spread_bps: 60.0,
                service_fee_bps: 45.0,
                network_fee_source: 0.20,
                slippage_base_bps: 12.0,
                slippage_per_10k_bps: 3.5,
            },
            PaymentRoute::LiquidityPool => Self {
                spread_bps: 25.0,
                service_fee_bps: 25.0,
                network_fee_source: 0.08,
                slippage_base_bps: 10.0,
                slippage_per_10k_bps: 4.0,
            },
        }
    }
}

/// Estimate total cross-border payment costs and compare available routes.
#[utoipa::path(
    post,
    path = "/api/cost-calculator/estimate",
    request_body = CostCalculationRequest,
    responses(
        (status = 200, description = "Cost estimate generated", body = CostCalculationResponse),
        (status = 304, description = "Not modified. Conditional request matched current response."),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Cost Calculator"
)]
pub async fn estimate_costs(
    State(price_feed): State<Arc<PriceFeedClient>>,
    request_headers: HeaderMap,
    Json(request): Json<CostCalculationRequest>,
) -> Response {
    let source_currency = normalize_currency(&request.source_currency);
    let destination_currency = normalize_currency(&request.destination_currency);

    if source_currency.is_empty() || destination_currency.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "source_currency and destination_currency are required",
        );
    }

    if request.source_amount <= 0.0 || !request.source_amount.is_finite() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "source_amount must be a positive number",
        );
    }

    if let Some(destination_amount) = request.destination_amount {
        if destination_amount <= 0.0 || !destination_amount.is_finite() {
            return error_response(
                StatusCode::BAD_REQUEST,
                "destination_amount must be a positive number when provided",
            );
        }
    }

    let requested_routes = request.routes.unwrap_or_else(PaymentRoute::default_routes);
    let unique_routes: Vec<PaymentRoute> = BTreeSet::from_iter(requested_routes.into_iter())
        .into_iter()
        .collect();

    if unique_routes.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "at least one route is required");
    }

    let source_usd_rate = match resolve_usd_rate(&price_feed, &source_currency).await {
        Ok(rate) => rate,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error),
    };

    let destination_usd_rate = match resolve_usd_rate(&price_feed, &destination_currency).await {
        Ok(rate) => rate,
        Err(error) => return error_response(StatusCode::BAD_REQUEST, &error),
    };

    if destination_usd_rate <= 0.0 {
        return error_response(StatusCode::BAD_REQUEST, "destination USD rate is invalid");
    }

    let mid_market_rate = source_usd_rate / destination_usd_rate;

    let mut route_estimates: Vec<RouteEstimate> = unique_routes
        .into_iter()
        .map(|route| {
            estimate_route(
                route,
                request.source_amount,
                request.destination_amount,
                mid_market_rate,
            )
        })
        .collect();

    route_estimates.sort_by(|a, b| {
        a.breakdown
            .total_fees_source
            .partial_cmp(&b.breakdown.total_fees_source)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let Some(best_route) = route_estimates.first().cloned() else {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to estimate routes",
        );
    };

    let response = CostCalculationResponse {
        source_currency: source_currency.clone(),
        destination_currency: destination_currency.clone(),
        source_amount: request.source_amount,
        destination_amount: request.destination_amount,
        source_usd_rate,
        destination_usd_rate,
        mid_market_rate,
        best_route,
        routes: route_estimates,
    };

    let route_key = response
        .routes
        .iter()
        .map(|route| route.route.as_key())
        .collect::<Vec<_>>()
        .join(",");

    let resource_key = format!(
        "cost-calculator:{}:{}:{:.8}:{}:{:?}",
        source_currency,
        destination_currency,
        request.source_amount,
        route_key,
        request.destination_amount
    );

    match cached_json_response(
        &request_headers,
        &resource_key,
        &response,
        DEFAULT_CACHE_TTL_SECONDS,
    ) {
        Ok(response) => response,
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to serialize response: {error}"),
        ),
    }
}

fn estimate_route(
    route: PaymentRoute,
    source_amount: f64,
    destination_target: Option<f64>,
    mid_market_rate: f64,
) -> RouteEstimate {
    let fees = RouteFees::for_route(route);
    let slippage_bps = (fees.slippage_base_bps
        + (source_amount / 10_000.0) * fees.slippage_per_10k_bps)
        .min(200.0);

    let destination_before_fees = source_amount * mid_market_rate;
    let spread_cost_destination = destination_before_fees * (fees.spread_bps / 10_000.0);
    let destination_after_spread = destination_before_fees - spread_cost_destination;

    let service_fee_source = source_amount * (fees.service_fee_bps / 10_000.0);
    let service_fee_destination = service_fee_source * mid_market_rate;
    let network_fee_destination = fees.network_fee_source * mid_market_rate;
    let slippage_cost_destination = destination_after_spread * (slippage_bps / 10_000.0);

    let estimated_destination_amount = (destination_after_spread
        - service_fee_destination
        - network_fee_destination
        - slippage_cost_destination)
        .max(0.0);

    let spread_cost_source = spread_cost_destination / mid_market_rate;
    let slippage_cost_source = slippage_cost_destination / mid_market_rate;
    let total_fees_source =
        spread_cost_source + service_fee_source + fees.network_fee_source + slippage_cost_source;
    let total_fees_destination = spread_cost_destination
        + service_fee_destination
        + network_fee_destination
        + slippage_cost_destination;

    let effective_rate = if source_amount > 0.0 {
        estimated_destination_amount / source_amount
    } else {
        0.0
    };

    let destination_shortfall = destination_target
        .map(|target| (target - estimated_destination_amount).max(0.0))
        .filter(|shortfall| *shortfall > 0.0);

    let additional_source_required = destination_shortfall
        .and_then(|shortfall| {
            if effective_rate > 0.0 {
                Some(shortfall / effective_rate)
            } else {
                None
            }
        })
        .filter(|required| required.is_finite() && *required > 0.0);

    RouteEstimate {
        route,
        route_name: route.label().to_string(),
        breakdown: RouteCostBreakdown {
            exchange_rate_mid: mid_market_rate,
            effective_rate,
            spread_bps: fees.spread_bps,
            slippage_bps,
            spread_cost_source,
            service_fee_source,
            network_fee_source: fees.network_fee_source,
            slippage_cost_source,
            total_fees_source,
            total_fees_destination,
            estimated_destination_amount,
            destination_shortfall,
            additional_source_required,
        },
    }
}

async fn resolve_usd_rate(price_feed: &PriceFeedClient, currency: &str) -> Result<f64, String> {
    if currency.contains(':') {
        if let Ok(rate) = price_feed.get_price(currency).await {
            if rate > 0.0 && rate.is_finite() {
                return Ok(rate);
            }
        }

        if let Some(asset_code) = currency.split(':').next() {
            if let Some(rate) = fallback_usd_rate(&asset_code.to_uppercase()) {
                return Ok(rate);
            }
        }

        return Err(format!("Unsupported currency or asset: {currency}"));
    }

    if let Some(asset_id) = price_feed_asset_id(currency) {
        if let Ok(rate) = price_feed.get_price(asset_id).await {
            if rate > 0.0 && rate.is_finite() {
                return Ok(rate);
            }
        }
    }

    fallback_usd_rate(currency).ok_or_else(|| format!("Unsupported currency or asset: {currency}"))
}

fn price_feed_asset_id(currency: &str) -> Option<&'static str> {
    match currency {
        "XLM" => Some("XLM:native"),
        "USDC" => Some("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
        "EURC" => Some("EURC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
        _ => None,
    }
}

fn fallback_usd_rate(currency: &str) -> Option<f64> {
    match currency {
        "USD" | "USDC" | "USDT" => Some(1.0),
        "EUR" | "EURC" => Some(1.08),
        "GBP" => Some(1.27),
        "NGN" => Some(0.00065),
        "KES" => Some(0.0077),
        "GHS" => Some(0.064),
        "PHP" => Some(0.0178),
        "INR" => Some(0.012),
        "XLM" => Some(0.12),
        "BTC" => Some(62000.0),
        "ETH" => Some(3200.0),
        _ => None,
    }
}

fn normalize_currency(input: &str) -> String {
    let value = input.trim();
    if value.contains(':') {
        if let Some((code, issuer)) = value.split_once(':') {
            if issuer.eq_ignore_ascii_case("native") {
                return format!("{}:native", code.trim().to_uppercase());
            }

            if code.eq_ignore_ascii_case("USDC") && issuer.eq_ignore_ascii_case(USDC_ISSUER) {
                return format!("USDC:{USDC_ISSUER}");
            }

            return format!(
                "{}:{}",
                code.trim().to_uppercase(),
                issuer.trim().to_uppercase()
            );
        }

        return value.to_string();
    }

    value.to_uppercase()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: message.to_string(),
        }),
    )
        .into_response()
}

pub fn routes(price_feed: Arc<PriceFeedClient>) -> Router {
    Router::new()
        .route("/estimate", post(estimate_costs))
        .with_state(price_feed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_currency() {
        assert_eq!(normalize_currency(" usd "), "USD");
        assert_eq!(normalize_currency("xlm:native"), "XLM:native");
        assert_eq!(
            normalize_currency("usdc:ga5zsejyb37jrc5avcia5mop4rhtm335x2kgx3ihojapp5re34k4kzvn"),
            format!("USDC:{USDC_ISSUER}")
        );
    }

    #[test]
    fn test_estimate_route_produces_positive_fees() {
        let estimate = estimate_route(
            PaymentRoute::StellarDex,
            1_000.0,
            Some(1_500_000.0),
            1_538.0,
        );
        assert!(estimate.breakdown.total_fees_source > 0.0);
        assert!(estimate.breakdown.estimated_destination_amount > 0.0);
    }

    #[test]
    fn test_fallback_rates_cover_common_assets() {
        assert_eq!(fallback_usd_rate("USD"), Some(1.0));
        assert_eq!(fallback_usd_rate("USDC"), Some(1.0));
        assert!(fallback_usd_rate("NGN").is_some());
    }
}
