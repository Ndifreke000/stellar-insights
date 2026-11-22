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

/// Default k-anonymity threshold: buckets with fewer than this many
/// distinct accounts are suppressed from public aggregates (#2876).
pub const DEFAULT_K_ANONYMITY_THRESHOLD: usize = 5;

/// Default per-request privacy budget (epsilon) for Laplace noise on
/// public aggregates (#2876).
pub const DEFAULT_PRIVACY_EPSILON: f64 = 1.0;

/// Privacy configuration for public analytics aggregates (#2876).
///
/// `k_threshold` controls k-anonymity suppression: any bucket backed by
/// fewer than `k_threshold` distinct accounts is dropped entirely rather
/// than reported. `epsilon` controls optional Laplace noise added to
/// surviving numeric aggregates; `None` disables noise (suppression only).
#[derive(Debug, Clone, Copy)]
pub struct PrivacyConfig {
    pub k_threshold: usize,
    pub epsilon: Option<f64>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            k_threshold: DEFAULT_K_ANONYMITY_THRESHOLD,
            epsilon: None,
        }
    }
}

impl PrivacyConfig {
    /// Build a config with k-anonymity suppression and optional Laplace
    /// noise. `epsilon` values that are non-finite or non-positive are
    /// treated as "noise disabled" so a bad config can never weaken
    /// privacy guarantees silently.
    #[must_use]
    pub fn new(k_threshold: usize, epsilon: Option<f64>) -> Self {
        let epsilon = epsilon.filter(|e| e.is_finite() && *e > 0.0);
        Self {
            k_threshold: k_threshold.max(1),
            epsilon,
        }
    }

    /// Whether a bucket with `distinct_accounts` distinct accounts should
    /// be published. Buckets below the k-anonymity threshold are
    /// suppressed.
    #[must_use]
    pub fn is_publishable(&self, distinct_accounts: usize) -> bool {
        distinct_accounts >= self.k_threshold
    }
}

/// Tracks the cumulative privacy budget spent on noisy public aggregates
/// (#2876). Each noisy query consumes `epsilon`; once the budget is
/// exhausted, callers should stop adding noise (or stop serving noisy
/// aggregates) to preserve the differential-privacy guarantee.
#[derive(Debug, Clone)]
pub struct PrivacyBudget {
    total: f64,
    spent: f64,
}

impl PrivacyBudget {
    #[must_use]
    pub fn new(total: f64) -> Self {
        Self {
            total: total.max(0.0),
            spent: 0.0,
        }
    }

    #[must_use]
    pub fn remaining(&self) -> f64 {
        (self.total - self.spent).max(0.0)
    }

    /// Attempt to charge `epsilon` against the budget. Returns `true` if
    /// the charge succeeded (budget available) and `false` if it would
    /// exceed the remaining budget, in which case nothing is charged.
    pub fn try_charge(&mut self, epsilon: f64) -> bool {
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return false;
        }
        if epsilon > self.remaining() {
            return false;
        }
        self.spent += epsilon;
        true
    }
}

/// Draw a sample from the Laplace distribution with mean 0 and scale
/// `sensitivity / epsilon` using inverse-CDF sampling. Returns 0.0 when
/// `epsilon` is non-positive or non-finite (noise disabled).
#[must_use]
pub fn laplace_noise(sensitivity: f64, epsilon: f64, uniform: f64) -> f64 {
    if !epsilon.is_finite() || epsilon <= 0.0 || sensitivity <= 0.0 {
        return 0.0;
    }
    let scale = sensitivity / epsilon;
    // Map uniform in [0, 1) to (-0.5, 0.5] to avoid ln(0).
    let u = uniform.clamp(f64::EPSILON, 1.0 - f64::EPSILON) - 0.5;
    -scale * u.signum() * (1.0 - 2.0 * u.abs()).ln()
}

