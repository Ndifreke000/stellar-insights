# 🌐 Stellar Insights - RPC Data Source Mapping

This document maps all frontend/backend endpoints to their data sources, indicating which use Stellar RPC/Horizon API vs which need external data.

---

## ✅ RPC/Horizon Data Sources (Real-time from Stellar Network)

### 1. **Payments Data**
- **Source:** Horizon API `/payments` endpoint
- **RPC Method:** `fetch_payments()`, `fetch_account_payments()`
- **Used By:**
  - `/api/rpc/payments` - Recent payment operations
  - `/api/rpc/payments/account/:account_id` - Account-specific payments
  - Corridor metrics calculation
  - Anchor transaction tracking

### 2. **Trades Data**
- **Source:** Horizon API `/trades` endpoint  
- **RPC Method:** `fetch_trades()`
- **Used By:**
  - `/api/rpc/trades` - Recent DEX trades
  - Corridor volume calculations
  - Liquidity metrics

### 3. **Order Book Data**
- **Source:** Horizon API `/order_book` endpoint
- **RPC Method:** `fetch_order_book()`
- **Used By:**
  - `/api/rpc/orderbook` - Trading pair order books
  - Liquidity depth calculations
  - Slippage estimates

### 4. **Ledger Data**
- **Source:** Horizon API `/ledgers` + RPC `getLedgers`
- **RPC Method:** `fetch_latest_ledger()`, `fetch_ledgers()`
- **Used By:**
  - `/api/rpc/ledger/latest` - Latest ledger info
  - Network health monitoring
  - Transaction confirmation tracking

### 5. **Account Data** (TO BE IMPLEMENTED)
- **Source:** Horizon API `/accounts/:account_id`
- **RPC Method:** `fetch_account_info()` (needs implementation)
- **Used By:**
  - Anchor account balances
  - Asset issuer verification
  - Trust line information

### 6. **Asset Data** (TO BE IMPLEMENTED)
- **Source:** Horizon API `/assets`
- **RPC Method:** `fetch_assets()` (needs implementation)
- **Used By:**
  - Asset discovery
  - Issuer information
  - Supply metrics

---

## 🔄 Derived/Calculated Data (From RPC Data)

### 1. **Anchor Metrics**
- **Calculated From:** Payments + Account data
- **Metrics:**
  - Total transactions (count payments for anchor account)
  - Success rate (successful vs failed payments)
  - Average settlement time (payment timestamps)
  - Volume USD (sum of payment amounts × prices)
- **Endpoints:**
  - `/api/anchors` - List anchors with metrics
  - `/api/anchors/:id` - Anchor details
  - `/api/anchors/account/:stellar_account` - By account

### 2. **Corridor Metrics**
- **Calculated From:** Payments + Trades + Order Books
- **Metrics:**
  - Success rate (payment success/failure)
  - Volume (sum of payments between asset pairs)
  - Liquidity depth (order book depth)
  - Slippage (order book spread analysis)
  - Latency (payment confirmation times)
- **Endpoints:**
  - `/api/corridors` - List corridors
  - `/api/corridors/:corridor_key` - Corridor details

### 3. **Network Health**
- **Calculated From:** Ledgers + Payments + Trades
- **Metrics:**
  - Transaction throughput (ledger transaction counts)
  - Network latency (ledger close times)
  - Success rates (payment success ratios)
- **Endpoints:**
  - `/api/health` - Network health dashboard

### 4. **Liquidity Metrics**
- **Calculated From:** Order Books + Trades
- **Metrics:**
  - Depth (order book volume at price levels)
  - 24h volume (trade volumes)
  - Spread (bid-ask difference)
- **Endpoints:**
  - `/api/liquidity` - Liquidity dashboard

---

## ⚠️ External Data Sources (Not from RPC)

### 1. **Anchor Metadata**
- **Source:** Manual entry or external APIs
- **Data:**
  - Anchor name
  - Home domain
  - Logo/branding
  - Contact information
- **Why External:** Not stored on Stellar blockchain
- **Endpoints:**
  - `POST /api/anchors` - Create anchor
  - `PUT /api/anchors/:id` - Update anchor info

### 2. **Asset Metadata**
- **Source:** stellar.toml files + manual entry
- **Data:**
  - Asset name/description
  - Logo
  - Issuer information
