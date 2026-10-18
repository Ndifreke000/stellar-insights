use crate::models::corridor::{compute_median, CorridorMetrics, PaymentRecord};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CorridorPayment {
    pub successful: bool,
    pub settlement_latency_ms: Option<i32>,
    pub amount_usd: f64,
}

/// Order book structures for computing liquidity depth
#[derive(Debug, Clone)]
pub struct OrderBookEntry {
    pub price: f64,
    pub amount_usd: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    pub bids: Vec<OrderBookEntry>, // Descending by price
    pub asks: Vec<OrderBookEntry>, // Ascending by price
}

/// Compute total liquidity in USD within a max slippage percent
#[must_use]
pub fn compute_liquidity_depth(order_book: &OrderBookSnapshot, max_slippage_percent: f64) -> f64 {
    if order_book.bids.is_empty() && order_book.asks.is_empty() {
        return 0.0;
    }

    let best_bid = order_book.bids.first().map_or(0.0, |b| b.price);
    let best_ask = order_book.asks.first().map_or(0.0, |a| a.price);
    if best_bid == 0.0 || best_ask == 0.0 {
        return 0.0;
    }

    let mid_price = f64::midpoint(best_bid, best_ask);
    let max_buy_price = mid_price * (1.0 + max_slippage_percent / 100.0);
    let min_sell_price = mid_price * (1.0 - max_slippage_percent / 100.0);

    // Buy-side liquidity (asks within max slippage)
    let buy_liquidity: f64 = order_book
        .asks
        .iter()
        .take_while(|a| a.price <= max_buy_price)
        .map(|a| a.amount_usd)
        .sum();

    // Sell-side liquidity (bids within max slippage)
    let sell_liquidity: f64 = order_book
        .bids
        .iter()
        .take_while(|b| b.price >= min_sell_price)
        .map(|b| b.amount_usd)
        .sum();

    buy_liquidity + sell_liquidity
}

/// USD-denominated reserves of one side of a liquidity pool, used to
/// estimate how much of a payment an AMM pool can absorb before price
/// impact exceeds a given slippage bound (#2667).
///
/// Reserves are expected in the same "simplified: assume both sides
/// equivalent" USD-ish units `LiquidityPoolAnalyzer::sync_pools` already
/// stores them in — this mirrors that existing approximation rather than
/// introducing a new one.
#[derive(Debug, Clone, Copy)]
pub struct AmmPoolReserves {
    pub reserve_a_usd: f64,
    pub reserve_b_usd: f64,
}

/// Estimate the USD liquidity a constant-product AMM pool (x*y=k) can
/// absorb on each side while keeping price impact within
/// `max_slippage_percent`.
///
/// For a constant-product pool, trading `dx` into reserve `x` moves the
/// price by approximately `dx / (x + dx)` (the standard Uniswap-v2-style
/// slippage approximation). Solving for `dx` at a target slippage `s`
/// (fractional) gives `dx = s * x / (1 - s)`. The same is computed for the
/// other side and summed, mirroring how `compute_liquidity_depth` combines
/// buy-side and sell-side order book depth.
#[must_use]
pub fn compute_amm_pool_depth(pool: &AmmPoolReserves, max_slippage_percent: f64) -> f64 {
    let s = (max_slippage_percent / 100.0).clamp(0.0, 0.99);
    if s <= 0.0 || pool.reserve_a_usd <= 0.0 || pool.reserve_b_usd <= 0.0 {
        return 0.0;
    }

    let side_a = s * pool.reserve_a_usd / (1.0 - s);
    let side_b = s * pool.reserve_b_usd / (1.0 - s);
    side_a + side_b
}

/// Combine order book depth and AMM pool depth into one liquidity-depth
/// figure for a corridor (#2667 — AMM pools provide real, on-chain
/// liquidity that was previously ignored entirely by the preflight
/// liquidity check; see `compute_live_corridors` in `api/corridors.rs`,
/// which is where this is actually used for the live preflight path).
#[must_use]
pub fn compute_combined_liquidity_depth(
    order_book: Option<&OrderBookSnapshot>,
    amm_pools: &[AmmPoolReserves],
    max_slippage_percent: f64,
) -> f64 {
    let order_book_depth =
        order_book.map_or(0.0, |ob| compute_liquidity_depth(ob, max_slippage_percent));
    let amm_depth: f64 = amm_pools
        .iter()
        .map(|p| compute_amm_pool_depth(p, max_slippage_percent))
        .sum();
    order_book_depth + amm_depth
}