/// Apply k-anonymity suppression and optional Laplace noise to a single
/// numeric aggregate. Returns `None` when the bucket is suppressed.
///
/// `sensitivity` is the L1 sensitivity of the aggregate (e.g. the max
/// contribution of a single account). `uniform` is a caller-supplied
/// uniform sample in [0, 1) so the function stays deterministic and
/// testable without an RNG dependency.
#[must_use]
pub fn privatize_aggregate(
    value: f64,
    distinct_accounts: usize,
    config: &PrivacyConfig,
    sensitivity: f64,
    uniform: f64,
) -> Option<f64> {
    if !config.is_publishable(distinct_accounts) {
        return None;
    }
    match config.epsilon {
        Some(eps) => Some(value + laplace_noise(sensitivity, eps, uniform)),
        None => Some(value),
    }
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

        let mut successful = 0i64;
        let mut failed = 0i64;
        let mut volume = 0.0;
        let mut latencies: Vec<i64> = Vec::new();

        for p in &corridor_payments {
            if p.successful {
                successful += 1;
                volume += p.amount_usd.max(0.0);
                if let Some(ms) = p.settlement_latency_ms {
                    if ms >= 0 {
                        latencies.push(i64::from(ms));
                    }
                }
            } else {
                failed += 1;
            }
        }

        let total = successful + failed;
        let success_rate = if total > 0 {
            (successful as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        let avg_latency = if latencies.is_empty() {
            None
        } else {
            Some((latencies.iter().sum::<i64>() / latencies.len() as i64) as i32)
        };
        let median_latency = compute_median(&mut latencies).map(|v| v as i32);

        results.push(CorridorMetrics {
            id: uuid::Uuid::nil().to_string(),
            corridor_key: key,
            source_asset_code: corridor.source_asset_code.clone(),
            source_asset_issuer: corridor.source_asset_issuer.clone(),
            destination_asset_code: corridor.destination_asset_code.clone(),
            destination_asset_issuer: corridor.destination_asset_issuer.clone(),
            date: chrono::Utc::now(),
            total_transactions: total,
            successful_transactions: successful,
            failed_transactions: failed,
            success_rate,
            volume_usd: volume,
            avg_settlement_latency_ms: avg_latency,
            median_settlement_latency_ms: median_latency,
            liquidity_depth_usd: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }

    results
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn buckets_below_k_are_suppressed() {
        let config = PrivacyConfig::new(5, None);
        assert!(privatize_aggregate(100.0, 4, &config, 1.0, 0.5).is_none());
        assert!(privatize_aggregate(100.0, 5, &config, 1.0, 0.5).is_some());
    }

    #[test]
    fn suppression_ignores_noise_setting() {
        let config = PrivacyConfig::new(3, Some(1.0));
        assert!(privatize_aggregate(42.0, 2, &config, 1.0, 0.1).is_none());
    }

    #[test]
    fn no_noise_when_epsilon_disabled() {
        let config = PrivacyConfig::new(1, None);
        let out = privatize_aggregate(42.0, 10, &config, 1.0, 0.9).unwrap();
        assert!((out - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn laplace_noise_is_zero_for_invalid_epsilon() {
        assert_eq!(laplace_noise(1.0, 0.0, 0.5), 0.0);
        assert_eq!(laplace_noise(1.0, f64::NAN, 0.5), 0.0);
    }

    #[test]
    fn privacy_budget_tracks_spend() {
        let mut budget = PrivacyBudget::new(2.0);
        assert!(budget.try_charge(1.0));
        assert!(budget.try_charge(1.0));
        assert!(!budget.try_charge(1.0));
        assert_eq!(budget.remaining(), 0.0);
    }

    #[test]
    fn privacy_budget_rejects_invalid_charge() {
        let mut budget = PrivacyBudget::new(1.0);
        assert!(!budget.try_charge(-1.0));
        assert!(!budget.try_charge(f64::INFINITY));
        assert_eq!(budget.remaining(), 1.0);
    }
}
