use anyhow::{Context, Result};
use async_lock::RwLock;
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Configuration for price feed service
#[derive(Debug, Clone)]
pub struct PriceFeedConfig {
    /// Provider to use (coingecko, coinmarketcap)
    pub provider: String,
    /// API key (optional for `CoinGecko` free tier, required for `CoinMarketCap`)
    pub api_key: Option<String>,
    /// Cache TTL in seconds (default: 900 = 15 minutes)
    pub cache_ttl_seconds: u64,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Maximum age (seconds) a price may have before it is considered stale
    /// and treated as unknown for USD conversions (default: 3600 = 1 hour)
    pub max_staleness_seconds: u64,
}

impl Default for PriceFeedConfig {
    fn default() -> Self {
        Self {
            provider: "coingecko".to_string(),
            api_key: None,
            cache_ttl_seconds: 900, // 15 minutes
            request_timeout_seconds: 10,
            max_staleness_seconds: 3600, // 1 hour
        }
    }
}

impl PriceFeedConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            provider: std::env::var("PRICE_FEED_PROVIDER")
                .unwrap_or_else(|_| "coingecko".to_string()),
            api_key: std::env::var("PRICE_FEED_API_KEY").ok(),
            cache_ttl_seconds: std::env::var("PRICE_FEED_CACHE_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(900),
            request_timeout_seconds: std::env::var("PRICE_FEED_REQUEST_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            max_staleness_seconds: std::env::var("PRICE_FEED_MAX_STALENESS_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
        }
    }
}

/// Cached price entry
#[derive(Debug, Clone)]
struct CachedPrice {
    price_usd: f64,
    timestamp: Instant,
}

/// Trait for price feed providers
#[async_trait::async_trait]
pub trait PriceFeedProvider: Send + Sync {
    /// Fetch price for a single asset
    async fn fetch_price(&self, asset_id: &str) -> Result<f64>;

    /// Fetch prices for multiple assets
    async fn fetch_prices(&self, asset_ids: &[String]) -> Result<HashMap<String, f64>>;

    /// Get provider name
    fn name(&self) -> &str;
}

/// `CoinGecko` provider implementation
pub struct CoinGeckoProvider {
    client: Client,
    api_key: Option<String>,
}

impl CoinGeckoProvider {
    #[must_use]
    pub fn new(api_key: Option<String>, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }
}

#[derive(Debug, Deserialize)]
struct CoinGeckoSimplePrice {
    usd: f64,
}

#[async_trait::async_trait]
impl PriceFeedProvider for CoinGeckoProvider {
    async fn fetch_price(&self, asset_id: &str) -> Result<f64> {
        let url = if let Some(api_key) = &self.api_key {
            format!(
                "https://pro-api.coingecko.com/api/v3/simple/price?ids={asset_id}&vs_currencies=usd&x_cg_pro_api_key={api_key}"
            )
        } else {
            format!(
                "https://api.coingecko.com/api/v3/simple/price?ids={asset_id}&vs_currencies=usd"
            )
        };

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to CoinGecko")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error: {status} - {body}");
        }

        let prices: HashMap<String, CoinGeckoSimplePrice> = response
            .json()
            .await
            .context("Failed to parse CoinGecko response")?;

        prices
            .get(asset_id)
            .map(|p| p.usd)
            .ok_or_else(|| anyhow::anyhow!("Price not found for asset: {asset_id}"))
    }

    async fn fetch_prices(&self, asset_ids: &[String]) -> Result<HashMap<String, f64>> {
        if asset_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids = asset_ids.join(",");
        let url = if let Some(api_key) = &self.api_key {
            format!(
                "https://pro-api.coingecko.com/api/v3/simple/price?ids={ids}&vs_currencies=usd&x_cg_pro_api_key={api_key}"
            )
        } else {
            format!("https://api.coingecko.com/api/v3/simple/price?ids={ids}&vs_currencies=usd")
        };

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to CoinGecko")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CoinGecko API error: {status} - {body}");
        }

        let prices: HashMap<String, CoinGeckoSimplePrice> = response
            .json()
            .await
            .context("Failed to parse CoinGecko response")?;

        Ok(prices.into_iter().map(|(k, v)| (k, v.usd)).collect())
    }

    fn name(&self) -> &'static str {
        "CoinGecko"
    }
}

