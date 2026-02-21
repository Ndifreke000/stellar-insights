use std::sync::Arc;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio::time::{Duration, interval};
use tracing::{info, warn, error};

// ─── Data Structures ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset_type: AssetType,
    pub code: Option<String>,
    pub issuer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Native,
    CreditAlphanum4,
    CreditAlphanum12,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Native => "native",
            AssetType::CreditAlphanum4 => "credit_alphanum4",
            AssetType::CreditAlphanum12 => "credit_alphanum12",
        }
    }
}

impl Asset {
    pub fn native() -> Self {
        Self { asset_type: AssetType::Native, code: None, issuer: None }
    }

    pub fn credit(code: impl Into<String>, issuer: impl Into<String>) -> Self {
        let code = code.into();
        let asset_type = if code.len() <= 4 {
            AssetType::CreditAlphanum4
        } else {
            AssetType::CreditAlphanum12
        };
        Self { asset_type, code: Some(code), issuer: Some(issuer.into()) }
    }

    pub fn pair_key(&self, counter: &Asset) -> String {
        let base = match &self.code {
            Some(c) => c.clone(),
            None => "XLM".to_string(),
        };
        let ctr = match &counter.code {
            Some(c) => c.clone(),
            None => "XLM".to_string(),
        };
        format!("{base}/{ctr}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityMetrics {
    pub total_bid_volume: f64,
    pub total_ask_volume: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread: f64,
    pub spread_bps: f64,
    pub mid_price: f64,
    pub depth_at_1_percent: f64,
    pub depth_at_5_percent: f64,
    pub fetched_at: i64,
}

// ─── Horizon API Response Types ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HorizonPriceLevel {
    price: String,
    amount: String,
}

#[derive(Debug, Deserialize)]
struct HorizonOrderBook {
    bids: Vec<HorizonPriceLevel>,
    asks: Vec<HorizonPriceLevel>,
}

// ─── Cache ───────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use tokio::sync::RwLock;
use std::time::Instant;

struct CacheEntry {
    metrics: LiquidityMetrics,
    order_book: OrderBook,
    cached_at: Instant,
}

pub struct CacheManager {
    entries: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl CacheManager {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    async fn get(&self, key: &str) -> Option<(LiquidityMetrics, OrderBook)> {
        let map = self.entries.read().await;
        if let Some(entry) = map.get(key) {
            if entry.cached_at.elapsed() < self.ttl {
                return Some((entry.metrics.clone(), entry.order_book.clone()));
            }
        }
        None
    }

    async fn set(&self, key: String, metrics: LiquidityMetrics, order_book: OrderBook) {
        let mut map = self.entries.write().await;
        map.insert(key, CacheEntry { metrics, order_book, cached_at: Instant::now() });
    }
}

// ─── DEX Aggregator ──────────────────────────────────────────────────────────

pub struct DexAggregator {
    http: Client,
    horizon_url: String,
    cache: Arc<CacheManager>,
}

impl DexAggregator {
    pub fn new(horizon_url: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            http: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            horizon_url: horizon_url.into(),
            cache: Arc::new(CacheManager::new(300)), // 5 min TTL
        })
    }

    /// Fetch order book from Horizon and return raw struct.
    pub async fn get_order_book(&self, base: &Asset, counter: &Asset, limit: u32) -> Result<OrderBook> {
        let mut params: Vec<(&str, String)> = vec![
            ("selling_asset_type", base.asset_type.as_str().to_string()),
            ("buying_asset_type",  counter.asset_type.as_str().to_string()),
            ("limit", limit.to_string()),
        ];
        if let (Some(code), Some(issuer)) = (&base.code, &base.issuer) {
            params.push(("selling_asset_code", code.clone()));
            params.push(("selling_asset_issuer", issuer.clone()));
        }
        if let (Some(code), Some(issuer)) = (&counter.code, &counter.issuer) {
            params.push(("buying_asset_code", code.clone()));
            params.push(("buying_asset_issuer", issuer.clone()));
        }

        let url = format!("{}/order_book", self.horizon_url);
        let resp = self.http.get(&url)
            .query(&params)
            .send()
            .await
            .context("Failed to fetch order book from Horizon")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Horizon returned {status}: {body}");
        }

        let raw: HorizonOrderBook = resp.json().await
            .context("Failed to parse Horizon order book response")?;

        let parse_levels = |levels: Vec<HorizonPriceLevel>| -> Vec<PriceLevel> {
            levels.into_iter().filter_map(|l| {
                let price = l.price.parse::<f64>().ok()?;
                let amount = l.amount.parse::<f64>().ok()?;
                Some(PriceLevel { price, amount })
            }).collect()
        };

