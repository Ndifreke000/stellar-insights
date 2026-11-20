# Cost-Based Rate Limiting

## Overview

Stellar Insights now uses cost-based rate limiting for expensive operations instead of simple request counting. This approach better reflects the actual resource consumption of different requests.

## How It Works

### Cost Units

Each expensive operation is assigned a cost in "units" based on its resource requirements:

| Operation | Cost Formula | Example |
|-----------|-------------|---------|
| Corridor export (single) | `10 + (days * 1)` | 30-day range = 40 units |
| Corridor export (all) | `10000 + (days * 1)` | 30-day range = 10030 units |
| Analytics dashboard | 5 units per request | N/A |

### Client Budgets

Each client receives a daily budget based on their tier:

| Tier | Daily Budget |
|------|-------------|
| Anonymous | 100 units |
| Authenticated | 1,000 units |
| Premium | 10,000 units |

### Budget Refresh

Budgets refresh every 24 hours at midnight UTC.

## API Headers

All export endpoints now include cost-related headers in responses:

| Header | Description |
|--------|-------------|
| `X-Cost-Estimate` | Estimated cost units for this request |
| `X-Remaining-Budget` | Budget remaining after this request |
| `X-Total-Budget` | Client's total daily budget |
| `X-Tier` | Client's tier (Anonymous/Authenticated/Premium) |
| `X-Budget-Refresh-At` | Unix timestamp of budget refresh |

## Response Codes

### 200 OK
Request succeeded. Headers include cost information.

### 429 Too Many Requests
Client has exceeded their budget. Response includes:
- `Retry-After: 86400` (24 hours until budget refresh)
- Cost information in headers
- JSON body explaining the limit

```json
{
  "error": "RATE_LIMITED",
  "code": "INSUFFICIENT_BUDGET",
  "message": "Insufficient budget for this export. Estimated cost: 40.00, Remaining budget: -20.00",
  "retry_after_seconds": 86400
}
```

## Usage Examples

### Example 1: Single Corridor Export (30 days)

```bash
curl -i "http://localhost:8080/api/v1/export/corridors?format=csv&start_date=2024-11-01&end_date=2024-11-30"
```

**Response Headers:**
```
HTTP/1.1 200 OK
X-Cost-Estimate: 40
X-Remaining-Budget: 960
X-Total-Budget: 1000
X-Tier: Authenticated
X-Budget-Refresh-At: 1733078400
```

**Cost Breakdown:**
- Base cost: 10 units
- Date range: 30 days × 1 unit/day = 30 units
- **Total: 40 units**

### Example 2: All Corridors Export (365 days)

```bash
curl -i "http://localhost:8080/api/v1/export/corridors?format=csv&start_date=2024-01-01&end_date=2024-12-31"
```

**Response Headers:**
```
HTTP/1.1 429 Too Many Requests
Retry-After: 86400
X-Cost-Estimate: 10375
X-Remaining-Budget: -375
X-Total-Budget: 1000
X-Tier: Authenticated
X-Budget-Refresh-At: 1733078400
```

**Cost Breakdown:**
- Base cost: 10,000 units (worst case for all corridors)
- Date range: 365 days × 1 unit/day = 365 units
- **Total: 10,365 units**
- **Result: INSUFFICIENT BUDGET** (authenticated tier only has 1000 units)

### Example 3: Premium Client with Large Export

A premium client with 10,000 units budget can export:
- 10 single-corridor 30-day exports (10 × 40 = 400 units)
- 20 analytics dashboard requests (20 × 5 = 100 units)
- 9 single-corridor 365-day exports (9 × 375 = 3375 units)
- Or various combinations within their 10,000 unit budget

## Migration Guide

### For API Clients

1. **Check response headers** for cost information on every request
2. **Monitor `X-Remaining-Budget`** to track your usage
3. **Handle 429 responses** gracefully with exponential backoff (24h retry)
4. **Consider upgrading** to Premium tier for higher limits

### For Developers

The cost-based rate limiter is automatically applied to:
- `GET /api/v1/export/corridors`
- `GET /api/v1/export/anchors`
- `GET /api/v1/export/payments` (when implemented)
- `GET /analytics/dashboard`

No code changes required - the middleware handles everything.

## Configuration

Cost configuration is in `backend/src/models/cost.rs`:

```rust
pub struct CostConfig {
    pub per_corridor: f64,           // 10.0
    pub per_day: f64,                 // 1.0
    pub per_analytics_request: f64,   // 5.0
    pub default_anonymous_budget: f64,   // 100.0
    pub default_authenticated_budget: f64, // 1000.0
    pub default_premium_budget: f64,     // 10000.0
}
```

## Implementation Details

### Files Created

1. **`backend/src/models/cost.rs`** - Cost calculation models
2. **`backend/src/middleware/cost_rate_limiter.rs`** - Cost rate limiter middleware
3. **Updated `backend/src/api/export.rs`** - Cost headers and budget checking
4. **Updated `backend/src/state.rs`** - AppState with CostRateLimiter
5. **Updated `backend/src/middleware/mod.rs`** - Module exports

### Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ Request
       ↓
┌─────────────────┐
│  Cost Rate      │ Check budget
│  Limiter        │ Deduct cost
└──────┬──────────┘
       │
       ↓ (if budget OK)
┌─────────────────┐
│   Handler       │ Execute query
└──────┬──────────┘
       │
       ↓
┌─────────────────┐
│  Add Cost       │ Include cost headers
│  Headers        │ Return response
└─────────────────┘
```

## Testing

Run the cost model tests:

```bash
cd backend
cargo test --lib models::cost
cargo test --lib middleware::cost_rate_limiter
```

## Future Enhancements

1. **Per-API-key budget tracking** - Track budgets per API key instead of per IP
2. **Cost estimation endpoint** - Pre-flight endpoint to check cost before executing
3. **Budget top-up** - Allow clients to purchase additional units
4. **Analytics dashboard cost** - Apply cost-based rate limiting to analytics endpoints
5. **Redis-backed storage** - Use Redis for cost tracking across multiple server instances

## References

- Issue: #2387 (Permalink pinned to commit b17fcc2)
- Related: #2669 (Export row limits)
