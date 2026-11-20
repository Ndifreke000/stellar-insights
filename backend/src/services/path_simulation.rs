//! Path simulation comparing Horizon path finding vs simulated execution
//!
//! This module fetches order books and AMM pools for candidate paths, simulates
//! execution, and compares results with Horizon's path finding to report slippage.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{self, Duration};

use crate::cache::CacheManager;
use crate::rpc::horizon::models::{HorizonPath, HorizonPathEntry};
use crate::rpc::stellar::Asset;
use crate::rpc::StellarRpcClient;
use crate::services::amm_simulation::{
    AmmPool, PathSegment, build_path, build_path as amm_build_path,
};

/// Cache key prefix for order books
pub const ORDER_BOOK_CACHE_PREFIX: &str = "order_book:";
/// TTL for order book cache (short: 30 seconds)
pub const ORDER_BOOK_TTL_SECONDS: u64 = 30;

/// Cache key prefix for AMM pools
pub const AMM_POOL_CACHE_PREFIX: &str = "amm_pool:";
/// TTL for AMM pool cache
pub const AMM_POOL_TTL_SECONDS: u64 = 30;

/// Path simulation result
#[derive(Debug, Clone)]
pub struct PathSimulationResult {
    /// Source asset
    pub source_asset: Asset,
    /// Destination asset
    pub destination_asset: Asset,
    /// Source amount
    pub source_amount: f64,
    /// Horizon's destination amount (what Horizon says it will deliver)
    pub horizon_destination_amount: f64,
    /// Simulated destination amount (what our simulation predicts)
    pub simulated_destination_amount: f64,
    /// Price impact difference (simulated - horizon)
    pub price_impact_delta: f64,
    /// Relative difference as percentage
    pub relative_difference: f64,
    /// Path segments used
    pub segments: Vec<PathSegment>,
}

impl PathSimulationResult {
    /// Calculate relative difference as a percentage
    pub fn relative_difference_pct(&self) -> f64 {
        if self.horizon_destination_amount <= 0.0 {
            return 0.0;
        }
        (self.simulated_destination_amount - self.horizon_destination_amount) / self.horizon_destination_amount * 100.0
    }
}

/// Order book cache entry
#[derive(Debug, Clone)]
pub struct OrderBookCacheEntry {
    pub order_book: crate::rpc::horizon::models::OrderBook,
    pub timestamp: i64,
}

/// AMM pool cache entry
#[derive(Debug, Clone)]
pub struct AmmPoolCacheEntry {
    pub pool: AmmPool,
    pub timestamp: i64,
}

/// Path simulation service
pub struct PathSimulationService {
    rpc_client: Arc<StellarRpcClient>,
    cache: Arc<CacheManager>,
}

impl PathSimulationService {
    /// Create a new path simulation service
    pub fn new(rpc_client: Arc<StellarRpcClient>, cache: Arc<CacheManager>) -> Self {
        Self { rpc_client, cache }
    }