        Ok(OrderBook {
            bids: parse_levels(raw.bids),
            asks: parse_levels(raw.asks),
        })
    }

    /// Calculate liquidity metrics from an order book.
    pub fn calculate_metrics(order_book: &OrderBook) -> Option<LiquidityMetrics> {
        if order_book.bids.is_empty() && order_book.asks.is_empty() {
            return None;
        }

        let best_bid = order_book.bids.first().map(|l| l.price).unwrap_or(0.0);
        let best_ask = order_book.asks.first().map(|l| l.price).unwrap_or(0.0);
        let mid_price = if best_bid > 0.0 && best_ask > 0.0 {
            (best_bid + best_ask) / 2.0
        } else {
            best_bid.max(best_ask)
        };

        let spread = if best_bid > 0.0 && best_ask > 0.0 { best_ask - best_bid } else { 0.0 };
        let spread_bps = if mid_price > 0.0 { (spread / mid_price) * 10_000.0 } else { 0.0 };

        let total_bid_volume: f64 = order_book.bids.iter().map(|l| l.amount).sum();
        let total_ask_volume: f64 = order_book.asks.iter().map(|l| l.amount).sum();

        let depth_at_1_percent  = Self::depth_at_impact(order_book, mid_price, 1.0);
        let depth_at_5_percent  = Self::depth_at_impact(order_book, mid_price, 5.0);

        Some(LiquidityMetrics {
            total_bid_volume,
            total_ask_volume,
            best_bid,
            best_ask,
            spread,
            spread_bps,
            mid_price,
            depth_at_1_percent,
            depth_at_5_percent,
            fetched_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Sum ask volumes within `pct` price impact from mid price.
    fn depth_at_impact(order_book: &OrderBook, mid_price: f64, pct: f64) -> f64 {
        if mid_price == 0.0 { return 0.0; }
        let target = mid_price * (1.0 + pct / 100.0);
        order_book.asks.iter()
            .filter(|l| l.price <= target)
            .map(|l| l.amount)
            .sum()
    }

    /// Get cached or fresh liquidity metrics for a corridor.
    pub async fn get_liquidity(&self, base: &Asset, counter: &Asset) -> Result<LiquidityMetrics> {
        let key = base.pair_key(counter);

        if let Some((metrics, _)) = self.cache.get(&key).await {
            return Ok(metrics);
        }

        let order_book = self.get_order_book(base, counter, 200).await?;
        let metrics = Self::calculate_metrics(&order_book)
            .unwrap_or_else(|| LiquidityMetrics {
                total_bid_volume: 0.0,
                total_ask_volume: 0.0,
                best_bid: 0.0,
                best_ask: 0.0,
                spread: 0.0,
                spread_bps: 0.0,
                mid_price: 0.0,
                depth_at_1_percent: 0.0,
                depth_at_5_percent: 0.0,
                fetched_at: chrono::Utc::now().timestamp(),
            });

        self.cache.set(key, metrics.clone(), order_book).await;
        Ok(metrics)
    }

    /// Background job that refreshes top corridors every 5 minutes.
    pub fn spawn_background_refresh(self: Arc<Self>, corridors: Vec<(Asset, Asset)>) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(300));
            loop {
                ticker.tick().await;
                info!("DEX background refresh: refreshing {} corridors", corridors.len());
                for (base, counter) in &corridors {
                    match self.get_order_book(base, counter, 200).await {
                        Ok(ob) => {
                            let key = base.pair_key(counter);
                            if let Some(metrics) = Self::calculate_metrics(&ob) {
                                self.cache.set(key, metrics, ob).await;
                            }
                        }
                        Err(e) => {
                            warn!("Background refresh failed for {}/{}: {e}",
                                base.code.as_deref().unwrap_or("XLM"),
                                counter.code.as_deref().unwrap_or("XLM"));
                        }
                    }
                }
            }
        });
    }
}

// ─── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_order_book() -> OrderBook {
        OrderBook {
            bids: vec![
                PriceLevel { price: 0.99, amount: 500.0 },
                PriceLevel { price: 0.98, amount: 1000.0 },
                PriceLevel { price: 0.95, amount: 2000.0 },
            ],
            asks: vec![
                PriceLevel { price: 1.01, amount: 400.0 },
                PriceLevel { price: 1.02, amount: 800.0 },
                PriceLevel { price: 1.06, amount: 1500.0 },
            ],
        }
    }

    #[test]
    fn test_calculate_metrics_basic() {
        let ob = sample_order_book();
        let m = DexAggregator::calculate_metrics(&ob).unwrap();

        assert!((m.best_bid - 0.99).abs() < 1e-9);
        assert!((m.best_ask - 1.01).abs() < 1e-9);
        assert!((m.mid_price - 1.00).abs() < 1e-9);
        assert!((m.spread - 0.02).abs() < 1e-6);
        // spread_bps = (0.02 / 1.00) * 10000 = 200
        assert!((m.spread_bps - 200.0).abs() < 1e-4);
    }

    #[test]
    fn test_total_volumes() {
        let ob = sample_order_book();
        let m = DexAggregator::calculate_metrics(&ob).unwrap();

        assert!((m.total_bid_volume - 3500.0).abs() < 1e-6);
        assert!((m.total_ask_volume - 2700.0).abs() < 1e-6);
    }

    #[test]
    fn test_depth_at_1_percent() {
        let ob = sample_order_book();
        let m = DexAggregator::calculate_metrics(&ob).unwrap();
        // mid = 1.00, 1% target = 1.01; asks at 1.01 qualify (price <= 1.01)
        assert!((m.depth_at_1_percent - 400.0).abs() < 1e-6);
    }

    #[test]
    fn test_depth_at_5_percent() {
        let ob = sample_order_book();
        let m = DexAggregator::calculate_metrics(&ob).unwrap();
        // mid = 1.00, 5% target = 1.05; asks at 1.01 and 1.02 qualify
        assert!((m.depth_at_5_percent - 1200.0).abs() < 1e-6);
    }

    #[test]
    fn test_empty_order_book_returns_none() {
        let ob = OrderBook { bids: vec![], asks: vec![] };
        assert!(DexAggregator::calculate_metrics(&ob).is_none());
    }

    #[test]
    fn test_asset_pair_key() {
        let base = Asset::credit("USDC", "GA5Z...");
        let counter = Asset::native();
        assert_eq!(base.pair_key(&counter), "USDC/XLM");
    }
}