- **Why External:** Stored in stellar.toml, not on-chain
- **Endpoints:**
  - `POST /api/anchors/:id/assets` - Add asset

### 3. **Price Data (USD Conversion)**
- **Source:** External price feeds (CoinGecko, CoinMarketCap, etc.)
- **Data:**
  - XLM/USD price
  - Asset/USD prices
- **Why External:** Stellar doesn't provide fiat prices
- **Needed For:**
  - Volume USD calculations
  - Liquidity USD values
  - Analytics dashboards

### 4. **Historical Aggregations**
- **Source:** Local database (cached from RPC)
- **Data:**
  - Daily/hourly metrics
  - Historical trends
  - Aggregated statistics
- **Why Cached:** Performance optimization
- **Endpoints:**
  - Historical charts
  - Trend analysis

---

## 🔧 Implementation Strategy

### Phase 1: Direct RPC Integration (Current)
```
Frontend → Backend API → RPC Client → Stellar Network
```

### Phase 2: Smart Caching
```
Frontend → Backend API → Cache Check → RPC Client → Stellar Network
                              ↓
                         Return Cached
```

### Phase 3: Real-time Updates
```
Frontend ← WebSocket ← Background Worker → RPC Client → Stellar Network
```

---

## 📊 Data Flow Examples

### Example 1: Anchor Metrics
```
1. Frontend requests: GET /api/anchors
2. Backend calls: rpc_client.fetch_account_payments(anchor_account)
3. Backend calculates:
   - total_transactions = payments.len()
   - successful = payments.filter(|p| p.successful).len()
   - success_rate = successful / total * 100
4. Backend returns calculated metrics
```

### Example 2: Corridor Health
```
1. Frontend requests: GET /api/corridors/USDC:GBBD..._XLM:native
2. Backend calls:
   - rpc_client.fetch_payments() // Filter by asset pair
   - rpc_client.fetch_order_book(USDC, XLM)
   - rpc_client.fetch_trades() // Filter by asset pair
3. Backend calculates:
   - success_rate from payments
   - liquidity_depth from order book
   - volume from trades
4. Backend returns corridor metrics
```

### Example 3: Real-time Payment Stream
```
1. Frontend connects: WebSocket /ws
2. Backend subscribes: Horizon streaming /payments
3. On new payment:
   - Filter relevant payments
   - Calculate updated metrics
   - Push to WebSocket clients
```

---

## 🚀 Required RPC Methods

### Already Implemented ✅
- `check_health()` - RPC health
- `fetch_latest_ledger()` - Latest ledger
- `fetch_ledgers()` - Ledger range
- `fetch_payments()` - Recent payments
- `fetch_account_payments()` - Account payments
- `fetch_trades()` - Recent trades
- `fetch_order_book()` - Order book

### To Be Implemented 🔨
- `fetch_account_info()` - Account details
- `fetch_assets()` - Asset list
- `fetch_asset_holders()` - Trust lines
- `stream_payments()` - Real-time payment stream
- `stream_ledgers()` - Real-time ledger stream
- `fetch_transaction()` - Transaction details
- `fetch_operations()` - Operation list

---

## 💾 Database Usage

### What Should Be in Database:
1. **Anchor Registry** - Manually added anchor metadata
2. **Asset Registry** - Manually added asset metadata  
3. **Cached Metrics** - Performance optimization only
4. **User Data** - Authentication, preferences
5. **Aggregations** - Pre-calculated historical data

### What Should NOT Be in Database:
1. ❌ Real-time payment data (fetch from RPC)
2. ❌ Current ledger data (fetch from RPC)
3. ❌ Live trade data (fetch from RPC)
4. ❌ Order book data (fetch from RPC)
5. ❌ Account balances (fetch from RPC)

---

## 🎯 Next Steps

1. **Update Anchor Endpoints** - Fetch metrics from RPC instead of DB
2. **Update Corridor Endpoints** - Calculate from RPC payment/trade data
3. **Implement Missing RPC Methods** - Account info, assets, etc.
4. **Add Price Feed Integration** - External API for USD prices
5. **Implement Caching Layer** - Redis for performance
6. **Add WebSocket Streaming** - Real-time updates

---

**Last Updated:** February 2, 2026
