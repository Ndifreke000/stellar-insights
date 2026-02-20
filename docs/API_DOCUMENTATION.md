# Stellar Insights API Documentation

## Overview

The Stellar Insights API provides comprehensive analytics for the Stellar network, including anchor monitoring, payment corridor insights, and real-time metrics.

**Base URL:** `http://localhost:8080` (development) | `https://api.stellarinsights.io` (production)

**Interactive Documentation:** Visit `/api/docs` for Swagger UI

## Authentication

Most endpoints are public. Protected endpoints require JWT authentication.

### Authentication Flow

1. **Login:** `POST /api/auth/login`
2. **Refresh Token:** `POST /api/auth/refresh`
3. **Logout:** `POST /api/auth/logout`

### Using Authentication

Include the JWT token in the Authorization header:

```
Authorization: Bearer <your_jwt_token>
```

## Error Codes

All endpoints follow standard HTTP status codes:

| Status Code | Description |
|-------------|-------------|
| 200 | Success |
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Missing or invalid authentication |
| 404 | Not Found - Resource doesn't exist |
| 429 | Too Many Requests - Rate limit exceeded |
| 500 | Internal Server Error |

### Error Response Format

```json
{
  "error": "Error message description",
  "code": "ERROR_CODE",
  "details": {}
}
```

## Rate Limiting

Rate limits are applied per IP address:

| Endpoint Pattern | Requests per Minute |
|------------------|---------------------|
| `/health` | 1000 |
| `/api/anchors` | 100 |
| `/api/corridors` | 100 |
| `/api/rpc/*` | 100 |

Rate limit headers are included in responses:
- `X-RateLimit-Limit`: Maximum requests allowed
- `X-RateLimit-Remaining`: Requests remaining
- `X-RateLimit-Reset`: Time when limit resets (Unix timestamp)

## Endpoints

### Health Check

#### GET /health

Check API health status.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

---

### Anchors

#### GET /api/anchors

List all anchors with key metrics.

**Query Parameters:**
- `limit` (optional, integer): Maximum number of results (default: 50, max: 100)
- `offset` (optional, integer): Pagination offset (default: 0)

**Response:**
```json
{
  "anchors": [
    {
      "id": "uuid",
      "name": "Anchor Name",
      "stellar_account": "GXXXXXXX...",
      "asset_count": 5,
      "total_volume_24h": 1000000.50,
      "transaction_count_24h": 150,
      "avg_settlement_time": 3.5,
      "success_rate": 0.98,
      "last_updated": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 25
}
```

**Data Source:** RPC + Database
- Anchor metadata from database
- Transaction metrics from RPC payment data

#### GET /api/anchors/:id

Get detailed information for a specific anchor.

**Path Parameters:**
- `id` (required, UUID): Anchor ID

