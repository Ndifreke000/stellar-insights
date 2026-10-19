use axum::{
    extract::State,
    http::HeaderMap,
    response::Response,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;
use utoipa::ToSchema;

use crate::error::{ApiError, DomainError};
use crate::http_cache::cached_json_response;
use crate::services::price_feed::PriceFeedClient;

const DEFAULT_CACHE_TTL_SECONDS: usize = 60;
const USDC_ISSUER: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
/// Cache TTL for Horizon strict-send path results (30 seconds as per issue #2550).
const SLIPPAGE_CACHE_TTL_SECS: u64 = 30;

// ── Horizon strict-send slippage cache ───────────────────────────────────────
// A simple in-process cache keyed by (source_asset, destination_asset, amount_bucket).
// The amount bucket rounds down to the nearest 100 to avoid excessive cache misses
// while still capturing the depth-dependence of slippage.

struct SlippageEntry {
    slippage_bps: f64,
    fetched_at: Instant,
}

static SLIPPAGE_CACHE: LazyLock<Mutex<HashMap<String, SlippageEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn slippage_cache_key(source: &str, destination: &str, amount: f64) -> String {
    // Bucket by rounding to nearest 100 units.
    let bucket = ((amount / 100.0).floor() as u64) * 100;
    format!("{source}/{destination}/{bucket}")
}

/// Query Horizon `/paths/strict-send` and return the actual slippage in bps.
/// Returns `None` when no path exists or the network call fails.
async fn horizon_slippage_bps(
    source_asset: &str,
    destination_asset: &str,
    source_amount: f64,
    mid_market_rate: f64,
) -> Option<f64> {
    let cache_key = slippage_cache_key(source_asset, destination_asset, source_amount);

    // Check cache first.
    {
        if let Ok(cache) = SLIPPAGE_CACHE.lock() {
            if let Some(entry) = cache.get(&cache_key) {
                if entry.fetched_at.elapsed().as_secs() < SLIPPAGE_CACHE_TTL_SECS {
                    return Some(entry.slippage_bps);
                }
            }
        }
    }

    let horizon_url = std::env::var("STELLAR_HORIZON_URL_MAINNET")
        .or_else(|_| std::env::var("STELLAR_HORIZON_URL_TESTNET"))
        .unwrap_or_else(|_| "https://horizon.stellar.org".to_string());

    // Build asset query params for Horizon.  Assets are either "native" (XLM)
    // or "credit_alphanum4"/"credit_alphanum12" with an issuer.  We encode them
    // as the simpler `asset_type=native` / `asset_code=X&asset_issuer=Y` form.
    let (src_type, src_code, src_issuer) = encode_horizon_asset(source_asset);
    let (dst_type, dst_code, dst_issuer) = encode_horizon_asset(destination_asset);

    let mut url = format!(
        "{horizon_url}/paths/strict-send?source_amount={source_amount:.7}&source_asset_type={src_type}&destination_asset_type={dst_type}"
    );
    if let Some(code) = src_code {
        url.push_str(&format!("&source_asset_code={code}"));
    }
    if let Some(issuer) = src_issuer {
        url.push_str(&format!("&source_asset_issuer={issuer}"));
    }
    if let Some(code) = dst_code {
        url.push_str(&format!("&destination_asset_code={code}"));
    }
    if let Some(issuer) = dst_issuer {
        url.push_str(&format!("&destination_asset_issuer={issuer}"));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;

    // Horizon returns `_embedded.records[].destination_amount` for the best path.
    let best_path_dest: f64 = resp["_embedded"]["records"]
        .as_array()
        .and_then(|records| records.first())
        .and_then(|r| r["destination_amount"].as_str())
        .and_then(|s| s.parse().ok())?;

    let mid_market_dest = source_amount * mid_market_rate;
    if mid_market_dest <= 0.0 || !mid_market_dest.is_finite() {
        return None;
    }

    // Clamp slippage to [0, 200] bps.
    let slippage_bps = ((mid_market_dest - best_path_dest) / mid_market_dest * 10_000.0)
        .max(0.0)
        .min(200.0);

    // Store in cache.
    if let Ok(mut cache) = SLIPPAGE_CACHE.lock() {
        cache.insert(
            cache_key,
            SlippageEntry {
                slippage_bps,
                fetched_at: Instant::now(),
            },
        );
    }

    Some(slippage_bps)
}

/// Encode an asset identifier (e.g. "USDC" or "XLM" or "NGN:GXYZ...") into
/// Horizon asset type + optional code/issuer parts.
/// Returns `(asset_type, optional_code, optional_issuer)`.
fn encode_horizon_asset(
    asset: &str,
) -> (String, Option<String>, Option<String>) {
    if asset.eq_ignore_ascii_case("XLM") || asset.eq_ignore_ascii_case("XLM:native") {
        return ("native".to_string(), None, None);
    }
    if let Some((code, issuer)) = asset.split_once(':') {
        let asset_type = if code.len() <= 4 {
            "credit_alphanum4"
        } else {
            "credit_alphanum12"
        };
        return (
            asset_type.to_string(),
            Some(code.to_uppercase()),
            Some(issuer.to_uppercase()),
        );
    }
    // No issuer supplied — return the type so the caller can build the param.
    let asset_type = if asset.len() <= 4 {
        "credit_alphanum4"
    } else {
        "credit_alphanum12"
    };
    (
        asset_type.to_string(),
        Some(asset.to_uppercase()),
        None,
    )
}

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

    const fn as_key(self) -> &'static str {
        match self {
            Self::StellarDex => "stellar_dex",
            Self::AnchorDirect => "anchor_direct",
            Self::LiquidityPool => "liquidity_pool",
        }
    }

    const fn label(self) -> &'static str {
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
    #[schema(example = 1_550_000.0)]
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


#[derive(Debug, Clone, Copy)]
struct RouteFees {
    spread_bps: f64,
    service_fee_bps: f64,
    network_fee_source: f64,
    /// Kept for reference; actual slippage is now sourced from Horizon strict-send paths.
    #[allow(dead_code)]
    slippage_base_bps: f64,
    /// Kept for reference; actual slippage is now sourced from Horizon strict-send paths.
    #[allow(dead_code)]
    slippage_per_10k_bps: f64,
}

impl RouteFees {
    const fn for_route(route: PaymentRoute) -> Self {
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
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Cost Calculator"
)]
#[allow(clippy::too_many_lines)]
pub async fn estimate_costs(
    State(price_feed): State<Arc<PriceFeedClient>>,
    request_headers: HeaderMap,
    Json(request): Json<CostCalculationRequest>,
) -> Result<Response, ApiError> {
    let source_currency = normalize_currency(&request.source_currency);
    let destination_currency = normalize_currency(&request.destination_currency);

    if source_currency.is_empty() || destination_currency.is_empty() {
        return Err(ApiError::bad_request(
            "MISSING_CURRENCIES",
            "source_currency and destination_currency are required",
        ));
    }

    if request.source_amount <= 0.0 || !request.source_amount.is_finite() {
        return Err(ApiError::bad_request(
            "INVALID_SOURCE_AMOUNT",
            "source_amount must be a positive number",
        ));
    }

    if let Some(destination_amount) = request.destination_amount {
        if destination_amount <= 0.0 || !destination_amount.is_finite() {
            return Err(ApiError::bad_request(
                "INVALID_DESTINATION_AMOUNT",
                "destination_amount must be a positive number when provided",
            ));
        }
    }

    let requested_routes = request.routes.unwrap_or_else(PaymentRoute::default_routes);
    let unique_routes: Vec<PaymentRoute> = requested_routes
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if unique_routes.is_empty() {
        return Err(ApiError::bad_request(
            "NO_ROUTES",
            "at least one route is required",
        ));
    }

    let source_usd_rate = resolve_usd_rate(&price_feed, &source_currency)
        .await
        .map_err(|e| ApiError::bad_request("UNSUPPORTED_SOURCE_CURRENCY", e.to_string()))?;

    let destination_usd_rate = resolve_usd_rate(&price_feed, &destination_currency)
        .await
        .map_err(|e| ApiError::bad_request("UNSUPPORTED_DESTINATION_CURRENCY", e.to_string()))?;

    if destination_usd_rate <= 0.0 {
        return Err(ApiError::bad_request(
            "INVALID_DESTINATION_RATE",
            "destination USD rate is invalid",
        ));
    }

    let mid_market_rate = source_usd_rate / destination_usd_rate;

    let mut route_estimates: Vec<RouteEstimate> =
        futures::future::join_all(unique_routes.into_iter().map(|route| {
            estimate_route(
                route,
                &source_currency,
                &destination_currency,
                request.source_amount,
                request.destination_amount,
                mid_market_rate,
            )
        }))
        .await;

    route_estimates.sort_by(|a, b| {
        a.breakdown
            .total_fees_source
            .partial_cmp(&b.breakdown.total_fees_source)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let Some(best_route) = route_estimates.first().cloned() else {
        return Err(ApiError::internal("ROUTE_ESTIMATION_FAILED", "failed to estimate routes"));
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

    cached_json_response(
        &request_headers,
        &resource_key,
        &response,
        DEFAULT_CACHE_TTL_SECONDS,
    )
    .map_err(|e| ApiError::internal("SERIALIZATION_FAILED", format!("failed to serialize response: {e}")))
}

async fn estimate_route(
    route: PaymentRoute,
    source_asset: &str,
    destination_asset: &str,
    source_amount: f64,
    destination_target: Option<f64>,
    mid_market_rate: f64,
) -> RouteEstimate {
    let fees = RouteFees::for_route(route);

    // Query Horizon strict-send paths for actual slippage. Fall back to the
    // static multipliers only when no path is returned (thin corridor or
    // network unavailable). A None from Horizon means the route is effectively
    // unavailable and slippage is unknown; we use 200 bps (the cap) so it
    // sorts last.
    let slippage_bps = horizon_slippage_bps(source_asset, destination_asset, source_amount, mid_market_rate)
        .await
        .unwrap_or(200.0);

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

async fn resolve_usd_rate(
    price_feed: &PriceFeedClient,
    currency: &str,
) -> Result<f64, DomainError> {
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

        return Err(DomainError::UnsupportedCurrency(currency.to_string()));
    }

    if let Some(asset_id) = price_feed_asset_id(currency) {
        if let Ok(rate) = price_feed.get_price(asset_id).await {
            if rate > 0.0 && rate.is_finite() {
                return Ok(rate);
            }
        }
    }

    fallback_usd_rate(currency)
        .ok_or_else(|| DomainError::UnsupportedCurrency(currency.to_string()))
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

    #[tokio::test]
    async fn test_estimate_route_produces_positive_fees() {
        let estimate = estimate_route(
            PaymentRoute::StellarDex,
            "USDC",
            "NGN",
            1_000.0,
            Some(1_500_000.0),
            1_538.0,
        )
        .await;
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
