---
title: "[FEATURE] Missing Price Feed Integration for USD Conversions"
labels: enhancement, critical, external-api, pricing, backend
assignees: ''
---

## 💡 Description

The backend currently has no integration with price feed APIs (CoinGecko, CoinMarketCap, etc.) to convert asset values to USD. All USD values are either hardcoded, estimated, or missing. This is documented as a critical need in `docs/EXTERNAL_DATA_SOURCES.md`.

## 📊 Current Behavior

- USD values are calculated incorrectly or missing
- No real-time price data for assets
- Liquidity and volume metrics are inaccurate
- Cannot compare corridors in common currency

## ✅ Expected Behavior

- Integrate with CoinGecko or CoinMarketCap API
- Fetch real-time prices for all Stellar assets
- Cache prices with appropriate TTL (5-15 minutes)
- Convert all asset amounts to USD for display
- Handle rate limiting and API failures gracefully

## 📁 Affected Files

- `backend/src/api/corridors_cached.rs` (all USD calculations)
- `backend/src/api/anchors_cached.rs` (volume calculations)
- New file needed: `backend/src/services/price_feed.rs`

## 🔧 Technical Requirements

1. Create `PriceFeedClient` service
2. Support multiple providers (CoinGecko primary, CoinMarketCap fallback)
3. Implement asset ID mapping (Stellar asset codes to API IDs)
4. Add Redis/in-memory caching for prices
5. Handle API rate limits (CoinGecko: 10-50 calls/min)
6. Implement fallback strategies when API unavailable

## 🎯 API Options

### CoinGecko (Recommended)
- Free tier: 10-50 calls/min
- Endpoint: `/simple/price`
- Supports multiple currencies
- Good Stellar asset coverage
- API: https://www.coingecko.com/en/api/documentation

### CoinMarketCap
- Requires API key
- 333 calls/day free tier
- More comprehensive data
- Better for institutional use

## 📋 Acceptance Criteria

- [ ] Create `PriceFeedClient` struct with trait-based design
- [ ] Implement CoinGecko provider
- [ ] Add price caching (15-minute TTL)
- [ ] Create asset mapping configuration
- [ ] Handle API failures with stale data fallback
- [ ] Add rate limiting protection
- [ ] Update all USD calculation functions
- [ ] Add unit and integration tests
- [ ] Document API key setup in README

## 🚀 Implementation Steps

1. Add dependencies: `reqwest`, `serde_json`
2. Create `backend/src/services/price_feed.rs`
3. Define `PriceFeedProvider` trait
4. Implement `CoinGeckoProvider`
5. Create asset mapping config file
6. Add caching layer with TTL
7. Integrate into main app state
8. Update corridor and anchor endpoints
9. Add monitoring/logging for API calls
10. Create fallback mechanism

## ⚙️ Configuration Example

```toml
# backend/.env
PRICE_FEED_PROVIDER=coingecko
PRICE_FEED_API_KEY=optional
PRICE_FEED_CACHE_TTL_SECONDS=900
```

## 🗺️ Asset Mapping Example

```json
{
  "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN": "usd-coin",
  "XLM:native": "stellar",
  "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2": "euro-coin"
}
```

## 💻 Code Example

```rust
pub trait PriceFeedProvider: Send + Sync {
    async fn get_price(&self, asset_id: &str, currency: &str) -> Result<f64>;
    async fn get_prices(&self, asset_ids: &[String], currency: &str) -> Result<HashMap<String, f64>>;
}

pub struct CoinGeckoProvider {
    client: reqwest::Client,
    api_key: Option<String>,
    rate_limiter: RateLimiter,
}

impl CoinGeckoProvider {
    pub async fn get_price(&self, asset_id: &str, currency: &str) -> Result<f64> {
        self.rate_limiter.acquire().await?;
        
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}",
            asset_id, currency
        );
        
        let response = self.client.get(&url).send().await?;
        let data: Value = response.json().await?;
        
        Ok(data[asset_id][currency].as_f64().unwrap_or(0.0))
    }
}
```

## 🏷️ Priority

**Critical** - All USD values are currently inaccurate

## 📚 References

- Documented in: `docs/EXTERNAL_DATA_SOURCES.md`
- CoinGecko API: https://www.coingecko.com/en/api/documentation
- CoinMarketCap API: https://coinmarketcap.com/api/documentation/v1/
