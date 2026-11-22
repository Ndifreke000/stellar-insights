//! AMM and order book simulation for slippage estimation
//!
//! This module provides constant-product AMM (Uniswap v2 style) and order-book
//! walk simulation to estimate path slippage for a given payment amount.

use std::collections::HashMap;

use crate::rpc::horizon::models::{OrderBook, OrderBookEntry};
use crate::rpc::stellar::Asset;

/// Represents a path segment that can be either an AMM pool or an order book
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PathSegment {
    /// Constant-product AMM pool (x * y = k)
    Amm(AmmPool),
    /// Order book with bids and asks
    OrderBook(OrderBook),
}

impl PathSegment {
    /// Get the selling asset for this segment
    pub fn selling_asset(&self) -> &Asset {
        match self {
            PathSegment::Amm(pool) => &pool.token0,
            PathSegment::OrderBook(book) => &book.base,
        }
    }

    /// Get the buying asset for this segment
    pub fn buying_asset(&self) -> &Asset {
        match self {
            PathSegment::Amm(pool) => &pool.token1,
            PathSegment::OrderBook(book) => &book.counter,
        }
    }
}

/// Constant-product AMM pool (Uniswap v2 style)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AmmPool {
    pub token0: Asset,
    pub token1: Asset,
    pub reserve0: String, // in stroops
    pub reserve1: String, // in stroops
    pub fee_bp: u32,      // fee in basis points (e.g., 30 for 0.3%)
}

impl AmmPool {
    /// Calculate how much token1 you get for selling token0
    pub fn calculate_output_for_input(
        &self,
        input_amount: f64, // in token0
        token0_is_selling: bool,
    ) -> f64 {
        let reserve_in = if token0_is_selling {
            self.reserve0.parse::<f64>().unwrap_or(0.0)
        } else {
            self.reserve1.parse::<f64>().unwrap_or(0.0)
        };

        let reserve_out = if token0_is_selling {
            self.reserve1.parse::<f64>().unwrap_or(0.0)
        } else {
            self.reserve0.parse::<f64>().unwrap_or(0.0)
        };

        let fee = self.fee_bp as f64 / 10000.0;
        let input_with_fee = input_amount * (1.0 - fee);

        // Constant product formula: (reserve_in + input_with_fee) * (reserve_out - output) = reserve_in * reserve_out
        // output = reserve_out - (reserve_in * reserve_out) / (reserve_in + input_with_fee)
        let output = reserve_out - (reserve_in * reserve_out) / (reserve_in + input_with_fee);

        output.max(0.0)
    }

    /// Calculate price impact for a given input amount
    pub fn calculate_price_impact(
        &self,
        input_amount: f64,
        token0_is_selling: bool,
    ) -> f64 {
        let output = self.calculate_output_for_input(input_amount, token0_is_selling);
        
        if input_amount <= 0.0 {
            return 0.0;
        }

        let effective_price = output / input_amount;
        
        // Compare to spot price (reserve_out / reserve_in)
        let spot_price = if token0_is_selling {
            self.reserve1.parse::<f64>().unwrap_or(0.0) / 
            self.reserve0.parse::<f64>().unwrap_or(1.0)
        } else {
            self.reserve0.parse::<f64>().unwrap_or(0.0) / 
            self.reserve1.parse::<f64>().unwrap_or(1.0)
        };

        if spot_price <= 0.0 {
            return 0.0;
        }

        // Price impact = 1 - (effective_price / spot_price)
        1.0 - (effective_price / spot_price)
    }
}

/// Order book with bids and asks
impl OrderBook {
    /// Walk the order book to find how much can be bought for a given sell amount
    pub fn walk_for_buy_amount(&self, sell_amount: f64) -> (f64, Vec<OrderBookEntry>) {
        let entries = if sell_amount > 0.0 {
            &self.asks
        } else {
            &self.bids
        };

        let mut remaining = sell_amount.abs();
        let mut bought = 0.0;
        let mut used_entries = Vec::new();

        for entry in entries {
            let price: f64 = entry.price.parse().unwrap_or(0.0);
            let amount: f64 = entry.amount.parse().unwrap_or(0.0);

            if remaining <= 0.0 {
                break;
            }

            let trade_amount = amount.min(remaining);
            used_entries.push(entry.clone());

            if sell_amount > 0.0 {
                // Buying with selling asset
                bought += trade_amount * price;
            } else {
                // Selling for buying asset
                bought += trade_amount;
            }

            remaining -= trade_amount;
        }

        (bought, used_entries)
    }

    /// Calculate price impact for a given trade amount
    pub fn calculate_price_impact(&self, trade_amount: f64) -> f64 {
        let (executed_amount, used_entries) = self.walk_for_buy_amount(trade_amount);
        
        if trade_amount <= 0.0 || used_entries.is_empty() {
            return 0.0;
        }

        // Average execution price
        let avg_price = executed_amount / trade_amount.abs();

        // Get the best price (first entry)
        let best_price: f64 = if trade_amount > 0.0 {
            self.asks.first().and_then(|e| e.price.parse().ok()).unwrap_or(0.0)
        } else {
            self.bids.first().and_then(|e| e.price.parse().ok()).unwrap_or(0.0)
        };

        if best_price <= 0.0 {
            return 0.0;
        }

        // Price impact = 1 - (avg_price / best_price)
        1.0 - (avg_price / best_price)
    }
}

/// Represents a complete payment path with multiple segments
#[derive(Debug, Clone)]
pub struct PaymentPath {
    pub segments: Vec<PathSegment>,
    pub source_asset: Asset,
    pub destination_asset: Asset,
}

