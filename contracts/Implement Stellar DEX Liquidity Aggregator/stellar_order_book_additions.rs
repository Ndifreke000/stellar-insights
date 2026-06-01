// ─── Additions to backend/src/rpc/stellar.rs ────────────────────────────────
//
// Add the following methods to your existing StellarRpcClient implementation.
// Also re-export the DexAggregator so callers can reach it through this module.

use crate::services::dex_aggregator::{Asset, DexAggregator, LiquidityMetrics, OrderBook};
use anyhow::Result;
use std::sync::Arc;

impl StellarRpcClient {
    // Convenience wrappers that delegate to the DexAggregator ----------------

    /// Return the raw order book for a given asset pair.
    pub async fn get_order_book(
        &self,
        base: &Asset,
        counter: &Asset,
        limit: u32,
    ) -> Result<OrderBook> {
        self.dex_aggregator.get_order_book(base, counter, limit).await
    }

    /// Return cached or freshly computed liquidity metrics for a corridor.
    pub async fn get_liquidity_metrics(
        &self,
        base: &Asset,
        counter: &Asset,
    ) -> Result<LiquidityMetrics> {
        self.dex_aggregator.get_liquidity(base, counter).await
    }
}

// ─── Wire-up ─────────────────────────────────────────────────────────────────
//
// Inside StellarRpcClient::new(), add the dex_aggregator field:
//
//   pub struct StellarRpcClient {
//       /* existing fields ... */
//       pub dex_aggregator: Arc<DexAggregator>,
//   }
//
//   impl StellarRpcClient {
//       pub fn new(horizon_url: &str) -> Self {
//           Self {
//               /* existing init ... */
//               dex_aggregator: DexAggregator::new(horizon_url),
//           }
//       }
//   }
//
// Then call spawn_background_refresh() once at server startup, e.g. in main.rs:
//
//   let top_corridors = vec![
//       (Asset::credit("USDC", GA5Z_ISSUER), Asset::native()),
//       (Asset::credit("BTC",  BTC_ISSUER),  Asset::native()),
//       // ... top 20
//   ];
//   rpc_client.dex_aggregator.clone().spawn_background_refresh(top_corridors);