    /// Fetch order book with caching
    pub async fn fetch_order_book_cached(
        &self,
        selling: &Asset,
        buying: &Asset,
        limit: u32,
    ) -> Result<crate::rpc::horizon::models::OrderBook, anyhow::Error> {
        let cache_key = format!("{}{}:{}:{}", ORDER_BOOK_CACHE_PREFIX, selling, buying, limit);

        // Try cache first
        if let Some(entry) = self.cache.get::<OrderBookCacheEntry>(&cache_key).await {
            // Check if TTL expired
            let now = chrono::Utc::now().timestamp();
            if now - entry.timestamp < ORDER_BOOK_TTL_SECONDS as i64 {
                return Ok(entry.order_book);
            }
        }

        // Fetch from RPC
        let order_book = self
            .rpc_client
            .fetch_order_book(selling, buying, limit)
            .await?;

        // Cache the result
        let entry = OrderBookCacheEntry {
            order_book: order_book.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.cache
            .set(&cache_key, &entry, Duration::from_secs(ORDER_BOOK_TTL_SECONDS))
            .await;

        Ok(order_book)
    }

    /// Fetch AMM pool with caching
    pub async fn fetch_amm_pool_cached(&self, pool_id: &str) -> Result<AmmPool, anyhow::Error> {
        let cache_key = format!("{}{}", AMM_POOL_CACHE_PREFIX, pool_id);

        // Try cache first
        if let Some(entry) = self.cache.get::<AmmPoolCacheEntry>(&cache_key).await {
            // Check if TTL expired
            let now = chrono::Utc::now().timestamp();
            if now - entry.timestamp < AMM_POOL_TTL_SECONDS as i64 {
                return Ok(entry.pool);
            }
        }

        // For now, return an error - pool data needs to be fetched separately
        anyhow::bail!("AMM pool cache not fully implemented - fetch from liquidity_pools API")
    }

    /// Fetch all order books for path segments
    pub async fn fetch_path_segments(
        &self,
        path: &[HorizonPathEntry],
    ) -> Result<Vec<PathSegment>, anyhow::Error> {
        let mut segments = Vec::new();

        for entry in path {
            let asset = Asset {
                asset_type: entry.asset_type.clone(),
                asset_code: entry.asset_code.clone(),
                asset_issuer: entry.asset_issuer.clone(),
            };

            // For each hop, fetch the order book
            // This is simplified - in production, we'd need to identify the exact pair
            if segments.is_empty() {
                continue;
            }

            let prev = segments.last().unwrap();
            let (selling, buying) = if prev.selling_asset() == &asset {
                (prev.selling_asset(), prev.buying_asset())
            } else {
                (prev.buying_asset(), &asset)
            };

            let order_book = self.fetch_order_book_cached(selling, buying, 10).await?;
            segments.push(PathSegment::OrderBook(order_book));
        }

        Ok(segments)
    }

    /// Simulate a payment path and compare with Horizon
    pub async fn simulate_path(
        &self,
        horizon_path: &HorizonPath,
        source_amount: f64,
    ) -> Result<PathSimulationResult, anyhow::Error> {
        // Parse Horizon path
        let source_amount_f64: f64 = horizon_path.source_amount.parse().unwrap_or(0.0);
        let destination_amount_f64: f64 = horizon_path.destination_amount.parse().unwrap_or(0.0);

        // Build segments for simulation
        let segments = self.fetch_path_segments(&horizon_path.path).await?;

        // Simulate the path
        let path = amm_build_path(
            &horizon_path.try_into()?, // This would need implementation
            &horizon_path.try_into()?, // Destination asset
            &segments,
        );

        let simulated_amount = if let Some(p) = path {
            // We'd need to calculate based on the path
            source_amount * 0.99 // Placeholder - actual calculation would use segments
        } else {
            source_amount
        };

        // Calculate price impact delta
        let price_impact_delta = simulated_amount - destination_amount_f64;
        let relative_difference = if destination_amount_f64 > 0.0 {
            price_impact_delta / destination_amount_f64 * 100.0
        } else {
            0.0
        };

        Ok(PathSimulationResult {
            source_asset: horizon_path.try_into()?, // Source asset
            destination_asset: horizon_path.try_into()?, // Destination asset
            source_amount,
            horizon_destination_amount: destination_amount_f64,
            simulated_destination_amount: simulated_amount,
            price_impact_delta,
            relative_difference,
            segments,
        })
    }

    /// Get slippage estimation for a corridor
    pub async fn get_slippage_estimation(
        &self,
        source_asset: &Asset,
        destination_asset: &Asset,
        source_amount: f64,
    ) -> Result<Option<PathSimulationResult>, anyhow::Error> {
        // Fetch Horizon path
        let source_amount_str = format!("{:.7}", source_amount);
        let horizon_paths = self
            .rpc_client
            .fetch_paths_strict_send(source_asset, &source_amount_str, destination_asset)
            .await?;

        if horizon_paths.is_empty() {
            return Ok(None);
        }

        // Use the first (best) path
        let best_path = &horizon_paths[0];

        // Simulate and compare
        self.simulate_path(best_path, source_amount).await.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_simulation_result_relative_difference() {
        let result = PathSimulationResult {
            source_asset: Asset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            destination_asset: Asset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: None,
            },
            source_amount: 100.0,
            horizon_destination_amount: 95.0,
            simulated_destination_amount: 92.0,
            price_impact_delta: -3.0,
            relative_difference: -3.1578947368421053,
            segments: vec![],
        };

        assert!((result.relative_difference_pct() - (-3.1578947368421053)).abs() < 0.001);
    }
}