/// Compute the median of a set of price samples.
///
/// Returns `None` when no valid (finite, positive) samples are provided.
/// For an even number of samples the mean of the two middle values is used.
#[must_use]
pub fn median_price(samples: &[f64]) -> Option<f64> {
    let mut valid: Vec<f64> = samples
        .iter()
        .copied()
        .filter(|p| p.is_finite() && *p > 0.0)
        .collect();

    if valid.is_empty() {
        return None;
    }

    valid.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = valid.len() / 2;
    if valid.len() % 2 == 0 {
        Some((valid[mid - 1] + valid[mid]) / 2.0)
    } else {
        Some(valid[mid])
    }
}

/// Main price feed client with caching
pub struct PriceFeedClient {
    provider: Arc<dyn PriceFeedProvider>,
    /// Optional secondary provider used to cross-check / aggregate prices.
    secondary_provider: Option<Arc<dyn PriceFeedProvider>>,
    cache: Arc<RwLock<HashMap<String, CachedPrice>>>,
    asset_mapping: Arc<HashMap<String, String>>,
    config: PriceFeedConfig,
}

impl PriceFeedClient {
    /// Create a new price feed client
    pub fn new(config: PriceFeedConfig, asset_mapping: HashMap<String, String>) -> Self {
        let timeout = Duration::from_secs(config.request_timeout_seconds);

        let provider: Arc<dyn PriceFeedProvider> = if config.provider.as_str() == "coingecko" {
            Arc::new(CoinGeckoProvider::new(config.api_key.clone(), timeout))
        } else {
            warn!(
                "Unknown provider '{}', defaulting to CoinGecko",
                config.provider
            );
            Arc::new(CoinGeckoProvider::new(config.api_key.clone(), timeout))
        };

        info!(
            "Initialized price feed client with provider: {}",
            provider.name()
        );

        Self {
            provider,
            secondary_provider: None,
            cache: Arc::new(RwLock::new(HashMap::new())),
            asset_mapping: Arc::new(asset_mapping),
            config,
        }
    }

    /// Attach a secondary provider so prices are aggregated via the median of
    /// both sources. This makes single-source distortions less likely.
    #[must_use]
    pub fn with_secondary_provider(mut self, provider: Arc<dyn PriceFeedProvider>) -> Self {
        self.secondary_provider = Some(provider);
        self
    }

    /// Returns true when the cached entry is older than the configured
    /// staleness threshold and must be treated as unknown.
    fn is_stale(&self, cached: &CachedPrice) -> bool {
        cached.timestamp.elapsed().as_secs() >= self.config.max_staleness_seconds
    }

