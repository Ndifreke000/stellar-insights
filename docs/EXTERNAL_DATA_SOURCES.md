# 🌍 External Data Sources Required

This document lists all data that **CANNOT** be obtained from Stellar RPC/Horizon API and requires external sources.

---

## 💰 Price Data (CRITICAL)

### **USD/Fiat Price Feeds**

**Why Needed:**
- Convert crypto amounts to USD for volume calculations
- Display fiat-equivalent values in UI
- Calculate liquidity in USD terms

**Required Prices:**
- XLM/USD
- USDC/USD  
- EURC/EUR
- Other stablecoin prices
- Major asset prices

**Recommended Sources:**

1. **CoinGecko API** (Free tier available)
   ```
   GET https://api.coingecko.com/api/v3/simple/price
   ?ids=stellar&vs_currencies=usd
   ```
   - Rate limit: 10-50 calls/minute (free)
   - Supports: XLM, major tokens
   - Documentation: https://www.coingecko.com/en/api

2. **CoinMarketCap API** (Free tier: 333 calls/day)
   ```
   GET https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest
   ?symbol=XLM&convert=USD
   ```
   - More reliable for enterprise
   - Better rate limits on paid tiers
   - Documentation: https://coinmarketcap.com/api/

3. **Stellar Expert API** (Stellar-specific)
   ```
   GET https://api.stellar.expert/explorer/public/asset/XLM-native/market
   ```
   - Stellar-focused
   - Free, no API key needed
   - Documentation: https://stellar.expert/openapi.html

4. **Fallback: Stellar DEX Prices**
   ```rust
   // Use order book to calculate price
   let order_book = rpc_client.fetch_order_book(xlm, usdc).await?;
   let mid_price = calculate_mid_price(&order_book);
   ```
   - Always available
   - May have slippage
   - Good for less liquid assets

**Implementation Priority:** 🔴 **HIGH** - Required for all USD calculations

**Endpoints Affected:**
- `/api/anchors` - volume_usd
- `/api/corridors` - liquidity_depth_usd, volume_usd
- `/api/analytics` - all USD metrics
- `/api/liquidity` - liquidity calculations

---

## 🏢 Anchor Metadata

### **Anchor Information**

**Why Needed:**
- Anchor names, logos, descriptions not on blockchain
- Contact information
- Regulatory status
- Service details

**Data Required:**
- Anchor name
- Home domain
- Logo URL
- Description
- Contact email
- Supported countries
- KYC requirements
- Fee structure

**Sources:**

1. **stellar.toml Files**
   ```
   GET https://{home_domain}/.well-known/stellar.toml
   ```
   - Standard Stellar metadata
   - Anchor self-published
   - Contains: currencies, validators, principals

2. **Manual Entry**
   - Admin panel for adding anchors
   - Verification process
   - Stored in local database

3. **Stellar Expert Directory**
   ```
   GET https://api.stellar.expert/explorer/directory/anchors
   ```
   - Curated anchor list
   - Basic metadata

**Implementation Priority:** 🟡 **MEDIUM** - Enhances UX but not critical

**Endpoints Affected:**
- `POST /api/anchors` - Create anchor with metadata
- `GET /api/anchors/:id` - Return full anchor details

---

## 🪙 Asset Metadata

### **Asset Information**

**Why Needed:**
- Asset names, symbols, descriptions not on blockchain
- Issuer information
- Asset images/logos

**Data Required:**
- Asset name (e.g., "USD Coin" for USDC)
- Asset symbol (e.g., "USDC")
- Decimals/precision
- Logo URL
- Description
- Issuer name
- Website
- Asset type (stablecoin, token, etc.)

**Sources:**

1. **stellar.toml from Issuer**
   ```toml
   [[CURRENCIES]]
   code = "USDC"
   issuer = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
   display_decimals = 7
   name = "USD Coin"
   desc = "USDC on Stellar"
   image = "https://example.com/usdc.png"
   ```

2. **Stellar Expert Asset Directory**
   ```
   GET https://api.stellar.expert/explorer/public/asset/{code}-{issuer}
   ```

3. **Manual Curation**
   - Database of known assets
   - Admin verification

**Implementation Priority:** 🟡 **MEDIUM** - Improves UX

**Endpoints Affected:**
- `POST /api/anchors/:id/assets` - Add asset metadata
- `GET /api/assets` - List assets with metadata

---

## 🌍 Geolocation Data

### **Country/Region Information**

**Why Needed:**
- Corridor geographic analysis
- Regulatory compliance
- Regional statistics

**Data Required:**
- Account country (if available)
- Anchor jurisdiction
- Supported regions
- Regulatory zones

**Sources:**

