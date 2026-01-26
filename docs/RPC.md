
# 🌐 Stellar Insights RPC API Documentation

Complete API reference for accessing real-time Stellar blockchain data and analytics.

---

## 📡 Base URL

**Local Development:**
```
http://localhost:8080
```

**Production:**
```
https://your-domain.com
```

---

## 🔌 RPC Endpoints

### Health Check

#### `GET /api/rpc/health`

Check Stellar RPC connection health and get network status.

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

#### `GET /api/rpc/ledger/latest`

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

#### `GET /api/rpc/payments`

Fetch recent payment operations from the Stellar network.

**Query Parameters:**
- `limit` (optional): Number of records to return (default: 20, max: 200)
- `cursor` (optional): Pagination cursor for next page

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
curl http://localhost:8080/api/rpc/payments?cursor=123456789&limit=20
```

---

### Account Payments

#### `GET /api/rpc/payments/account/:account_id`

Get payment history for a specific Stellar account.

**Path Parameters:**
- `account_id` (required): Stellar account address (G...)

**Query Parameters:**
- `limit` (optional): Number of records (default: 20)

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

#### `GET /api/rpc/trades`

Fetch recent trade operations from the Stellar DEX.

**Query Parameters:**
- `limit` (optional): Number of records (default: 20)
- `cursor` (optional): Pagination cursor

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

#### `GET /api/rpc/orderbook`

Get order book for a specific trading pair.

**Query Parameters:**
- `selling_asset_type` (required): "native" or "credit_alphanum4" or "credit_alphanum12"
- `selling_asset_code` (optional): Asset code (required if not native)
- `selling_asset_issuer` (optional): Issuer address (required if not native)
- `buying_asset_type` (required): Asset type
- `buying_asset_code` (optional): Asset code
- `buying_asset_issuer` (optional): Issuer address
- `limit` (optional): Number of price levels (default: 20)

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

#### `GET /api/anchors`

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

#### `GET /api/anchors/:id`

Get detailed information for a specific anchor.

**Example:**
```bash
curl http://localhost:8080/api/anchors/1
```

---

#### `GET /api/anchors/account/:stellar_account`

Get anchor by Stellar account address.

**Example:**
```bash
curl http://localhost:8080/api/anchors/account/GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

---

#### `POST /api/anchors`

Create a new anchor.

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

#### `PUT /api/anchors/:id/metrics`

Update anchor performance metrics.

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

#### `GET /api/corridors`

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

#### `GET /api/corridors/:corridor_key`

Get detailed metrics for a specific corridor.

**Path Parameters:**
- `corridor_key`: Format `SOURCE_ASSET:DEST_ASSET` (e.g., `USDC:GBBD..._XLM:native`)

**Example:**
```bash
curl http://localhost:8080/api/corridors/USDC:GBBD..._XLM:native
```

---

#### `POST /api/corridors`

Create a new corridor for tracking.

**Request Body:**
```json
{
  "source_asset": "USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5",
  "destination_asset": "XLM:native"
}
```

---

## 🔧 Configuration

### Environment Variables

Create a `.env` file in the backend directory:

```env
# Database
DATABASE_URL=sqlite:stellar_insights.db

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080

# Stellar RPC
STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
STELLAR_HORIZON_URL=https://horizon.stellar.org

# Mock Mode (for testing without real RPC calls)
RPC_MOCK_MODE=false

# Logging
RUST_LOG=info
```

---

## 🚀 Quick Start

### 1. Start the Backend

```bash
cd backend
cargo run
```

Server will start on `http://localhost:8080`

### 2. Test the API

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

## 🧪 Mock Mode

For development without hitting the real Stellar network:

```env
RPC_MOCK_MODE=true
```

This will return mock data for all RPC endpoints.

---

## 📝 Response Codes

| Code | Description |
|------|-------------|
| 200 | Success |
| 400 | Bad Request - Invalid parameters |
| 404 | Not Found - Resource doesn't exist |
| 500 | Internal Server Error |
| 503 | Service Unavailable - RPC connection failed |

---

## 🔗 External Resources

- **OnFinality RPC:** https://stellar.api.onfinality.io/public
- **Stellar Horizon:** https://horizon.stellar.org
- **Stellar Documentation:** https://developers.stellar.org
- **Horizon API Docs:** https://developers.stellar.org/api/horizon

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
```

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
```

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
    
    Ok(())
}
```

---

**Last Updated:** January 26, 2026  
**API Version:** v0.1.0

