use crate::models::corridor::{compute_median, CorridorMetrics, PaymentRecord};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CorridorTransaction {
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
pub fn compute_liquidity_depth(order_book: &OrderBookSnapshot, max_slippage_percent: f64) -> f64 {
    if order_book.bids.is_empty() && order_book.asks.is_empty() {
        return 0.0;
    }

    let best_bid = order_book.bids.first().map(|b| b.price).unwrap_or(0.0);
    let best_ask = order_book.asks.first().map(|a| a.price).unwrap_or(0.0);
    if best_bid == 0.0 || best_ask == 0.0 {
        return 0.0;
    }

    let mid_price = (best_bid + best_ask) / 2.0;
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

/// Computes corridor metrics from transactions, calculating average and median settlement latency with optional liquidity depth.
pub fn compute_corridor_metrics(
    txns: &[CorridorTransaction],
    order_book: Option<&OrderBookSnapshot>, // Optional snapshot for liquidity depth
    slippage_percent: f64,                  // e.g., 1.0 = 1% slippage
) -> CorridorMetrics {
    if txns.is_empty() {
        return CorridorMetrics {
            id: uuid::Uuid::nil().to_string(),
            corridor_key: String::new(),
            asset_a_code: String::new(),
            asset_a_issuer: String::new(),
            asset_b_code: String::new(),
            asset_b_issuer: String::new(),
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
                latency_sum += ms as i64;
                latency_values.push(ms as i64);
            }
        } else {
            failed_transactions += 1;
        }
    }

    let success_rate = (successful_transactions as f64 / total_transactions as f64) * 100.0;
    let avg_settlement_latency_ms = if !latency_values.is_empty() {
        Some((latency_sum / latency_values.len() as i64) as i32)
    } else {
        None
    };
    let median_settlement_latency_ms = compute_median(&mut latency_values).map(|v| v as i32);

    // Compute liquidity depth using order book snapshot if provided
    let liquidity_depth_usd = order_book
        .map(|ob| compute_liquidity_depth(ob, slippage_percent))
        .unwrap_or(0.0);

    CorridorMetrics {
        id: uuid::Uuid::nil().to_string(),
        corridor_key: String::new(),
        asset_a_code: String::new(),
        asset_a_issuer: String::new(),
        asset_b_code: String::new(),
        asset_b_issuer: String::new(),
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
                    latency_sum += latency_ms;
                    latency_values.push(latency_ms);
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

        let avg_settlement_latency_ms = if !latency_values.is_empty() {
            Some((latency_sum / latency_values.len() as i64) as i32)
        } else {
            None
        };
        let median_settlement_latency_ms = compute_median(&mut latency_values).map(|v| v as i32);

        results.push(CorridorMetrics {
            id: uuid::Uuid::new_v4().to_string(), // Generate new ID for this snapshot
            corridor_key: key,
            asset_a_code: corridor.asset_a_code,
            asset_a_issuer: corridor.asset_a_issuer,
            asset_b_code: corridor.asset_b_code,
            asset_b_issuer: corridor.asset_b_issuer,
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
            .find(|m| m.asset_a_code == "EURC" || m.asset_a_code == "USDC") // Normalized
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
            CorridorTransaction {
                successful: true,
                settlement_latency_ms: Some(1000),
                amount_usd: 100.0,
            },
            CorridorTransaction {
                successful: true,
                settlement_latency_ms: Some(3000),
                amount_usd: 200.0,
            },
            CorridorTransaction {
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
            CorridorTransaction {
                successful: false,
                settlement_latency_ms: None,
                amount_usd: 10.0,
            },
            CorridorTransaction {
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
}