1. **Anchor stellar.toml**
   - Supported countries listed
   - Regulatory information

2. **IP Geolocation** (for analytics only)
   - MaxMind GeoIP2
   - IP2Location
   - **Note:** Not reliable for blockchain data

3. **Manual Configuration**
   - Admin-defined regions
   - Anchor-reported data

**Implementation Priority:** 🟢 **LOW** - Nice to have

**Endpoints Affected:**
- `/api/analytics/regions` - Regional statistics
- `/api/corridors` - Geographic filtering

---

## 📊 Historical Exchange Rates

### **Historical Price Data**

**Why Needed:**
- Calculate historical USD values
- Trend analysis
- Performance metrics over time

**Data Required:**
- Historical XLM/USD prices
- Historical asset prices
- Daily/hourly price points

**Sources:**

1. **CoinGecko Historical API**
   ```
   GET https://api.coingecko.com/api/v3/coins/stellar/market_chart
   ?vs_currency=usd&days=30
   ```

2. **CoinMarketCap Historical**
   ```
   GET https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/historical
   ```

3. **Cache Strategy**
   - Store daily prices in database
   - Update once per day
   - Reduce API calls

**Implementation Priority:** 🟡 **MEDIUM** - For historical analysis

**Endpoints Affected:**
- `/api/analytics/historical` - Historical charts
- `/api/corridors/:id` - Historical volume

---

## 🔐 Authentication Data

### **User Accounts**

**Why Needed:**
- User authentication
- API key management
- Access control

**Data Required:**
- User credentials
- API keys
- Permissions
- Session tokens

**Sources:**
- Local database only
- Not related to blockchain

**Implementation Priority:** 🟡 **MEDIUM** - For protected endpoints

**Endpoints Affected:**
- `POST /api/auth/login`
- `POST /api/auth/register`
- Protected endpoints

---

## 📝 Implementation Checklist

### Phase 1: Critical (Required for MVP)
- [ ] Implement price feed integration (CoinGecko or CMC)
- [ ] Add price caching layer (Redis)
- [ ] Create fallback to DEX prices
- [ ] Update all USD calculations to use price feed

### Phase 2: Enhanced Metadata
- [ ] Implement stellar.toml parser
- [ ] Add anchor metadata admin panel
- [ ] Create asset metadata database
- [ ] Integrate Stellar Expert API

### Phase 3: Advanced Features
- [ ] Historical price data caching
- [ ] Geolocation integration
- [ ] Regional analytics
- [ ] Advanced filtering

---

## 🔧 Configuration

### Environment Variables

Add to `backend/.env`:

```env
# Price Feeds
COINGECKO_API_KEY=your_api_key_here
COINMARKETCAP_API_KEY=your_api_key_here
PRICE_FEED_PROVIDER=coingecko  # or coinmarketcap
PRICE_CACHE_TTL=300  # 5 minutes

# Stellar Expert
STELLAR_EXPERT_API=https://api.stellar.expert/explorer/public

# Fallback Prices
USE_DEX_FALLBACK=true
DEFAULT_XLM_USD_PRICE=0.10  # Fallback if all APIs fail
```

---

## 📊 API Integration Examples

### CoinGecko Integration

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct CoinGeckoPrice {
    stellar: CoinGeckoPriceData,
}

#[derive(Deserialize)]
struct CoinGeckoPriceData {
    usd: f64,
}

pub async fn fetch_xlm_price() -> Result<f64> {
    let client = Client::new();
    let response = client
        .get("https://api.coingecko.com/api/v3/simple/price")
        .query(&[("ids", "stellar"), ("vs_currencies", "usd")])
        .send()
        .await?;
    
    let price: CoinGeckoPrice = response.json().await?;
    Ok(price.stellar.usd)
}
```

### Stellar.toml Parser

```rust
use reqwest::Client;

pub async fn fetch_anchor_metadata(home_domain: &str) -> Result<AnchorMetadata> {
    let client = Client::new();
    let toml_url = format!("https://{}/.well-known/stellar.toml", home_domain);
    
    let response = client.get(&toml_url).send().await?;
    let toml_content = response.text().await?;
    
    let metadata: AnchorMetadata = toml::from_str(&toml_content)?;
    Ok(metadata)
}
```

---

## ⚠️ Important Notes

1. **Rate Limiting**: All external APIs have rate limits. Implement caching!
2. **Fallbacks**: Always have fallback strategies for critical data (prices)
3. **Caching**: Cache external data aggressively (5-15 minutes for prices)
4. **Error Handling**: External APIs can fail - handle gracefully
5. **Costs**: Some APIs require paid plans for production use

---

**Last Updated:** February 2, 2026
