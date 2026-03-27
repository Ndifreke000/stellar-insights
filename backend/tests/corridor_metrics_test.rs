//! Tests for corridor calculation logic – `compute_liquidity_depth`,
//! `compute_corridor_metrics`, and datetime-filtered analytics.
//!
//! These extend the inline `#[cfg(test)]` suite inside `services/analytics.rs`
//! with additional edge cases and boundary conditions.

use stellar_insights_backend::services::analytics::{
    compute_corridor_metrics, compute_liquidity_depth, OrderBookEntry, OrderBookSnapshot,
    CorridorTransaction,
};

// ── compute_liquidity_depth ───────────────────────────────────────────────────

#[test]
fn test_liquidity_depth_empty_order_book_returns_zero() {
    let book = OrderBookSnapshot {
        bids: vec![],
        asks: vec![],
    };
    assert_eq!(compute_liquidity_depth(&book, 1.0), 0.0);
}

#[test]
fn test_liquidity_depth_bids_only_returns_zero() {
    let book = OrderBookSnapshot {
        bids: vec![OrderBookEntry {
            price: 1.0,
            amount_usd: 1000.0,
        }],
        asks: vec![],
    };
    // asks is empty → best_ask == 0 → return 0.0
    assert_eq!(compute_liquidity_depth(&book, 1.0), 0.0);
}

#[test]
fn test_liquidity_depth_asks_only_returns_zero() {
    let book = OrderBookSnapshot {
        bids: vec![],
        asks: vec![OrderBookEntry {
            price: 1.0,
            amount_usd: 1000.0,
        }],
    };
    // bids is empty → best_bid == 0 → return 0.0
    assert_eq!(compute_liquidity_depth(&book, 1.0), 0.0);
}

#[test]
fn test_liquidity_depth_tight_spread_both_sides_within_slippage() {
    // mid = (0.99 + 1.01) / 2 = 1.00
    // 1% slippage: max_buy = 1.01, min_sell = 0.99
    let book = OrderBookSnapshot {
        bids: vec![OrderBookEntry {
            price: 0.99,
            amount_usd: 500.0,
        }],
        asks: vec![OrderBookEntry {
            price: 1.01,
            amount_usd: 600.0,
        }],
    };
    let depth = compute_liquidity_depth(&book, 1.0);
    // buy_liquidity (ask within 1.01) = 600, sell_liquidity (bid >= 0.99) = 500
    assert!((depth - 1100.0).abs() < 0.01);
}

#[test]
fn test_liquidity_depth_ask_outside_slippage_excluded() {
    // mid = 1.0; 0.5% slippage → max_buy = 1.005
    // ask at 1.02 is outside that range
    let book = OrderBookSnapshot {
        bids: vec![OrderBookEntry {
            price: 0.995,
            amount_usd: 300.0,
        }],
        asks: vec![OrderBookEntry {
            price: 1.02,
            amount_usd: 800.0,
        }],
    };
    let depth = compute_liquidity_depth(&book, 0.5);
    // ask price 1.02 > max_buy_price (1.0075), so buy_liquidity = 0
    // bid price 0.995 >= min_sell_price (0.9925 = 1.0*(1 - 0.0050)) → sell = 300
    assert!(depth < 800.0, "distant ask should be excluded");
}

#[test]
fn test_liquidity_depth_multiple_ask_levels_partially_included() {
    // Asks at 1.001, 1.005, 1.015; mid = 1.0; 1% slippage → max_buy = 1.01
    let book = OrderBookSnapshot {
        bids: vec![OrderBookEntry {
            price: 0.999,
            amount_usd: 200.0,
        }],
        asks: vec![
            OrderBookEntry {
                price: 1.001,
                amount_usd: 100.0,
            },
            OrderBookEntry {
                price: 1.005,
                amount_usd: 200.0,
            },
            OrderBookEntry {
                price: 1.015, // excluded
                amount_usd: 999.0,
            },
        ],
    };
    let depth = compute_liquidity_depth(&book, 1.0);
    // Only asks ≤ 1.01 count: 100 + 200 = 300
    // All bids ≥ 0.99: 200
    assert!((depth - 500.0).abs() < 1.0);
}

#[test]
fn test_liquidity_depth_zero_slippage_excludes_all() {
    // 0% slippage: max_buy == mid_price, min_sell == mid_price
    // No ask price can be <= mid (it's always above), no bid >= mid (always below)
    let book = OrderBookSnapshot {
        bids: vec![OrderBookEntry {
            price: 0.99,
            amount_usd: 500.0,
        }],
        asks: vec![OrderBookEntry {
            price: 1.01,
            amount_usd: 500.0,
        }],
    };
    let depth = compute_liquidity_depth(&book, 0.0);
    assert_eq!(depth, 0.0);
}

// ── compute_corridor_metrics – settlement latency edge cases ──────────────────

fn successful_txn(latency_ms: Option<i32>, amount: f64) -> CorridorTransaction {
    CorridorTransaction {
        successful: true,
        settlement_latency_ms: latency_ms,
        amount_usd: amount,
    }
}