impl PaymentPath {
    /// Simulate a payment through this path and return the result
    pub fn simulate_payment(&self, source_amount: f64) -> PaymentPathResult {
        let mut current_amount = source_amount;
        let mut total_price_impact = 0.0;
        let mut segment_results = Vec::new();

        for segment in &self.segments {
            let (output, impact) = match segment {
                PathSegment::Amm(pool) => {
                    let is_token0_selling = pool.token0 == self.source_asset;
                    let output = pool.calculate_output_for_input(current_amount, is_token0_selling);
                    let impact = pool.calculate_price_impact(current_amount, is_token0_selling);
                    (output, impact)
                }
                PathSegment::OrderBook(book) => {
                    let (output, _) = book.walk_for_buy_amount(current_amount);
                    let impact = book.calculate_price_impact(current_amount);
                    (output, impact)
                }
            };

            total_price_impact += impact;
            segment_results.push(PathSegmentResult {
                segment: segment.clone(),
                input: current_amount,
                output,
                price_impact: impact,
            });

            current_amount = output;
        }

        PaymentPathResult {
            source_amount,
            destination_amount: current_amount,
            total_price_impact,
            segment_results,
        }
    }
}

/// Result of simulating a payment through a path
#[derive(Debug, Clone)]
pub struct PaymentPathResult {
    pub source_amount: f64,
    pub destination_amount: f64,
    pub total_price_impact: f64,
    pub segment_results: Vec<PathSegmentResult>,
}

/// Result of a single path segment
#[derive(Debug, Clone)]
pub struct PathSegmentResult {
    pub segment: PathSegment,
    pub input: f64,
    pub output: f64,
    pub price_impact: f64,
}

/// Build a path from source to destination using available segments
pub fn build_path(
    source_asset: &Asset,
    destination_asset: &Asset,
    segments: &[PathSegment],
) -> Option<PaymentPath> {
    // Simple greedy path finder - starts from source and follows segments
    // In production, this would use a proper graph traversal algorithm
    
    let mut current_asset = source_asset.clone();
    let mut path_segments = Vec::new();
    let mut visited = std::collections::HashSet::new();

    while current_asset != *destination_asset {
        let found = segments.iter().find(|s| {
            (s.selling_asset() == &current_asset || s.buying_asset() == &current_asset) &&
            !visited.contains(s)
        })?;

        visited.insert(found);

        if found.selling_asset() == &current_asset {
            path_segments.push(found.clone());
            current_asset = found.buying_asset().clone();
        } else if found.buying_asset() == &current_asset {
            // Reverse the segment since we're buying from it
            match found.clone() {
                PathSegment::Amm(mut pool) => {
                    std::mem::swap(&mut pool.token0, &mut pool.token1);
                    std::mem::swap(&mut pool.reserve0, &mut pool.reserve1);
                    path_segments.push(PathSegment::Amm(pool));
                }
                PathSegment::OrderBook(book) => {
                    // Order books have bids/asks, we'd need to reverse the book
                    // For simplicity, skip reverse paths for now
                    return None;
                }
            }
            current_asset = found.selling_asset().clone();
        } else {
            return None;
        }

        if path_segments.len() > 5 {
            // Prevent infinite loops and overly long paths
            return None;
        }
    }

    Some(PaymentPath {
        segments: path_segments,
        source_asset: source_asset.clone(),
        destination_asset: destination_asset.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amm_constant_product() {
        // Pool with 1000 token0 and 1000 token1 (1:1 ratio)
        let pool = AmmPool {
            token0: Asset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            token1: Asset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()),
            },
            reserve0: "10000000000".to_string(), // 10000 XLM in stroops
            reserve1: "10000000000".to_string(), // 10000 USDC in stroops
            fee_bp: 30, // 0.3%
        };

        // Selling 1000 XLM should get slightly less than 1000 USDC due to fees
        let output = pool.calculate_output_for_input(1000.0, true);
        assert!(output > 995.0 && output < 1000.0, "Got {}", output);
    }

    #[test]
    fn test_amm_price_impact() {
        let pool = AmmPool {
            token0: Asset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            token1: Asset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()),
            },
            reserve0: "1000000000000".to_string(), // 100000 XLM
            reserve1: "1000000000000".to_string(), // 100000 USDC
            fee_bp: 30,
        };

        // Small trade should have minimal price impact
        let impact = pool.calculate_price_impact(100.0, true);
        assert!(impact < 0.001, "Small trade impact should be low: {}", impact);

        // Large trade should have higher price impact
        let impact_large = pool.calculate_price_impact(10000.0, true);
        assert!(impact_large > impact, "Large trade should have higher impact");
    }

    #[test]
    fn test_order_book_walk() {
        let book = OrderBook {
            bids: vec![
                OrderBookEntry {
                    price: "100.0".to_string(),
                    amount: "50.0".to_string(),
                },
                OrderBookEntry {
                    price: "99.0".to_string(),
                    amount: "100.0".to_string(),
                },
            ],
            asks: vec![
                OrderBookEntry {
                    price: "101.0".to_string(),
                    amount: "30.0".to_string(),
                },
            ],
            base: Asset {
                asset_type: "native".to_string(),
                asset_code: None,
                asset_issuer: None,
            },
            counter: Asset {
                asset_type: "credit_alphanum4".to_string(),
                asset_code: Some("USDC".to_string()),
                asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()),
            },
        };

        // Buying 40 USDC should use 30 at 101 and 10 at 100
        let (bought, used) = book.walk_for_buy_amount(40.0);
        assert_eq!(used.len(), 2);
        assert!(bought > 3900.0 && bought < 4100.0, "Got {}", bought);
    }
}