**Response:**
```json
{
  "id": "uuid",
  "name": "Anchor Name",
  "stellar_account": "GXXXXXXX...",
  "home_domain": "example.com",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### GET /api/anchors/account/:stellar_account

Get anchor by Stellar account address.

**Path Parameters:**
- `stellar_account` (required, string): Stellar account address (G...)

**Response:** Same as GET /api/anchors/:id

#### GET /api/anchors/:id/assets

List all assets issued by an anchor.

**Path Parameters:**
- `id` (required, UUID): Anchor ID

**Response:**
```json
[
  {
    "id": "uuid",
    "anchor_id": "uuid",
    "asset_code": "USDC",
    "asset_issuer": "GXXXXXXX...",
    "total_supply": 10000000.00,
    "holders_count": 5000,
    "created_at": "2024-01-01T00:00:00Z"
  }
]
```

#### POST /api/anchors

Create a new anchor. **Requires authentication.**

**Request Body:**
```json
{
  "name": "New Anchor",
  "stellar_account": "GXXXXXXX...",
  "home_domain": "example.com"
}
```

**Response:** 201 Created with anchor object

#### PUT /api/anchors/:id/metrics

Update anchor metrics. **Requires authentication.**

**Path Parameters:**
- `id` (required, UUID): Anchor ID

**Request Body:**
```json
{
  "total_volume_24h": 1000000.50,
  "transaction_count_24h": 150,
  "avg_settlement_time": 3.5,
  "success_rate": 0.98
}
```

**Response:** 200 OK

#### POST /api/anchors/:id/assets

Add an asset to an anchor. **Requires authentication.**

**Path Parameters:**
- `id` (required, UUID): Anchor ID

**Request Body:**
```json
{
  "asset_code": "USDC",
  "asset_issuer": "GXXXXXXX..."
}
```

**Response:** 201 Created

---

### Corridors

#### GET /api/corridors

List all payment corridors with performance metrics.

**Query Parameters:**
- `limit` (optional, integer): Maximum number of results (default: 50)
- `offset` (optional, integer): Pagination offset (default: 0)
- `success_rate_min` (optional, float): Minimum success rate filter (0.0-1.0)
- `success_rate_max` (optional, float): Maximum success rate filter (0.0-1.0)
- `volume_min` (optional, float): Minimum 24h volume filter
- `volume_max` (optional, float): Maximum 24h volume filter
- `asset_code` (optional, string): Filter by asset code
- `time_period` (optional, string): Time period for metrics (24h, 7d, 30d)

**Response:**
```json
[
  {
    "corridor_key": "USDC-USD",
    "source_asset": "USDC",
    "destination_asset": "USD",
    "volume_24h": 500000.00,
    "transaction_count": 250,
    "avg_latency_seconds": 4.2,
    "success_rate": 0.97,
    "liquidity_depth": 1000000.00,
    "last_updated": "2024-01-15T10:30:00Z"
  }
]
```

**Data Source:** RPC
- Payment data from Horizon API
- Trade data from Horizon API
- Order book data from Horizon API

#### GET /api/corridors/:corridor_key

Get detailed corridor information with historical data.

**Path Parameters:**
- `corridor_key` (required, string): Corridor identifier (e.g., "USDC-USD")

**Response:**
```json
{
  "corridor_key": "USDC-USD",
  "source_asset": "USDC",
  "destination_asset": "USD",
  "current_metrics": {
    "volume_24h": 500000.00,
    "transaction_count": 250,
    "avg_latency_seconds": 4.2,
    "success_rate": 0.97
  },
  "historical_data": {
    "success_rate": [
      {"timestamp": "2024-01-15T00:00:00Z", "value": 0.97}
    ],
    "latency": [
      {"timestamp": "2024-01-15T00:00:00Z", "value": 4.2}
    ],
    "liquidity": [
      {"timestamp": "2024-01-15T00:00:00Z", "value": 1000000.00}
    ]
  }
}
```

#### POST /api/corridors

Create a new corridor. **Requires authentication.**

**Request Body:**
```json
{
  "source_asset": "USDC",
  "destination_asset": "USD",
  "source_anchor_id": "uuid",
  "destination_anchor_id": "uuid"
}
```

**Response:** 201 Created

#### PUT /api/corridors/:id/metrics-from-transactions

Update corridor metrics from transaction data. **Requires authentication.**

**Path Parameters:**
- `id` (required, UUID): Corridor ID

**Response:** 200 OK

---

### RPC Endpoints

#### GET /api/rpc/health

Check RPC connection health.

**Response:**
```json
{
  "status": "healthy",
  "rpc_url": "https://stellar.api.onfinality.io/public",
  "horizon_url": "https://horizon.stellar.org",
  "mock_mode": false
}
```

#### GET /api/rpc/ledger/latest

Get latest ledger information.

**Response:**
```json
{
  "sequence": 12345678,
  "hash": "abc123...",
  "closed_at": "2024-01-15T10:30:00Z",
  "transaction_count": 150
}
```

#### GET /api/rpc/payments

Get recent payments from the network.

**Query Parameters:**
- `limit` (optional, integer): Maximum number of results (default: 10, max: 200)
- `cursor` (optional, string): Pagination cursor

**Response:**
```json
[
  {
    "id": "payment_id",
    "from": "GXXXXXXX...",
    "to": "GYYYYYYY...",
    "asset_code": "USDC",
    "asset_issuer": "GZZZZZZZ...",
    "amount": "100.00",
    "created_at": "2024-01-15T10:30:00Z"
  }
]
```

#### GET /api/rpc/payments/account/:account_id

Get payments for a specific account.

**Path Parameters:**
- `account_id` (required, string): Stellar account address

**Query Parameters:**
- `limit` (optional, integer): Maximum number of results (default: 10, max: 200)

**Response:** Same as GET /api/rpc/payments

#### GET /api/rpc/trades

Get recent trades from the network.

**Query Parameters:**
- `limit` (optional, integer): Maximum number of results (default: 10, max: 200)
- `cursor` (optional, string): Pagination cursor

**Response:**
```json
[
  {
    "id": "trade_id",
    "base_asset": "USDC",
    "counter_asset": "XLM",
    "base_amount": "100.00",
    "counter_amount": "1000.00",
    "price": "10.00",
    "created_at": "2024-01-15T10:30:00Z"
  }
]
```

#### GET /api/rpc/orderbook

Get order book for an asset pair.

**Query Parameters:**
- `selling_asset_code` (required, string): Selling asset code
- `selling_asset_issuer` (optional, string): Selling asset issuer (omit for XLM)
- `buying_asset_code` (required, string): Buying asset code
- `buying_asset_issuer` (optional, string): Buying asset issuer (omit for XLM)
- `limit` (optional, integer): Maximum number of orders per side (default: 20)

**Response:**
```json
{
  "bids": [
    {"price": "10.00", "amount": "1000.00"}
  ],
  "asks": [
    {"price": "10.10", "amount": "500.00"}
  ]
}
```

---

### Fee Bump Tracking

#### GET /api/fee-bumps

List fee bump transactions.

**Query Parameters:**
- `limit` (optional, integer): Maximum number of results
- `status` (optional, string): Filter by status (pending, confirmed, failed)

**Response:**
```json
[
  {
    "id": "uuid",
    "original_tx_hash": "abc123...",
    "bumped_tx_hash": "def456...",
    "original_fee": 100,
    "bumped_fee": 200,
    "status": "confirmed",
    "created_at": "2024-01-15T10:30:00Z"
  }
]
```

#### GET /api/fee-bumps/:id

Get details of a specific fee bump transaction.

**Path Parameters:**
- `id` (required, UUID): Fee bump transaction ID

**Response:** Single fee bump object

---

### Cache Management

#### GET /api/cache/stats

Get cache statistics and hit rates.

**Response:**
```json
{
  "total_requests": 10000,
  "cache_hits": 8500,
  "cache_misses": 1500,
  "hit_rate": 0.85,
  "total_keys": 150
}
```

#### POST /api/cache/reset

Reset cache statistics.

**Response:**
```json
{
  "message": "Cache statistics reset successfully"
}
```

---

### Metrics

#### GET /api/metrics/overview

Get system-wide metrics overview.

**Response:**
```json
{
  "total_anchors": 25,
  "total_corridors": 150,
  "total_volume_24h": 10000000.00,
  "total_transactions_24h": 5000,
  "avg_success_rate": 0.96,
  "avg_settlement_time": 4.5
}
```

---

## Data Sources

The API integrates data from multiple sources:

1. **Database:** Anchor metadata, user data, historical aggregates
2. **Stellar RPC:** Real-time payment, trade, and ledger data
3. **Horizon API:** Historical transaction data
4. **Cache:** Redis cache for performance optimization

## Caching Strategy

- Anchor list: 5 minutes TTL
- Corridor list: 2 minutes TTL
- Metrics overview: 1 minute TTL
- Individual resources: 5 minutes TTL

Cache keys include query parameters to ensure accurate filtering.

## WebSocket Support

Real-time updates are available via WebSocket connection (implementation in progress).

## SDK Examples

### JavaScript/TypeScript

```typescript
const response = await fetch('http://localhost:8080/api/anchors?limit=10');
const data = await response.json();
console.log(data.anchors);
```

### Python

```python
import requests

response = requests.get('http://localhost:8080/api/anchors', params={'limit': 10})
data = response.json()
print(data['anchors'])
```

### cURL

```bash
curl -X GET "http://localhost:8080/api/anchors?limit=10"
```

## Support

For issues or questions:
- Email: support@stellarinsights.io
- GitHub: [Repository URL]
- Documentation: http://localhost:8080/api/docs
