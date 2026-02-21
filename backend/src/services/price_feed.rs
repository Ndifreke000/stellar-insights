use anyhow::{Context, Result};
use async_lock::RwLock;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Configuration for price feed service
#[derive(Debug, Clone)]
pub struct PriceFeedConfig {
    /// Provider to use (coingecko, coinmarketcap)
    pub provider: String,
    /// API key (optional for CoinGecko free tier, required for CoinMarketCap)
    pub api_key: Option<String>,
    /// Cache TTL in seconds (default: 900 = 15 minutes)
    pub cache_ttl_seconds: u64,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
}

impl Default for PriceFeedConfig {
    fn default() -> Self {
        Self {
            provider: "coingecko".to_string(),
            api_key: None,
            cache_ttl_seconds: 900, // 15 minutes
            request_timeout_seconds: 10,
        }
    }
}

impl PriceFeedConfig {
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

/// CoinGecko provider implementation
pub struct CoinGeckoProvider {
    client: Client,
    api_key: Option<String>,
}

impl CoinGeckoProvider {
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
                "https://pro-api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&x_cg_pro_api_key={}",
                asset_id, api_key
            )
        } else {
            format!(
                "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
                asset_id
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
            anyhow::bail!("CoinGecko API error: {} - {}", status, body);
        }

        let prices: HashMap<String, CoinGeckoSimplePrice> = response
            .json()
            .await
            .context("Failed to parse CoinGecko response")?;

        prices
            .get(asset_id)
            .map(|p| p.usd)
            .ok_or_else(|| anyhow::anyhow!("Price not found for asset: {}", asset_id))
    }

    async fn fetch_prices(&self, asset_ids: &[String]) -> Result<HashMap<String, f64>> {
        if asset_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids = asset_ids.join(",");
        let url = if let Some(api_key) = &self.api_key {
            format!(
                "https://pro-api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&x_cg_pro_api_key={}",
                ids, api_key
            )
        } else {
            format!(
                "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
                ids
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
            anyhow::bail!("CoinGecko API error: {} - {}", status, body);
        }

        let prices: HashMap<String, CoinGeckoSimplePrice> = response
            .json()
            .await
            .context("Failed to parse CoinGecko response")?;

        Ok(prices.into_iter().map(|(k, v)| (k, v.usd)).collect())
    }

    fn name(&self) -> &str {
        "CoinGecko"
    }
}

/// Main price feed client with caching
pub struct PriceFeedClient {
    provider: Arc<dyn PriceFeedProvider>,
    cache: Arc<RwLock<HashMap<String, CachedPrice>>>,
    asset_mapping: Arc<HashMap<String, String>>,
    config: PriceFeedConfig,
}

impl PriceFeedClient {
    /// Create a new price feed client
    pub fn new(config: PriceFeedConfig, asset_mapping: HashMap<String, String>) -> Self {
        let timeout = Duration::from_secs(config.request_timeout_seconds);

        let provider: Arc<dyn PriceFeedProvider> = match config.provider.as_str() {
            "coingecko" => Arc::new(CoinGeckoProvider::new(config.api_key.clone(), timeout)),
            _ => {
                warn!(
                    "Unknown provider '{}', defaulting to CoinGecko",
                    config.provider
                );
                Arc::new(CoinGeckoProvider::new(config.api_key.clone(), timeout))
            }
        };

        info!(
            "Initialized price feed client with provider: {}",
            provider.name()
        );

        Self {
            provider,
            cache: Arc::new(RwLock::new(HashMap::new())),
            asset_mapping: Arc::new(asset_mapping),
            config,
        }
    }

