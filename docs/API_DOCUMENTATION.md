# Stellar Insights API Documentation

## Overview

The Stellar Insights API provides real-time payment analytics, anchor monitoring, and cross-border payment corridor insights for the Stellar network.

**Base URL:** `https://api.stellarinsights.io`  
**API Version:** 1.0.0  
**OpenAPI Spec:** `/api-docs/openapi.json`

## Interactive API Explorer

Access the interactive Swagger UI at:
- **Development:** `http://localhost:8080/swagger-ui`
- **Production:** `https://api.stellarinsights.io/swagger-ui`

## Authentication

### API Key Authentication

Include your API key in the `Authorization` header:

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://api.stellarinsights.io/api/anchors
```

### OAuth 2.0

For third-party integrations, use OAuth 2.0:

```bash
# Get authorization code
GET /api/oauth/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=YOUR_REDIRECT_URI

# Exchange for access token
POST /api/oauth/token
Content-Type: application/json

{
  "grant_type": "authorization_code",
  "code": "AUTH_CODE",
  "client_id": "YOUR_CLIENT_ID",
  "client_secret": "YOUR_CLIENT_SECRET"
}
```

## Core Endpoints

### Anchors
- `GET /api/anchors` - List all anchors
- `GET /api/anchors/{id}` - Get anchor details
- `GET /api/anchors/account/{account}` - Get anchor by account
- `GET /api/anchors/{id}/muxed` - Get muxed account analytics

### Payment Corridors
- `GET /api/corridors` - List payment corridors
- `GET /api/corridors/{source}/{destination}` - Get corridor details
- `GET /api/corridors/{source}/{destination}/metrics` - Get corridor metrics

### Price Feed
- `GET /api/prices` - Get current asset prices
- `GET /api/prices/{asset}` - Get specific asset price
- `POST /api/prices/convert` - Convert between assets

### Cost Calculator
- `POST /api/cost-calculator/estimate` - Estimate payment costs
- `POST /api/cost-calculator/routes` - Compare payment routes

### Alerts
- `GET /api/alerts/rules` - List alert rules
- `POST /api/alerts/rules` - Create alert rule
- `PUT /api/alerts/rules/{id}` - Update alert rule
- `DELETE /api/alerts/rules/{id}` - Delete alert rule
- `GET /api/alerts/history` - Get alert history

### Webhooks
- `POST /api/webhooks` - Register webhook
- `GET /api/webhooks` - List webhooks
- `DELETE /api/webhooks/{id}` - Delete webhook
- `POST /api/webhooks/{id}/test` - Test webhook

## Request/Response Examples

### Get Anchor Details

**Request:**
```bash
curl -X GET "https://api.stellarinsights.io/api/anchors/anchor-123" \
  -H "Authorization: Bearer YOUR_API_KEY"
```

**Response:**
```json
{
  "id": "anchor-123",
  "name": "Example Anchor",
  "account": "GBUQWP3BOUZX34ULNQG23RQ6F4BVWCIYHBT6RIKCEWXC5ZAZMQG5HJ5",
  "domain": "example.com",
  "status": "active",
  "metrics": {
    "total_transactions": 15234,
    "success_rate": 0.9987,
    "avg_transaction_time_ms": 2340,
    "total_volume_usd": 5234000
  }
}
```

### Estimate Payment Costs

**Request:**
```bash
curl -X POST "https://api.stellarinsights.io/api/cost-calculator/estimate" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "source_asset": "USD:GBUQWP3BOUZX34ULNQG23RQ6F4BVWCIYHBT6RIKCEWXC5ZAZMQG5HJ5",
    "destination_asset": "EUR:GBUQWP3BOUZX34ULNQG23RQ6F4BVWCIYHBT6RIKCEWXC5ZAZMQG5HJ5",
    "amount": 1000
  }'
```

**Response:**
```json
{
  "routes": [
    {
      "path": ["USD", "EUR"],
      "total_cost": 15.50,
      "exchange_rate": 0.92,
      "fees": {
        "network_fee": 0.00001,
        "bridge_fee": 0.50,
        "liquidity_fee": 15.00
      },
      "estimated_time_ms": 3000
    }
  ]
}
```

## Error Handling

All errors follow a consistent format:

```json
{
  "error": "INVALID_REQUEST",
  "message": "Missing required parameter: source_asset",
  "status": 400,
  "request_id": "req-12345"
}
```

### Common Error Codes

| Code | Status | Description |
|------|--------|-------------|
| `INVALID_REQUEST` | 400 | Invalid request parameters |
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |

## Rate Limiting

Rate limits are applied per API key:

- **Anonymous:** 100 requests/minute
- **Authenticated:** 1,000 requests/minute
- **Premium:** 10,000 requests/minute

Rate limit headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1234567890
```

## Pagination

List endpoints support pagination:

```bash
GET /api/anchors?page=1&limit=50&sort=name&order=asc
```

**Response:**
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 50,
    "total": 1234,
    "pages": 25
  }
}
```

## WebSocket API

Real-time updates via WebSocket:

```javascript
const ws = new WebSocket('wss://api.stellarinsights.io/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'corridor:USD:EUR'
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Update:', data);
};
```

## SDKs and Libraries

- **JavaScript/TypeScript:** `npm install @stellar-insights/sdk`
- **Python:** `pip install stellar-insights`
- **Go:** `go get github.com/stellar-insights/go-sdk`

## Support

- **Documentation:** https://docs.stellarinsights.io
- **API Status:** https://status.stellarinsights.io
- **Support Email:** support@stellarinsights.io
- **GitHub Issues:** https://github.com/Ndifreke000/stellar-insights/issues

## Changelog

See [CHANGELOG.md](../CHANGELOG.md) for API version history and breaking changes.