fn failed_txn() -> CorridorTransaction {
    CorridorTransaction {
        successful: false,
        settlement_latency_ms: None,
        amount_usd: 50.0,
    }
}

#[test]
fn test_metrics_single_success_no_latency() {
    let txns = vec![successful_txn(None, 100.0)];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.total_transactions, 1);
    assert_eq!(m.successful_transactions, 1);
    assert_eq!(m.failed_transactions, 0);
    assert_eq!(m.success_rate, 100.0);
    assert_eq!(m.avg_settlement_latency_ms, None);
    assert_eq!(m.median_settlement_latency_ms, None);
    assert_eq!(m.volume_usd, 100.0);
}

#[test]
fn test_metrics_volume_counts_successful_only() {
    let txns = vec![
        successful_txn(Some(500), 200.0),
        failed_txn(), // amount_usd: 50
    ];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    // Only successful payments should contribute to volume_usd
    assert_eq!(m.volume_usd, 200.0);
    assert_eq!(m.total_transactions, 2);
    assert_eq!(m.successful_transactions, 1);
    assert_eq!(m.failed_transactions, 1);
}

#[test]
fn test_metrics_negative_latency_is_ignored() {
    let txns = vec![
        successful_txn(Some(1000), 100.0),
        successful_txn(Some(-200), 100.0), // invalid, should be filtered
        successful_txn(Some(3000), 100.0),
    ];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    // avg of 1000 + 3000 / 2 = 2000
    assert_eq!(m.avg_settlement_latency_ms, Some(2000));
    assert_eq!(m.median_settlement_latency_ms, Some(2000));
}

#[test]
fn test_metrics_single_latency_value() {
    let txns = vec![successful_txn(Some(1234), 50.0)];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.avg_settlement_latency_ms, Some(1234));
    assert_eq!(m.median_settlement_latency_ms, Some(1234));
}

#[test]
fn test_metrics_even_number_latency_median() {
    // Median of [1000, 2000, 3000, 4000] = (2000 + 3000) / 2 = 2500
    let txns = vec![
        successful_txn(Some(1000), 10.0),
        successful_txn(Some(2000), 10.0),
        successful_txn(Some(3000), 10.0),
        successful_txn(Some(4000), 10.0),
    ];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.avg_settlement_latency_ms, Some(2500));
    assert_eq!(m.median_settlement_latency_ms, Some(2500));
}

#[test]
fn test_metrics_odd_latency_median_middle_value() {
    // Median of [1000, 2000, 3000] = 2000
    let txns = vec![
        successful_txn(Some(3000), 10.0),
        successful_txn(Some(1000), 10.0),
        successful_txn(Some(2000), 10.0),
    ];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.median_settlement_latency_ms, Some(2000));
}

#[test]
fn test_metrics_all_failed_no_volume_no_latency() {
    let txns = vec![failed_txn(), failed_txn(), failed_txn()];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.success_rate, 0.0);
    assert_eq!(m.volume_usd, 0.0);
    assert_eq!(m.avg_settlement_latency_ms, None);
    assert_eq!(m.median_settlement_latency_ms, None);
    assert_eq!(m.successful_transactions, 0);
    assert_eq!(m.failed_transactions, 3);
}

#[test]
fn test_metrics_success_rate_is_percentage() {
    let txns = vec![
        successful_txn(Some(500), 100.0),
        successful_txn(Some(600), 100.0),
        failed_txn(),
    ];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    let expected_rate = (2.0 / 3.0) * 100.0;
    assert!((m.success_rate - expected_rate).abs() < 0.001);
}

#[test]
fn test_metrics_large_dataset_100_transactions() {
    let mut txns: Vec<CorridorTransaction> = (0..80).map(|_| successful_txn(Some(500), 10.0)).collect();
    txns.extend((0..20).map(|_| failed_txn()));

    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.total_transactions, 100);
    assert_eq!(m.successful_transactions, 80);
    assert_eq!(m.failed_transactions, 20);
    assert!((m.success_rate - 80.0).abs() < 0.001);
    assert_eq!(m.volume_usd, 800.0); // 80 * 10.0
}

#[test]
fn test_metrics_with_order_book_yields_positive_depth() {
    let txns = vec![successful_txn(Some(500), 100.0)];
    let order_book = OrderBookSnapshot {
        bids: vec![OrderBookEntry {
            price: 0.99,
            amount_usd: 5000.0,
        }],
        asks: vec![OrderBookEntry {
            price: 1.01,
            amount_usd: 5000.0,
        }],
    };
    let m = compute_corridor_metrics(&txns, Some(&order_book), 1.0);
    assert!(m.liquidity_depth_usd > 0.0);
}

#[test]
fn test_metrics_without_order_book_depth_is_zero() {
    let txns = vec![successful_txn(Some(500), 100.0)];
    let m = compute_corridor_metrics(&txns, None, 1.0);
    assert_eq!(m.liquidity_depth_usd, 0.0);
}
