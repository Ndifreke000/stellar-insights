# 🌐 Stellar Insights RPC API Documentation

**Version:** v0.1.0  
**Last Updated:** February 26, 2026

Complete API reference for accessing real-time Stellar blockchain data and analytics.

---

## 📋 Table of Contents

1. [Quick Start](#quick-start)
2. [Configuration](#configuration)
3. [RPC Endpoints](#rpc-endpoints)
4. [Analytics Endpoints](#analytics-endpoints)
5. [Response Codes](#response-codes)
6. [Usage Examples](#usage-examples)
7. [External Resources](#external-resources)

---

## 🚀 Quick Start

### Base URLs

**Local Development:**
```
http://localhost:8080
```

**Production:**
```
https://your-domain.com
```

### Start the Backend

```bash
cd backend
cargo run
```

Server starts on `http://localhost:8080`

### Test the API

```bash
# Health check
curl http://localhost:8080/health

# RPC health
curl http://localhost:8080/api/rpc/health

# Get latest ledger
curl http://localhost:8080/api/rpc/ledger/latest

# Get recent payments
curl http://localhost:8080/api/rpc/payments?limit=10
```

---

## 🔧 Configuration

### Environment Variables

Create `.env` file in backend directory:

```env
# Database
DATABASE_URL=sqlite:stellar_insights.db

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080

# Stellar Network
STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
STELLAR_HORIZON_URL=https://horizon.stellar.org

# Mock Mode (for testing without real RPC calls)
RPC_MOCK_MODE=false

# Logging
RUST_LOG=info
```

### Mock Mode

For development without hitting the real Stellar network:

```env
RPC_MOCK_MODE=true
```

Returns mock data for all RPC endpoints.

---

## 🔌 RPC Endpoints

### Health Check

**Endpoint:** `GET /api/rpc/health`

Check Stellar RPC connection health and network status.

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": "healthy",
    "latestLedger": 51583040,
    "oldestLedger": 51565760,
    "ledgerRetentionWindow": 17281
  }
}
```

**Example:**
```bash
curl http://localhost:8080/api/rpc/health
```

---

### Latest Ledger

**Endpoint:** `GET /api/rpc/ledger/latest`

Get the most recent ledger information from Stellar network.

**Response:**
```json
{
  "sequence": 51583040,
  "hash": "abc123...",
  "closed_at": "2026-01-26T10:30:00Z",
  "transaction_count": 142,
  "operation_count": 389
}
```

**Example:**
```bash
curl http://localhost:8080/api/rpc/ledger/latest
```

---

### Payments

**Endpoint:** `GET /api/rpc/payments`

Fetch recent payment operations from the Stellar network.

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `limit` | integer | No | 20 | Number of records (max: 200) |
| `cursor` | string | No | - | Pagination cursor |

**Response:**
```json
{
  "_embedded": {
    "records": [
      {
        "id": "123456789",
        "type": "payment",
        "from": "GABC...",
        "to": "GDEF...",
        "asset_type": "credit_alphanum4",
        "asset_code": "USDC",
        "asset_issuer": "GBBD...",
        "amount": "100.0000000",
        "created_at": "2026-01-26T10:30:00Z"
      }
    ]
  },
  "_links": {
    "next": {
      "href": "/api/rpc/payments?cursor=123456789&limit=20"
    }
  }
}
```

**Examples:**
```bash
# Get 20 most recent payments
curl http://localhost:8080/api/rpc/payments

# Get 50 payments
curl http://localhost:8080/api/rpc/payments?limit=50

# Paginate with cursor
curl "http://localhost:8080/api/rpc/payments?cursor=123456789&limit=20"
```

---

### Account Payments

**Endpoint:** `GET /api/rpc/payments/account/:account_id`

Get payment history for a specific Stellar account.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `account_id` | string | Yes | Stellar account address (G...) |

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `limit` | integer | No | 20 | Number of records |

**Response:**
```json
{
  "_embedded": {
    "records": [
      {
        "id": "123456789",
        "type": "payment",
        "from": "GABC...",
        "to": "GDEF...",
        "asset_code": "USDC",
        "amount": "100.0000000"
      }
    ]
  }
}
```

**Example:**
```bash
curl http://localhost:8080/api/rpc/payments/account/GABC123...
```

---

### Trades

**Endpoint:** `GET /api/rpc/trades`

Fetch recent trade operations from the Stellar DEX.

**Query Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `limit` | integer | No | 20 | Number of records |
| `cursor` | string | No | - | Pagination cursor |

**Response:**
```json
{
  "_embedded": {
    "records": [
      {
        "id": "123456789",
        "base_asset_type": "credit_alphanum4",
        "base_asset_code": "USDC",
        "base_amount": "100.0000000",
        "counter_asset_type": "native",
        "counter_amount": "95.0000000",
        "price": {
          "n": 95,
          "d": 100
        },
        "ledger_close_time": "2026-01-26T10:30:00Z"
      }
    ]
  }
}
```

**Example:**
```bash
curl http://localhost:8080/api/rpc/trades?limit=50
```

---

### Order Book

**Endpoint:** `GET /api/rpc/orderbook`

Get order book for a specific trading pair.

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `selling_asset_type` | string | Yes | "native", "credit_alphanum4", or "credit_alphanum12" |
| `selling_asset_code` | string | Conditional | Required if not native |
| `selling_asset_issuer` | string | Conditional | Required if not native |
| `buying_asset_type` | string | Yes | Asset type |
| `buying_asset_code` | string | Conditional | Required if not native |
| `buying_asset_issuer` | string | Conditional | Required if not native |
| `limit` | integer | No | Number of price levels (default: 20) |

**Response:**
```json
{
  "bids": [
    {
      "price": "0.9500000",
      "amount": "1000.0000000",
      "price_r": {
        "n": 95,
        "d": 100
      }
    }
  ],
  "asks": [
    {
      "price": "1.0500000",
      "amount": "500.0000000",
      "price_r": {
        "n": 105,
        "d": 100
      }
    }
  ],
  "base": {
    "asset_type": "credit_alphanum4",
    "asset_code": "USDC",
    "asset_issuer": "GBBD..."
  },
  "counter": {
    "asset_type": "native"
  }
}
```

**Example:**
```bash
# USDC/XLM order book
curl "http://localhost:8080/api/rpc/orderbook?\
selling_asset_type=credit_alphanum4&\
selling_asset_code=USDC&\
selling_asset_issuer=GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5&\
buying_asset_type=native&\
limit=10"
```

---

## 📊 Analytics Endpoints

### Anchors

#### List All Anchors

**Endpoint:** `GET /api/anchors`

List all tracked anchors with their metrics.

**Response:**
```json
[
  {
    "id": 1,
    "name": "Circle",
    "stellar_account": "GBBD...",
    "home_domain": "circle.com",
    "total_transactions": 1000000,
    "successful_transactions": 998500,
    "success_rate": 99.85,
    "avg_settlement_time_ms": 4200,
    "volume_usd": 50000000.00,
    "created_at": "2026-01-01T00:00:00Z"
  }
]
```

**Example:**
```bash
curl http://localhost:8080/api/anchors
```

---

#### Get Anchor by ID

**Endpoint:** `GET /api/anchors/:id`

Get detailed information for a specific anchor.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Anchor ID |

**Example:**
```bash
curl http://localhost:8080/api/anchors/1
```

---

#### Get Anchor by Account

**Endpoint:** `GET /api/anchors/account/:stellar_account`

Get anchor by Stellar account address.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `stellar_account` | string | Yes | Stellar account address |

**Example:**
```bash
curl http://localhost:8080/api/anchors/account/GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

---

#### Create Anchor

**Endpoint:** `POST /api/anchors`

Create a new anchor for tracking.

**Request Body:**
```json
{
  "name": "Circle",
  "stellar_account": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
  "home_domain": "circle.com"
}
```

**Example:**
```bash
curl -X POST http://localhost:8080/api/anchors \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Circle",
    "stellar_account": "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
    "home_domain": "circle.com"
  }'
```

---

#### Update Anchor Metrics

**Endpoint:** `PUT /api/anchors/:id/metrics`

Update anchor performance metrics.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | integer | Yes | Anchor ID |

**Request Body:**
```json
{
  "total_transactions": 1000,
  "successful_transactions": 990,
  "avg_settlement_time_ms": 4200,
  "volume_usd": 100000.00
}
```

**Example:**
```bash
curl -X PUT http://localhost:8080/api/anchors/1/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "total_transactions": 1000,
    "successful_transactions": 990,
    "avg_settlement_time_ms": 4200,
    "volume_usd": 100000.00
  }'
```

---

### Corridors

#### List All Corridors

**Endpoint:** `GET /api/corridors`

List all payment corridors with health metrics.

**Response:**
```json
[
  {
    "id": 1,
    "source_asset": "USDC:GBBD...",
    "destination_asset": "XLM:native",
    "success_rate": 98.5,
    "avg_slippage": 0.25,
    "total_volume_usd": 5000000.00,
    "transaction_count": 15000,
    "health_score": 95.2,
    "last_updated": "2026-01-26T10:30:00Z"
  }
]
```

**Example:**
```bash
curl http://localhost:8080/api/corridors
```

---

#### Get Corridor Details

**Endpoint:** `GET /api/corridors/:corridor_key`

Get detailed metrics for a specific corridor.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `corridor_key` | string | Yes | Format: `SOURCE_ASSET:DEST_ASSET` |

**Example:**
```bash
curl http://localhost:8080/api/corridors/USDC:GBBD..._XLM:native
```

---

#### Create Corridor

**Endpoint:** `POST /api/corridors`

Create a new corridor for tracking.

**Request Body:**
```json
{
  "source_asset": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
  "destination_asset": "XLM:native"
}
```

**Example:**
```bash
curl -X POST http://localhost:8080/api/corridors \
  -H "Content-Type: application/json" \
  -d '{
    "source_asset": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
    "destination_asset": "XLM:native"
  }'
```

---

## 📝 Response Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 201 | Created |
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Authentication required |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found - Resource doesn't exist |
| 429 | Too Many Requests - Rate limit exceeded |
| 500 | Internal Server Error |
| 503 | Service Unavailable - RPC connection failed |

---

## 💡 Usage Examples

### JavaScript/TypeScript

```typescript
// Fetch recent payments
const response = await fetch('http://localhost:8080/api/rpc/payments?limit=10');
const data = await response.json();
console.log(data);

// Get anchor details
const anchor = await fetch('http://localhost:8080/api/anchors/1');
const anchorData = await anchor.json();

// Create new corridor
const newCorridor = await fetch('http://localhost:8080/api/corridors', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    source_asset: 'USDC:GBBD...',
    destination_asset: 'XLM:native'
  })
});
```

---

### Python

```python
import requests

# Get latest ledger
response = requests.get('http://localhost:8080/api/rpc/ledger/latest')
ledger = response.json()
print(ledger)

# Get corridors
corridors = requests.get('http://localhost:8080/api/corridors').json()
for corridor in corridors:
    print(f"{corridor['source_asset']} -> {corridor['destination_asset']}: {corridor['success_rate']}%")

# Create anchor
new_anchor = requests.post(
    'http://localhost:8080/api/anchors',
    json={
        'name': 'Circle',
        'stellar_account': 'GBBD...',
        'home_domain': 'circle.com'
    }
)
```

---

### Rust

```rust
use reqwest;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Get payments
    let response = client
        .get("http://localhost:8080/api/rpc/payments")
        .query(&[("limit", "10")])
        .send()
        .await?;
    
    let data: Value = response.json().await?;
    println!("{:#?}", data);
    
    // Get anchors
    let anchors: Value = client
        .get("http://localhost:8080/api/anchors")
        .send()
        .await?
        .json()
        .await?;
    
    println!("{:#?}", anchors);
    
    Ok(())
}
```

---

### cURL

```bash
# Health check
curl http://localhost:8080/health

# Get payments with pagination
curl "http://localhost:8080/api/rpc/payments?limit=50"

# Get order book
curl "http://localhost:8080/api/rpc/orderbook?\
selling_asset_type=credit_alphanum4&\
selling_asset_code=USDC&\
selling_asset_issuer=GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5&\
buying_asset_type=native"

# Create anchor
curl -X POST http://localhost:8080/api/anchors \
  -H "Content-Type: application/json" \
  -d '{"name":"Circle","stellar_account":"GBBD...","home_domain":"circle.com"}'

# Update anchor metrics
curl -X PUT http://localhost:8080/api/anchors/1/metrics \
  -H "Content-Type: application/json" \
  -d '{"total_transactions":1000,"successful_transactions":990}'
```

---

## 🔗 External Resources

- **OnFinality RPC:** https://stellar.api.onfinality.io/public
- **Stellar Horizon:** https://horizon.stellar.org
- **Stellar Documentation:** https://developers.stellar.org
- **Horizon API Docs:** https://developers.stellar.org/api/horizon
- **Stellar SDK:** https://github.com/stellar/js-stellar-sdk

---

## 🔐 Authentication

Currently, the API is open for development. Production deployment should implement:

- API key authentication
- Rate limiting per key
- IP whitelisting
- OAuth 2.0 for user-specific operations

See `FUTURE_TASKS.md` for authentication implementation plan.

---

## 📈 Rate Limiting

**Current Limits (Development):**
- No rate limiting

**Planned Limits (Production):**
- 100 requests/minute per IP
- 1000 requests/hour per API key
- Burst allowance: 20 requests

---

## 🐛 Error Handling

All errors follow this format:

```json
{
  "error": {
    "code": "INVALID_PARAMETER",
    "message": "Invalid account address format",
    "details": {
      "parameter": "account_id",
      "expected": "Stellar address starting with G"
    }
  }
}
```

**Common Error Codes:**
- `INVALID_PARAMETER` - Invalid request parameter
- `NOT_FOUND` - Resource not found
- `RPC_ERROR` - Stellar RPC connection error
- `DATABASE_ERROR` - Internal database error
- `RATE_LIMIT_EXCEEDED` - Too many requests

---

**For issues or questions, see:** `FUTURE_TASKS.md`