    /// Get price for a Stellar asset, returns USD value
    pub async fn get_price(&self, stellar_asset: &str) -> Result<f64> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(stellar_asset) {
                let age = cached.timestamp.elapsed();
                if age.as_secs() < self.config.cache_ttl_seconds {
                    debug!("Cache hit for {}: ${}", stellar_asset, cached.price_usd);
                    return Ok(cached.price_usd);
                }
            }
        }

        // Map Stellar asset to provider asset ID
        let asset_id = self
            .asset_mapping
            .get(stellar_asset)
            .ok_or_else(|| anyhow::anyhow!("No mapping found for asset: {}", stellar_asset))?;

        // Fetch from provider
        debug!("Fetching price for {} ({})", stellar_asset, asset_id);
        match self.provider.fetch_price(asset_id).await {
            Ok(price) => {
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
                Ok(price)
            }
            Err(e) => {
                error!("Failed to fetch price for {}: {}", stellar_asset, e);

                // Try to return stale cache data as fallback
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(stellar_asset) {
                    warn!(
                        "Using stale cache data for {} (age: {:?})",
                        stellar_asset,
                        cached.timestamp.elapsed()
                    );
                    return Ok(cached.price_usd);
                }

                Err(e)
            }
        }
    }

    /// Get prices for multiple Stellar assets
    pub async fn get_prices(&self, stellar_assets: &[String]) -> HashMap<String, f64> {
        let mut result = HashMap::new();
        let mut to_fetch = Vec::new();

        // Check cache for each asset
        {
            let cache = self.cache.read().await;
            for asset in stellar_assets {
                if let Some(cached) = cache.get(asset) {
                    let age = cached.timestamp.elapsed();
                    if age.as_secs() < self.config.cache_ttl_seconds {
                        result.insert(asset.clone(), cached.price_usd);
                        continue;
                    }
                }
                to_fetch.push(asset.clone());
            }
        }

        if to_fetch.is_empty() {
            return result;
        }

        // Map to provider asset IDs
        let provider_ids: Vec<String> = to_fetch
            .iter()
            .filter_map(|asset| self.asset_mapping.get(asset).cloned())
            .collect();

        if provider_ids.is_empty() {
            return result;
        }

        // Fetch from provider
        match self.provider.fetch_prices(&provider_ids).await {
            Ok(prices) => {
                let mut cache = self.cache.write().await;

                // Map back to Stellar assets and update cache
                for (stellar_asset, provider_id) in to_fetch.iter().zip(provider_ids.iter()) {
                    if let Some(&price) = prices.get(provider_id) {
                        cache.insert(
                            stellar_asset.clone(),
                            CachedPrice {
                                price_usd: price,
                                timestamp: Instant::now(),
                            },
                        );
                        result.insert(stellar_asset.clone(), price);
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch prices: {}", e);

                // Use stale cache as fallback
                let cache = self.cache.read().await;
                for asset in &to_fetch {
                    if let Some(cached) = cache.get(asset) {
                        warn!("Using stale cache for {}", asset);
                        result.insert(asset.clone(), cached.price_usd);
                    }
                }
            }
        }

        result
    }

    /// Convert an amount in a Stellar asset to USD
    pub async fn convert_to_usd(&self, stellar_asset: &str, amount: f64) -> Result<f64> {
        let price = self.get_price(stellar_asset).await?;
        Ok(amount * price)
    }

    /// Clear the cache (useful for testing)
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Price cache cleared");
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

/// Default asset mapping for common Stellar assets
pub fn default_asset_mapping() -> HashMap<String, String> {
    let mut mapping = HashMap::new();

    // Native XLM
    mapping.insert("XLM:native".to_string(), "stellar".to_string());
    mapping.insert("native".to_string(), "stellar".to_string());

    // USDC
    mapping.insert(
        "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        "usd-coin".to_string(),
    );

    // EURC
    mapping.insert(
        "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2".to_string(),
        "euro-coin".to_string(),
    );

    // USDT
    mapping.insert(
        "USDT:GCQTGZQQ5G4PTM2GL7CDIFKUBIPEC52BROAQIAPW53XBRJVN6ZJVTG6V".to_string(),
        "tether".to_string(),
    );

    // BTC (various anchors)
    mapping.insert(
        "BTC:GDXTJEK4JZNSTNQAWA53RZNS2GIKTDRPEUWDXELFMKU52XNECNVDVXDI".to_string(),
        "bitcoin".to_string(),
    );

    // ETH (various anchors)
    mapping.insert(
        "ETH:GDXTJEK4JZNSTNQAWA53RZNS2GIKTDRPEUWDXELFMKU52XNECNVDVXDI".to_string(),
        "ethereum".to_string(),
    );

    // yXLM (Ultra Stellar)
    mapping.insert(
        "yXLM:GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55".to_string(),
        "stellar".to_string(),
    );

    // AQUA
    mapping.insert(
        "AQUA:GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA".to_string(),
        "aquarius".to_string(),
    );

    mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        std::env::set_var("PRICE_FEED_PROVIDER", "coingecko");
        std::env::set_var("PRICE_FEED_CACHE_TTL_SECONDS", "600");

        let config = PriceFeedConfig::from_env();
        assert_eq!(config.provider, "coingecko");
        assert_eq!(config.cache_ttl_seconds, 600);
    }

    #[test]
    fn test_default_asset_mapping() {
        let mapping = default_asset_mapping();
        assert!(mapping.contains_key("XLM:native"));
        assert_eq!(mapping.get("XLM:native").unwrap(), "stellar");
        assert!(
            mapping.contains_key("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")
        );
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let config = PriceFeedConfig {
            cache_ttl_seconds: 1,
            ..Default::default()
        };
        let mapping = default_asset_mapping();
        let client = PriceFeedClient::new(config, mapping);

        // Manually insert a cached price
        {
            let mut cache = client.cache.write().await;
            cache.insert(
                "XLM:native".to_string(),
                CachedPrice {
                    price_usd: 0.10,
                    timestamp: Instant::now(),
                },
            );
        }

        // Check cache stats
        let (total, fresh) = client.cache_stats().await;
        assert_eq!(total, 1);
        assert_eq!(fresh, 1);

        // Wait for cache to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        let (total, fresh) = client.cache_stats().await;
        assert_eq!(total, 1);
        assert_eq!(fresh, 0);
    }
}