/// Computes corridor metrics from transactions, calculating average and median settlement latency with optional liquidity depth.
#[must_use]
pub fn compute_corridor_metrics(
    txns: &[CorridorPayment],
    order_book: Option<&OrderBookSnapshot>, // Optional snapshot for liquidity depth
    slippage_percent: f64,                  // e.g., 1.0 = 1% slippage
) -> CorridorMetrics {
    if txns.is_empty() {
        return CorridorMetrics {
            id: uuid::Uuid::nil().to_string(),
            corridor_key: String::new(),
            source_asset_code: String::new(),
            source_asset_issuer: String::new(),
            destination_asset_code: String::new(),
            destination_asset_issuer: String::new(),
            date: chrono::Utc::now(),
            success_rate: 0.0,
            avg_settlement_latency_ms: None,
            median_settlement_latency_ms: None,
            liquidity_depth_usd: 0.0,
            volume_usd: 0.0,
            total_transactions: 0,
            successful_transactions: 0,
            failed_transactions: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
    }

    let total_transactions = txns.len() as i64;
    let mut successful_transactions = 0;
    let mut failed_transactions = 0;
    let mut latency_sum = 0i64;
    let mut latency_values: Vec<i64> = Vec::new();
    let mut volume_usd = 0.0;

    for t in txns {
        if t.successful {
            successful_transactions += 1;
            volume_usd += t.amount_usd.max(0.0);
            if let Some(ms) = t.settlement_latency_ms {
                if ms >= 0 {
                    latency_sum += i64::from(ms);
                    latency_values.push(i64::from(ms));
                }
            }
        } else {
            failed_transactions += 1;
        }
    }

    let success_rate = (successful_transactions as f64 / total_transactions as f64) * 100.0;
    let avg_settlement_latency_ms = if latency_values.is_empty() {
        None
    } else {
        Some((latency_sum / latency_values.len() as i64) as i32)
    };
    let median_settlement_latency_ms = compute_median(&mut latency_values).map(|v| v as i32);

    // Compute liquidity depth using order book snapshot if provided
    let liquidity_depth_usd =
        order_book.map_or(0.0, |ob| compute_liquidity_depth(ob, slippage_percent));

    CorridorMetrics {
        id: uuid::Uuid::nil().to_string(),
        corridor_key: String::new(),
        source_asset_code: String::new(),
        source_asset_issuer: String::new(),
        destination_asset_code: String::new(),
        destination_asset_issuer: String::new(),
        date: chrono::Utc::now(),
        total_transactions,
        successful_transactions,
        failed_transactions,
        success_rate,
        volume_usd,
        avg_settlement_latency_ms,
        median_settlement_latency_ms,
        liquidity_depth_usd,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// Computes corridor metrics from payment records, aggregating settlement latency (both average and median) per corridor.
#[must_use]
pub fn compute_metrics_from_payments(payments: &[PaymentRecord]) -> Vec<CorridorMetrics> {
    let mut corridor_map: HashMap<String, Vec<&PaymentRecord>> = HashMap::new();

    // Group payments by corridor
    for payment in payments {
        let corridor = payment.get_corridor();
        let key = corridor.to_string_key();
        corridor_map.entry(key).or_default().push(payment);
    }

    let mut results = Vec::new();

    for (key, corridor_payments) in corridor_map {
        // We need to parse the key back to get asset details, or just take from the first payment
        // Taking from first payment is safer/easier if we trust the grouping
        let first = corridor_payments[0];
        let corridor = first.get_corridor();

        let total_transactions = corridor_payments.len() as i64;
        let mut successful_transactions = 0;
        let mut failed_transactions = 0;
        let mut volume_usd = 0.0;
        let mut latency_sum = 0i64;
        let mut latency_values: Vec<i64> = Vec::new();

        for p in &corridor_payments {
            if p.successful {
                successful_transactions += 1;
                volume_usd += p.amount; // Assuming amount is already USD or normalized.
                                        // Compute settlement latency from submission/confirmation times
                if let Some(latency_ms) = p.settlement_latency_ms() {
                    // Filter out negative latencies which might be due to data synchronization issues
                    if latency_ms >= 0 {
                        latency_sum += latency_ms;
                        latency_values.push(latency_ms);
                    }
                }
            } else {
                failed_transactions += 1;
            }
        }

        let success_rate = if total_transactions > 0 {
            (successful_transactions as f64 / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        let avg_settlement_latency_ms = if latency_values.is_empty() {
            None
        } else {
            Some((latency_sum / latency_values.len() as i64) as i32)
        };
        let median_settlement_latency_ms = compute_median(&mut latency_values).map(|v| v as i32);

        results.push(CorridorMetrics {
            id: uuid::Uuid::new_v4().to_string(), // Generate new ID for this snapshot
            corridor_key: key,
            source_asset_code: corridor.source_asset_code,
            source_asset_issuer: corridor.source_asset_issuer,
            destination_asset_code: corridor.destination_asset_code,
            destination_asset_issuer: corridor.destination_asset_issuer,
            date: chrono::Utc::now(),
            total_transactions,
            successful_transactions,
            failed_transactions,
            success_rate,
            volume_usd,
            avg_settlement_latency_ms,
            median_settlement_latency_ms,
            liquidity_depth_usd: 0.0, // Needs order book
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }

    results
}

/// Filter payments by time window and compute metrics
#[must_use]
pub fn compute_metrics_by_window(
    payments: &[PaymentRecord],
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> Vec<CorridorMetrics> {
    let filtered: Vec<PaymentRecord> = payments
        .iter()
        .filter(|p| p.timestamp >= start && p.timestamp <= end)
        .cloned()
        .collect();

    compute_metrics_from_payments(&filtered)
}

/// Excludes payments that touch a merged Stellar account from corridor stats.
///
/// Why this matters (#2664): once an account is merged away via
/// `account_merge`, it no longer represents an independent counterparty —
/// its source identity has vanished and its destination side received a
/// one-off balance dump that has nothing to do with normal corridor flow.
/// Counting those payments in `success_rate`/`volume_usd` distorts the
/// corridor's real health (e.g. a burst of merges into one destination
/// account can make a corridor look far busier or more reliable than it
/// actually is).
///
/// `merged_accounts` should contain every account that appears as either
/// `source_account` or `destination_account` on an `account_merges` row
/// (see `AccountMergeDetector::get_merged_account_set`). A payment is
/// dropped if *either* side is in that set.
///
/// Returns the retained payments plus how many were excluded, so callers
/// can log/alert on unexpectedly high exclusion counts.
#[must_use]
pub fn filter_merged_accounts(
    payments: &[PaymentRecord],
    merged_accounts: &std::collections::HashSet<String>,
) -> (Vec<PaymentRecord>, usize) {
    if merged_accounts.is_empty() {
        return (payments.to_vec(), 0);
    }

    let mut excluded = 0usize;
    let retained: Vec<PaymentRecord> = payments
        .iter()
        .filter(|p| {
            let touches_merged = merged_accounts.contains(&p.source_account)
                || merged_accounts.contains(&p.destination_account);
            if touches_merged {
                excluded += 1;
            }
            !touches_merged
        })
        .cloned()
        .collect();

    (retained, excluded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::corridor::PaymentRecord;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_payment_record(
        source_code: &str,
        dest_code: &str,
        amount: f64,
        successful: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> PaymentRecord {
        PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: source_code.to_string(),
            source_asset_issuer: "issuer1".to_string(),
            destination_asset_code: dest_code.to_string(),
            destination_asset_issuer: "issuer2".to_string(),
            amount,
            successful,
            timestamp,
            submission_time: None,
            confirmation_time: None,
            source_account: "GSOURCETESTACCOUNT".to_string(),
            destination_account: "GDESTTESTACCOUNT".to_string(),
        }
    }

    fn create_test_payment_with_latency(
        source_code: &str,
        dest_code: &str,
        amount: f64,
        successful: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
        latency_ms: i64,
    ) -> PaymentRecord {
        let submission = timestamp - chrono::Duration::milliseconds(latency_ms);
        PaymentRecord {
            id: Uuid::new_v4(),
            source_asset_code: source_code.to_string(),
            source_asset_issuer: "issuer1".to_string(),
            destination_asset_code: dest_code.to_string(),
            destination_asset_issuer: "issuer2".to_string(),
            amount,
            successful,
            timestamp,
            submission_time: Some(submission),
            confirmation_time: Some(timestamp),
            source_account: "GSOURCETESTACCOUNT".to_string(),
            destination_account: "GDESTTESTACCOUNT".to_string(),
        }
    }

    #[test]
    fn test_compute_metrics_from_payments() {
        let payments = vec![
            create_test_payment_record("USDC", "EURC", 100.0, true, Utc::now()),
            create_test_payment_record("USDC", "EURC", 50.0, true, Utc::now()),
            create_test_payment_record("BTC", "ETH", 10.0, false, Utc::now()),
        ];

        let metrics = compute_metrics_from_payments(&payments);
        assert_eq!(metrics.len(), 2);

        // Find USDC -> EURC metrics
        let usdc_metrics = metrics
            .iter()
            .find(|m| m.source_asset_code == "EURC" || m.source_asset_code == "USDC") // Normalized
            .expect("Should find USDC/EURC metrics");

        assert_eq!(usdc_metrics.total_transactions, 2);
        assert_eq!(usdc_metrics.successful_transactions, 2);
        assert_eq!(usdc_metrics.success_rate, 100.0);
        assert_eq!(usdc_metrics.volume_usd, 150.0);
    }

    #[test]
    fn test_compute_metrics_by_window() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        let two_days_ago = now - chrono::Duration::days(2);

        let payments = vec![
            create_test_payment_record("USDC", "EURC", 100.0, true, now),
            create_test_payment_record("USDC", "EURC", 50.0, true, yesterday), // Should include
            create_test_payment_record("USDC", "EURC", 50.0, true, two_days_ago), // Should exclude
        ];

        // Window: 30 hours ago to now (covers yesterday and today, excludes 2 days ago)
        let start = now - chrono::Duration::hours(30);
        let end = now + chrono::Duration::seconds(10); // buffer

        let metrics = compute_metrics_by_window(&payments, start, end);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_transactions, 2);
    }

    #[test]
    fn test_compute_corridor_metrics_basic() {
        let txns = vec![
            CorridorPayment {
                successful: true,
                settlement_latency_ms: Some(1000),
                amount_usd: 100.0,
            },
            CorridorPayment {
                successful: true,
                settlement_latency_ms: Some(3000),
                amount_usd: 200.0,
            },
            CorridorPayment {
                successful: false,
                settlement_latency_ms: None,
                amount_usd: 50.0,
            },
        ];

        let order_book = OrderBookSnapshot {
            bids: vec![
                OrderBookEntry {
                    price: 99.0,
                    amount_usd: 150.0,
                },
                OrderBookEntry {
                    price: 98.5,
                    amount_usd: 100.0,
                },
            ],
            asks: vec![
                OrderBookEntry {
                    price: 101.0,
                    amount_usd: 200.0,
                },
                OrderBookEntry {
                    price: 102.0,
                    amount_usd: 50.0,
                },
            ],
        };

        let metrics = compute_corridor_metrics(&txns, Some(&order_book), 1.0); // 1% slippage
        assert_eq!(metrics.total_transactions, 3);
        assert_eq!(metrics.successful_transactions, 2);
        assert_eq!(metrics.failed_transactions, 1);
        assert_eq!(metrics.success_rate, (2.0 / 3.0) * 100.0);
        assert_eq!(metrics.avg_settlement_latency_ms, Some(2000));
        assert_eq!(metrics.median_settlement_latency_ms, Some(2000)); // Median of [1000, 3000]
        assert!(metrics.liquidity_depth_usd > 0.0); // computed from order book
    }

    #[test]
    fn test_compute_corridor_metrics_empty() {
        let metrics = compute_corridor_metrics(&[], None, 1.0);
        assert_eq!(metrics.total_transactions, 0);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.avg_settlement_latency_ms, None);
        assert_eq!(metrics.median_settlement_latency_ms, None);
        assert_eq!(metrics.liquidity_depth_usd, 0.0);
    }

    #[test]
    fn test_compute_corridor_metrics_all_failed() {
        let txns = vec![
            CorridorPayment {
                successful: false,
                settlement_latency_ms: None,
                amount_usd: 10.0,
            },
            CorridorPayment {
                successful: false,
                settlement_latency_ms: None,
                amount_usd: 20.0,
            },
        ];
        let metrics = compute_corridor_metrics(&txns, None, 1.0);
        assert_eq!(metrics.success_rate, 0.0);
        assert_eq!(metrics.avg_settlement_latency_ms, None);
        assert_eq!(metrics.median_settlement_latency_ms, None);
        assert_eq!(metrics.liquidity_depth_usd, 0.0);
    }

    #[test]
    fn test_median_latency_from_payments() {
        let now = Utc::now();
        let payments = vec![
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 1000),
            create_test_payment_with_latency("USDC", "EURC", 200.0, true, now, 2000),
            create_test_payment_with_latency("USDC", "EURC", 150.0, true, now, 3000),
        ];

        let metrics = compute_metrics_from_payments(&payments);
        assert_eq!(metrics.len(), 1);

        let m = &metrics[0];
        assert_eq!(m.total_transactions, 3);
        assert_eq!(m.successful_transactions, 3);
        assert_eq!(m.avg_settlement_latency_ms, Some(2000)); // (1000 + 2000 + 3000) / 3
        assert_eq!(m.median_settlement_latency_ms, Some(2000)); // Median of [1000, 2000, 3000]
    }

    #[test]
    fn test_median_latency_even_count() {
        let now = Utc::now();
        let payments = vec![
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 1000),
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 2000),
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 3000),
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 4000),
        ];

        let metrics = compute_metrics_from_payments(&payments);
        assert_eq!(metrics.len(), 1);

        let m = &metrics[0];
        assert_eq!(m.avg_settlement_latency_ms, Some(2500)); // (1000 + 2000 + 3000 + 4000) / 4
        assert_eq!(m.median_settlement_latency_ms, Some(2500)); // (2000 + 3000) / 2
    }

    #[test]
    fn test_median_latency_with_negative_values() {
        let now = Utc::now();
        let payments = vec![
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 1000),
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, -500), // Should be filtered
            create_test_payment_with_latency("USDC", "EURC", 100.0, true, now, 3000),
        ];

        let metrics = compute_metrics_from_payments(&payments);
        assert_eq!(metrics.len(), 1);

        let m = &metrics[0];
        // Only 1000 and 3000 should be considered
        assert_eq!(m.avg_settlement_latency_ms, Some(2000)); // (1000 + 3000) / 2
        assert_eq!(m.median_settlement_latency_ms, Some(2000)); // Median of [1000, 3000]
    }

    fn payment_with_accounts(source: &str, destination: &str, amount: f64) -> PaymentRecord {
        let mut p = create_test_payment_record("USDC", "EURC", amount, true, Utc::now());
        p.source_account = source.to_string();
        p.destination_account = destination.to_string();
        p
    }

    /// Fixture mirroring issue #2664: a corridor has three normal payments
    /// plus one that landed in an account right before it merged away. The
    /// merged destination should be excluded so it can't inflate
    /// `volume_usd`/`success_rate` for the corridor.
    #[test]
    fn test_filter_merged_accounts_excludes_merge_fixture() {
        let normal_a = payment_with_accounts("GALICE", "GBOB", 100.0);
        let normal_b = payment_with_accounts("GALICE", "GBOB", 200.0);
        let merged_destination = payment_with_accounts("GALICE", "GMERGEDAWAY", 5_000.0);
        let merged_source = payment_with_accounts("GMERGEDAWAY", "GCAROL", 10.0);

        let payments = vec![
            normal_a.clone(),
            normal_b.clone(),
            merged_destination,
            merged_source,
        ];

        let mut merged_accounts = std::collections::HashSet::new();
        merged_accounts.insert("GMERGEDAWAY".to_string());

        let (retained, excluded) = filter_merged_accounts(&payments, &merged_accounts);

        assert_eq!(excluded, 2);
        assert_eq!(retained.len(), 2);
        assert!(retained.iter().all(|p| p.source_account != "GMERGEDAWAY"
            && p.destination_account != "GMERGEDAWAY"));

        // Confirms the distortion the issue describes: without filtering, the
        // 5,000 XLM merge dump would dominate corridor volume.
        let metrics_unfiltered = compute_metrics_from_payments(&payments);
        let metrics_filtered = compute_metrics_from_payments(&retained);
        assert!(metrics_unfiltered[0].volume_usd > metrics_filtered[0].volume_usd);
        assert!((metrics_filtered[0].volume_usd - 300.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_filter_merged_accounts_noop_when_set_empty() {
        let payments = vec![payment_with_accounts("GALICE", "GBOB", 100.0)];
        let (retained, excluded) = filter_merged_accounts(&payments, &std::collections::HashSet::new());
        assert_eq!(excluded, 0);
        assert_eq!(retained.len(), 1);
    }

    // ── #2667: AMM pool depth combined with order book depth ───────────────

    #[test]
    fn test_compute_amm_pool_depth_zero_without_liquidity() {
        let empty_pool = AmmPoolReserves {
            reserve_a_usd: 0.0,
            reserve_b_usd: 0.0,
        };
        assert_eq!(compute_amm_pool_depth(&empty_pool, 1.0), 0.0);
    }

    #[test]
    fn test_compute_amm_pool_depth_scales_with_slippage_tolerance() {
        let pool = AmmPoolReserves {
            reserve_a_usd: 100_000.0,
            reserve_b_usd: 100_000.0,
        };

        let tight = compute_amm_pool_depth(&pool, 0.1);
        let loose = compute_amm_pool_depth(&pool, 2.0);

        // A larger slippage tolerance means the trade can move the price
        // further, so more of the pool's reserves are usable.
        assert!(loose > tight);

        // At 1% slippage: dx = 0.01 * 100_000 / 0.99 per side, summed over
        // both sides.
        let one_pct = compute_amm_pool_depth(&pool, 1.0);
        let expected_side = 0.01 * 100_000.0 / 0.99;
        assert!((one_pct - 2.0 * expected_side).abs() < 1e-6);
    }

    #[test]
    fn test_compute_combined_liquidity_depth_adds_amm_to_order_book() {
        let order_book = OrderBookSnapshot {
            bids: vec![OrderBookEntry {
                price: 99.0,
                amount_usd: 1_000.0,
            }],
            asks: vec![OrderBookEntry {
                price: 101.0,
                amount_usd: 1_000.0,
            }],
        };
        let order_book_only = compute_combined_liquidity_depth(Some(&order_book), &[], 5.0);
        assert!(order_book_only > 0.0);

        let pool = AmmPoolReserves {
            reserve_a_usd: 50_000.0,
            reserve_b_usd: 50_000.0,
        };
        let combined = compute_combined_liquidity_depth(Some(&order_book), &[pool], 5.0);

        // A pool with real depth must make the combined figure strictly
        // larger than the order book alone — this is the scenario the issue
        // asks to be tested ("Test where AMM provides depth").
        assert!(combined > order_book_only);
        assert!((combined - (order_book_only + compute_amm_pool_depth(&pool, 5.0))).abs() < 1e-6);
    }

    #[test]
    fn test_compute_combined_liquidity_depth_amm_only() {
        // No order book at all (e.g. an asset with no DEX offers) but a
        // funded AMM pool should still produce a positive liquidity figure
        // instead of silently reporting zero liquidity.
        let pool = AmmPoolReserves {
            reserve_a_usd: 20_000.0,
            reserve_b_usd: 20_000.0,
        };
        let depth = compute_combined_liquidity_depth(None, &[pool], 1.0);
        assert!(depth > 0.0);
    }
}