    /// Get price for a Stellar asset, returns USD value.
    ///
    /// Prices are aggregated across the configured providers using the median.
    /// If the only available price is stale (older than
    /// `max_staleness_seconds`), an error is returned so callers can treat the
    /// USD conversion as unknown.
    pub async fn get_price(&self, stellar_asset: &str) -> Result<f64> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(stellar_asset) {
                let age = cached.timestamp.elapsed();
                if age.as_secs() < self.config.cache_ttl_seconds {
                    if self.is_stale(cached) {
                        warn!(
                            "Cached price for {} is stale (age {}s), treating as unknown",
                            stellar_asset,
                            age.as_secs()
                        );
                        anyhow::bail!("Price for {stellar_asset} is stale");
                    }
                    debug!("Cache hit for {}: ${}", stellar_asset, cached.price_usd);
                    return Ok(cached.price_usd);
                }
            }
        }

        // Map Stellar asset to provider asset ID
        let asset_id = self
            .asset_mapping
            .get(stellar_asset)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for asset: {stellar_asset}"))?;

        // Fetch from primary provider
        debug!("Fetching price for {} ({})", stellar_asset, asset_id);
        let mut samples: Vec<f64> = Vec::new();
        match self.provider.fetch_price(asset_id).await {
            Ok(price) => samples.push(price),
            Err(e) => {
                error!("Failed to fetch price for {}: {}", stellar_asset, e);
            }
        }

        // Fetch from secondary provider (if configured) for median aggregation
        if let Some(secondary) = &self.secondary_provider {
            match secondary.fetch_price(asset_id).await {
                Ok(price) => samples.push(price),
                Err(e) => {
                    warn!(
                        "Secondary provider {} failed for {}: {}",
                        secondary.name(),
                        stellar_asset,
                        e
                    );
                }
            }
        }

        if let Some(price) = median_price(&samples) {
            // Update cache
            let mut cache = self.cache.write().await;
            cache.insert(
                stellar_asset.to_string(),
                CachedPrice {
                    price_usd: price,
                    timestamp: Instant::now(),
                },
            );
            info!("Fetched price for {}: ${}", stellar_asset, price);
            return Ok(price);
        }

        // Try to return stale cache data as fallback, but only if it is not
        // itself stale — otherwise the price is unknown.
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(stellar_asset) {
            if self.is_stale(cached) {
                warn!(
                    "Only stale cached price available for {}, treating as unknown",
                    stellar_asset
                );
                anyhow::bail!("Price for {stellar_asset} is stale");
            }
            warn!(
                "Using cached price for {} after fetch failure: ${}",
                stellar_asset,
                cached.price_usd
            );
            return Ok(cached.price_usd);
        }

        anyhow::bail!("Failed to fetch price for {stellar_asset} from all providers")
    }

    /// Get price for a Stellar asset, returning `None` when the price is
    /// unknown (missing mapping, all providers failed, or stale).
    ///
    /// This is the preferred entry point for USD conversions in preflight so
    /// that stale prices surface as `unknown` liquidity checks rather than
    /// silently distorting amounts.
    pub async fn get_price_opt(&self, stellar_asset: &str) -> Option<f64> {
        match self.get_price(stellar_asset).await {
            Ok(price) => Some(price),
            Err(e) => {
                debug!("Price unknown for {}: {}", stellar_asset, e);
                None
            }
        }
    }

    /// Get prices for multiple Stellar assets, returning USD values.
    ///
    /// Assets whose price is unknown (including stale prices) are omitted from
    /// the result so callers can treat them as unknown.
    pub async fn get_prices(&self, stellar_assets: &[String]) -> Result<HashMap<String, f64>> {
        let mut result = HashMap::new();
        for asset in stellar_assets {
            if let Some(price) = self.get_price_opt(asset).await {
                result.insert(asset.clone(), price);
            }
        }
        Ok(result)
    }

    /// Clear the price cache (useful for testing)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total = cache.len();
        let fresh = cache
            .values()
            .filter(|c| c.timestamp.elapsed().as_secs() < self.config.cache_ttl_seconds)
            .count();
        (total, fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_number_of_samples() {
        assert_eq!(median_price(&[1.0, 3.0, 2.0]), Some(2.0));
    }

    #[test]
    fn median_even_number_of_samples() {
        assert_eq!(median_price(&[1.0, 2.0, 3.0, 4.0]), Some(2.5));
    }

    #[test]
    fn median_ignores_invalid_samples() {
        assert_eq!(median_price(&[f64::NAN, -1.0, 0.0, 5.0, 7.0]), Some(6.0));
    }

    #[test]
    fn median_empty_returns_none() {
        assert_eq!(median_price(&[]), None);
        assert_eq!(median_price(&[f64::NAN, -2.0]), None);
    }

    #[test]
    fn median_single_sample() {
        assert_eq!(median_price(&[42.0]), Some(42.0));
    }

    #[test]
    fn stale_cache_is_detected() {
        let config = PriceFeedConfig {
            max_staleness_seconds: 60,
            ..PriceFeedConfig::default()
        };
        let client = PriceFeedClient::new(config, HashMap::new());

        let fresh = CachedPrice {
            price_usd: 1.0,
            timestamp: Instant::now(),
        };
        assert!(!client.is_stale(&fresh));

        let stale = CachedPrice {
            price_usd: 1.0,
            timestamp: Instant::now() - Duration::from_secs(120),
        };
        assert!(client.is_stale(&stale));
    }

    #[tokio::test]
    async fn stale_cached_price_is_unknown() {
        let config = PriceFeedConfig {
            cache_ttl_seconds: 3600,
            max_staleness_seconds: 60,
            ..PriceFeedConfig::default()
        };
        let mut mapping = HashMap::new();
        mapping.insert("USDC".to_string(), "usd-coin".to_string());
        let client = PriceFeedClient::new(config, mapping);

        {
            let mut cache = client.cache.write().await;
            cache.insert(
                "USDC".to_string(),
                CachedPrice {
                    price_usd: 1.0,
                    timestamp: Instant::now() - Duration::from_secs(120),
                },
            );
        }

        assert!(client.get_price("USDC").await.is_err());
        assert_eq!(client.get_price_opt("USDC").await, None);
    }
}
