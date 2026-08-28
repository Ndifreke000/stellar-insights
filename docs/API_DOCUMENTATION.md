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
- `GET /api/corridors/{corridor_key}` - Get corridor details

### Contract Events
- `GET /api/analytics/verification-summary` - Get smart contract verification summary
- `GET /api/analytics/contract-events` - List contract events
- `GET /api/analytics/contract-events/{id}` - Get details of a contract event
- `GET /api/analytics/contract-events/epoch/{epoch}` - List contract events for a specific epoch
- `GET /api/analytics/event-stats` - Get contract event statistics

### SEP-31 Proxy
- `GET /api/sep31/info` - Get SEP-31 anchor info
- `POST /api/sep31/quote` - Create a SEP-31 payment quote
- `POST /api/sep31/transactions` - Create a SEP-31 transaction
- `GET /api/sep31/transactions` - List SEP-31 transactions
- `GET /api/sep31/transactions/{id}` - Get a specific SEP-31 transaction
- `GET /api/sep31/customer` - Get KYC customer information
- `PUT /api/sep31/customer` - Update KYC customer information
- `GET /api/sep31/anchors` - List configured SEP-31 anchors

### Snapshots
- `POST /api/snapshots/generate` - Generate a ledger state snapshot
- `GET /api/snapshots/contract/health` - Check health of snapshot contract service

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

The API implements a three-tier rate limiting system based on client authentication status and subscription level.

### Rate Limit Tiers

| Tier | Limit | Default Requests/Minute |
|------|-------|-------------------------|
| Anonymous | Per IP address (IPv6 masked to /48) | 60 |
| Authenticated | Per API key or user account | 200 |
| Premium | Paid subscription tier | 1,000 |

**How to upgrade your tier:**

1. **Authenticated tier:** Create an account and generate an API key
   - Include key in `Authorization: Bearer YOUR_API_KEY` header
   - Keys follow format `si_live_*` (production) or `si_test_*` (testing)

2. **Premium tier:** Contact support@stellarinsights.io with your use case
   - Specify your expected request volume
   - Include your API key ID
   - Include organization name and contact information

### Per-Endpoint Rate Limits

Some endpoints have stricter limits due to computational cost:

| Endpoint | Authenticated | Premium |
|----------|---------------|---------|
| `/api/export/csv` | 10/min | 20/min |
| `/api/export/excel` | 10/min | 20/min |
| `/api/analytics` | 40/min | 100/min |
| `/api/rpc` | 200/min | 500/min |

### Response Headers

Every API response includes rate limit information in headers:

```
RateLimit-Limit: 200                          # Your current limit
RateLimit-Remaining: 195                      # Requests remaining in current window
RateLimit-Reset: 1234567890                   # Unix timestamp when limit resets
X-RateLimit-Policy: 200 requests per 60 seconds
X-RateLimit-Client: api-key-abc123            # Your client ID (authenticated requests)
```

When you exceed your rate limit, the response includes:

```
HTTP/1.1 429 Too Many Requests
Retry-After: 45                                # Seconds to wait before retrying
RateLimit-Limit: 200
RateLimit-Remaining: 0
RateLimit-Reset: 1234567890
X-RateLimit-Policy: 200 requests per 60 seconds

{
  "error": "Rate limit exceeded",
  "limit": 200,
  "reset_after": 45
}
```

### Handling Rate Limits

**Recommended client implementation:**

```javascript
// Example: JavaScript client with retry logic
async function fetchWithRateLimit(url, options = {}) {
  const response = await fetch(url, options);
  
  // Check rate limit headers
  const limit = response.headers.get('RateLimit-Limit');
  const remaining = response.headers.get('RateLimit-Remaining');
  const reset = response.headers.get('RateLimit-Reset');
  
  if (response.status === 429) {
    const retryAfter = response.headers.get('Retry-After');
    const delaySeconds = parseInt(retryAfter || '60');
    
    console.warn(`Rate limited. Waiting ${delaySeconds}s before retry`);
    await new Promise(resolve => setTimeout(resolve, delaySeconds * 1000));
    
    // Retry the request
    return fetchWithRateLimit(url, options);
  }
  
  // Log remaining requests
  if (remaining) {
    console.log(`Requests remaining: ${remaining}/${limit}`);
  }
  
  return response.json();
}
```

```python
# Example: Python client with backoff
import requests
import time

def fetch_with_rate_limit(url, headers=None):
    response = requests.get(url, headers=headers)
    
    # Check rate limit headers
    limit = response.headers.get('RateLimit-Limit')
    remaining = response.headers.get('RateLimit-Remaining')
    
    if response.status_code == 429:
        retry_after = int(response.headers.get('Retry-After', 60))
        print(f"Rate limited. Waiting {retry_after}s before retry...")
        time.sleep(retry_after)
        return fetch_with_rate_limit(url, headers)
    
    # Log remaining requests
    if remaining:
        print(f"Requests remaining: {remaining}/{limit}")
    
    return response.json()
```

### Best Practices

1. **Monitor your usage:** Check `RateLimit-Remaining` header to avoid hitting the limit
2. **Implement backoff:** Use `Retry-After` header value when implementing retries
3. **Batch requests:** Group multiple queries into single requests when possible
4. **Cache results:** Store responses with appropriate TTLs to reduce API calls
5. **Contact support:** If you consistently hit rate limits, request a higher tier

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
