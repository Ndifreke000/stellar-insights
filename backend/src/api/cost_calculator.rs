use axum::{extract::State, http::HeaderMap, response::Response, routing::post, Json, Router};
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

// ── World Bank Remittance Prices Worldwide (RPW) benchmarks ──────────────────
// A small, curated snapshot of RPW corridor averages (percent total cost to send
// $200 / $500).  RPW is published quarterly; this table is the ingestion target
// and is refreshed by replacing the entries below.  Each entry maps a sending
// country to a receiving country (ISO-3166 alpha-2) with the average total cost
// as a percentage of the send amount.

#[derive(Debug, Clone, Copy)]
struct RpwBenchmark {
    from_country: &'static str,
    to_country: &'static str,
    /// Average total cost (%) for a $200 send.
    cost_pct_200: f64,
    /// Average total cost (%) for a $500 send.
    cost_pct_500: f64,
}

/// Curated RPW snapshot.  Values are illustrative corridor averages from the
/// World Bank Remittance Prices Worldwide dataset and are used as benchmarks.
static RPW_BENCHMARKS: &[RpwBenchmark] = &[
    RpwBenchmark { from_country: "US", to_country: "NG", cost_pct_200: 5.4, cost_pct_500: 4.1 },
    RpwBenchmark { from_country: "US", to_country: "KE", cost_pct_200: 4.9, cost_pct_500: 3.8 },
    RpwBenchmark { from_country: "US", to_country: "GH", cost_pct_200: 6.1, cost_pct_500: 4.7 },
    RpwBenchmark { from_country: "US", to_country: "PH", cost_pct_200: 4.6, cost_pct_500: 3.5 },
    RpwBenchmark { from_country: "US", to_country: "IN", cost_pct_200: 4.2, cost_pct_500: 3.2 },
    RpwBenchmark { from_country: "US", to_country: "MX", cost_pct_200: 4.8, cost_pct_500: 3.7 },
    RpwBenchmark { from_country: "GB", to_country: "NG", cost_pct_200: 5.9, cost_pct_500: 4.4 },
    RpwBenchmark { from_country: "GB", to_country: "IN", cost_pct_200: 4.5, cost_pct_500: 3.4 },
    RpwBenchmark { from_country: "EU", to_country: "NG", cost_pct_200: 6.3, cost_pct_500: 4.9 },
];

/// Map a currency code to the ISO-3166 country used by RPW corridors.
/// Returns `None` for currencies that are not tied to a single country.
fn currency_to_country(currency: &str) -> Option<&'static str> {
    match currency.to_uppercase().as_str() {
        "USD" | "USDC" => Some("US"),
        "GBP" => Some("GB"),
        "EUR" => Some("EU"),
        "NGN" => Some("NG"),
        "KES" => Some("KE"),
        "GHS" => Some("GH"),
        "PHP" => Some("PH"),
        "INR" => Some("IN"),
        "MXN" => Some("MX"),
        _ => None,
    }
}

/// Look up the RPW benchmark for a currency pair, if one exists.
fn rpw_benchmark(source_currency: &str, destination_currency: &str) -> Option<RpwBenchmark> {
    let from = currency_to_country(source_currency)?;
    let to = currency_to_country(destination_currency)?;
    RPW_BENCHMARKS
        .iter()
        .find(|b| b.from_country == from && b.to_country == to)
        .copied()
}

/// Side-by-side comparison of a Stellar route against the RPW benchmark for a
/// given send amount.  `stellar_cost_pct` is the total cost as a percentage of
/// the send amount; `rpw_cost_pct` is the World Bank benchmark.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RpwComparison {
    pub send_amount_usd: f64,
    pub stellar_cost_pct: f64,
    pub rpw_cost_pct: f64,
    pub stellar_cost_usd: f64,
    pub rpw_cost_usd: f64,
    /// Positive when Stellar is cheaper than the RPW benchmark.
    pub savings_usd: f64,
    pub savings_pct_points: f64,
}

/// Build a comparison for a single send amount, if an RPW benchmark exists.
fn build_rpw_comparison(
    source_currency: &str,
    destination_currency: &str,
    send_amount_usd: f64,
    stellar_cost_pct: f64,
) -> Option<RpwComparison> {
    let benchmark = rpw_benchmark(source_currency, destination_currency)?;
    let rpw_cost_pct = if send_amount_usd <= 200.0 {
        benchmark.cost_pct_200
    } else {
        benchmark.cost_pct_500
    };
    let stellar_cost_usd = send_amount_usd * stellar_cost_pct / 100.0;
    let rpw_cost_usd = send_amount_usd * rpw_cost_pct / 100.0;
    Some(RpwComparison {
        send_amount_usd,
        stellar_cost_pct,
        rpw_cost_pct,
        stellar_cost_usd,
        rpw_cost_usd,
        savings_usd: rpw_cost_usd - stellar_cost_usd,
        savings_pct_points: rpw_cost_pct - stellar_cost_pct,
    })
}

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

    let resp: serde_json::Value = client.get(&url).send().await.ok()?.json().await.ok()?;

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
fn encode_horizon_asset(asset: &str) -> (String, Option<String>, Option<String>) {
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
    (asset_type.to_string(), Some(asset.to_uppercase()), None)
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
    /// Side-by-side comparison against World Bank RPW benchmarks for $200 / $500.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpw_comparison: Option<Vec<RpwComparison>>,
}

#[derive(Debug, Clone, Copy)]
struct RouteFees {
    spread_bps: f64,
    service_fee_bps: f64,
    network_fee_source: f64,
    /// Kept for reference; actual slippage is now sourced from Horizon strict-send paths.
    #[allow(dead_code)]
    slippage_bps: f64,
}

#[cfg(test)]
mod rpw_tests {
    use super::*;

    #[test]
    fn maps_currency_to_country() {
        assert_eq!(currency_to_country("USDC"), Some("US"));
        assert_eq!(currency_to_country("ngn"), Some("NG"));
        assert_eq!(currency_to_country("XLM"), None);
    }

    #[test]
    fn finds_benchmark_for_known_corridor() {
        let b = rpw_benchmark("USDC", "NGN").expect("US->NG benchmark");
        assert_eq!(b.from_country, "US");
        assert_eq!(b.to_country, "NG");
    }

    #[test]
    fn no_benchmark_for_unknown_corridor() {
        assert!(rpw_benchmark("USDC", "XLM").is_none());
    }

    #[test]
    fn comparison_uses_200_bucket_for_small_sends() {
        let cmp = build_rpw_comparison("USDC", "NGN", 200.0, 1.0).expect("comparison");
        assert!((cmp.rpw_cost_pct - 5.4).abs() < 1e-9);
        assert!((cmp.stellar_cost_usd - 2.0).abs() < 1e-9);
        assert!((cmp.rpw_cost_usd - 10.8).abs() < 1e-9);
        assert!(cmp.savings_usd > 0.0);
    }

    #[test]
    fn comparison_uses_500_bucket_for_large_sends() {
        let cmp = build_rpw_comparison("USDC", "NGN", 500.0, 1.0).expect("comparison");
        assert!((cmp.rpw_cost_pct - 4.1).abs() < 1e-9);
        assert!((cmp.rpw_cost_usd - 20.5).abs() < 1e-9);
    }
}
