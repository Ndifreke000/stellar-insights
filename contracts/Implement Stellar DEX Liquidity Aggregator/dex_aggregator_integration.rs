// backend/tests/dex_aggregator_integration.rs
//
// Run with:  cargo test --test dex_aggregator_integration -- --nocapture
//
// These tests hit the real Horizon testnet, so they require an internet
// connection. They are gated behind an env-var so CI can skip them:
//   RUN_INTEGRATION_TESTS=1 cargo test --test dex_aggregator_integration

use std::sync::Arc;
use stellar_liquidity_backend::services::dex_aggregator::{Asset, DexAggregator};

const HORIZON_TESTNET: &str = "https://horizon-testnet.stellar.org";

// Well-known testnet USDC issuer
const USDC_ISSUER_TESTNET: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";

fn should_run() -> bool {
    std::env::var("RUN_INTEGRATION_TESTS").map(|v| v == "1").unwrap_or(false)
}

#[tokio::test]
async fn test_fetch_order_book_native_usdc() {
    if !should_run() {
        println!("Skipping integration test (set RUN_INTEGRATION_TESTS=1 to enable)");
        return;
    }

    let agg = DexAggregator::new(HORIZON_TESTNET);
    let base    = Asset::credit("USDC", USDC_ISSUER_TESTNET);
    let counter = Asset::native();

    let ob = agg.get_order_book(&base, &counter, 20).await
        .expect("Order book fetch should succeed");

    println!("Bids: {:?}", ob.bids.len());
    println!("Asks: {:?}", ob.asks.len());

    // The testnet might have an empty order book, but the call must not error.
    assert!(ob.bids.len() <= 20);
    assert!(ob.asks.len() <= 20);
}

#[tokio::test]
async fn test_liquidity_metrics_end_to_end() {
    if !should_run() {
        return;
    }

    let agg = DexAggregator::new(HORIZON_TESTNET);
    let base    = Asset::credit("USDC", USDC_ISSUER_TESTNET);
    let counter = Asset::native();

    let metrics = agg.get_liquidity(&base, &counter).await
        .expect("Liquidity metrics should succeed");

    println!("Metrics: {metrics:#?}");

    assert!(metrics.spread_bps >= 0.0);
    assert!(metrics.depth_at_1_percent >= 0.0);
    assert!(metrics.depth_at_5_percent >= metrics.depth_at_1_percent);
}

#[tokio::test]
async fn test_cache_hit() {
    if !should_run() {
        return;
    }

    let agg = DexAggregator::new(HORIZON_TESTNET);
    let base    = Asset::credit("USDC", USDC_ISSUER_TESTNET);
    let counter = Asset::native();

    // First call populates cache
    let m1 = agg.get_liquidity(&base, &counter).await.unwrap();
    // Second call should hit cache (same fetched_at timestamp)
    let m2 = agg.get_liquidity(&base, &counter).await.unwrap();

    assert_eq!(m1.fetched_at, m2.fetched_at, "Second call should be served from cache");
}
