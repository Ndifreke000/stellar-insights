// ─── Patch for backend/src/api/corridors_cached.rs ──────────────────────────
//
// 1. Add `liquidity: Option<LiquidityMetrics>` to your CorridorResponse struct.
// 2. Call get_liquidity_metrics() when building corridor responses.
//
// The snippet below shows the added logic; merge it into your existing handler.

use crate::services::dex_aggregator::{Asset, LiquidityMetrics};
use axum::{extract::State, Json};
use anyhow::Result;

/// Extended corridor response that now includes real DEX liquidity data.
#[derive(serde::Serialize)]
pub struct CorridorResponse {
    // ── existing fields ──────────────────────────────────────────────────────
    pub id: String,
    pub base_asset: String,
    pub counter_asset: String,
    pub volume_24h: f64,
    // ... other existing fields ...

    // ── new liquidity fields ─────────────────────────────────────────────────
    pub liquidity: Option<LiquidityMetrics>,
}

/// Helper: enrich a list of corridors with live liquidity data.
/// Runs fetches concurrently; errors are silently mapped to None so one bad
/// corridor never breaks the entire endpoint.
pub async fn enrich_with_liquidity(
    corridors: Vec<RawCorridor>,         // your existing type
    rpc: &crate::rpc::stellar::StellarRpcClient,
) -> Vec<CorridorResponse> {
    use futures::future::join_all;

    let futures = corridors.into_iter().map(|c| async {
        let base    = parse_asset(&c.base_asset_code, &c.base_asset_issuer);
        let counter = parse_asset(&c.counter_asset_code, &c.counter_asset_issuer);

        let liquidity = rpc
            .get_liquidity_metrics(&base, &counter)
            .await
            .ok();

        CorridorResponse {
            id:            c.id,
            base_asset:    c.base_asset_code,
            counter_asset: c.counter_asset_code,
            volume_24h:    c.volume_24h,
            liquidity,
        }
    });

    join_all(futures).await
}

fn parse_asset(code: &str, issuer: &str) -> Asset {
    if code.eq_ignore_ascii_case("XLM") || code.eq_ignore_ascii_case("native") {
        Asset::native()
    } else {
        Asset::credit(code, issuer)
    }
}

// ─── Corridor detail endpoint ─────────────────────────────────────────────────
//
// In your existing `/corridors/{pair}` handler, replace the hardcoded liquidity
// block with:
//
//   let (base, counter) = pair_from_path(&pair)?;
//   let liquidity = state.rpc.get_liquidity_metrics(&base, &counter).await.ok();
//   let order_book = state.rpc.get_order_book(&base, &counter, 50).await.ok();
//
// Then include both in the response JSON.
