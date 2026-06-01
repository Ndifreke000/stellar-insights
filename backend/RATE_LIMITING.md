# REST API Rate Limiting per Client

## Overview

The Stellar Insights backend implements sophisticated per-client rate limiting to prevent abuse and ensure fair usage across all API consumers. Rate limits are applied based on client identity (API key, authenticated user, or IP address) with different tiers for different client types.

## Features

### Client Identification

The rate limiter identifies clients using the following hierarchy:

1. **API Key** - Clients using API keys (`si_live_*` or `si_test_*` format)
2. **Authenticated User** - Users authenticated via JWT tokens
3. **IP Address** - Anonymous clients identified by their IP address (fallback)

### Rate Limit Tiers

Three tiers of rate limits are supported:

| Tier | Description | Default Limit (req/min) |
|------|-------------|------------------------|
| **Anonymous** | Unauthenticated requests identified by IP | 60 |
| **Authenticated** | Requests with valid API key or JWT token | 200 |
| **Premium** | Premium tier clients (future enhancement) | 1000 |

### Per-Endpoint Configuration

Each endpoint can have its own rate limit configuration with tier-specific limits:

```rust
RateLimitConfig {
    requests_per_minute: 100,  // Base limit (legacy)
    whitelist_ips: vec![],     // IPs exempt from rate limiting
    client_limits: Some(ClientRateLimits {
        authenticated: 200,     // Limit for authenticated clients
        premium: 1000,          // Limit for premium clients
        anonymous: 60,          // Limit for anonymous clients
    }),
}
```

## Current Endpoint Limits

| Endpoint | Anonymous | Authenticated | Premium |
|----------|-----------|---------------|---------|
| `/health` | 1000 | 1000 | 5000 |
| `/api/anchors` | 60 | 200 | 1000 |
| `/api/corridors` | 60 | 200 | 1000 |
| `/api/rpc/payments` | 50 | 300 | 2000 |
| `/api/rpc/trades` | 50 | 300 | 2000 |
| `/api/liquidity-pools` | 60 | 200 | 1000 |
| `/api/prices` | 60 | 300 | 1500 |
| `/api/account-merges` | 60 | 200 | 1000 |
| `/api/achievements` | 60 | 200 | 1000 |

## Usage

### For API Consumers

#### Using API Keys

Include your API key in the `Authorization` header:

```bash
curl -H "Authorization: Bearer si_live_your_api_key_here" \
  https://api.stellar-insights.com/api/anchors
```

API key authenticated requests receive higher rate limits (200-300 req/min vs 50-60 req/min for anonymous).

#### Using JWT Tokens

For user-authenticated requests, include your JWT token:

```bash
curl -H "Authorization: Bearer your_jwt_token_here" \
  https://api.stellar-insights.com/api/anchors
```

#### Anonymous Requests

Requests without authentication are rate limited by IP address with lower limits:

```bash
curl https://api.stellar-insights.com/api/anchors
```

### Rate Limit Headers

All responses include rate limit information in headers:

```
RateLimit-Limit: 200
RateLimit-Remaining: 195
RateLimit-Reset: 60
X-RateLimit-Client: apikey:abc123...
```

- `RateLimit-Limit`: Maximum requests allowed per minute
- `RateLimit-Remaining`: Requests remaining in current window
- `RateLimit-Reset`: Seconds until the rate limit resets
- `X-RateLimit-Client`: Client identifier (for debugging)

### Rate Limit Exceeded Response

When rate limit is exceeded, the API returns:

```json
HTTP/1.1 429 Too Many Requests
RateLimit-Limit: 60
RateLimit-Remaining: 0
RateLimit-Reset: 45

{
  "error": "Rate limit exceeded",
  "limit": 60,
  "reset_after": 45
}
```

## Implementation Details

### Architecture

```
Request → Extract Client ID → Check Tier → Apply Limit → Redis/Memory Store
```

1. **Client Extraction**: Middleware extracts client identifier from request headers
2. **Tier Detection**: Determines client tier (Anonymous/Authenticated/Premium)
3. **Limit Selection**: Selects appropriate rate limit based on tier
4. **Storage**: Uses Redis for distributed rate limiting, falls back to in-memory store

### Storage Backend

- **Primary**: Redis with sliding window algorithm
- **Fallback**: In-memory HashMap for development/testing
- **Key Format**: `ratelimit:{endpoint}:{client_type}:{client_id}`

### API Key Validation

API keys are validated against the database:
- Hash comparison for security
- Status check (active/revoked)
- Expiration check
- Last used timestamp update

## Configuration

### Environment Variables

```bash
# Redis connection for distributed rate limiting
REDIS_URL=redis://127.0.0.1:6379

# Database for API key validation
DATABASE_URL=sqlite:./stellar_insights.db
```

### Programmatic Configuration

```rust
// Initialize rate limiter with database support
let rate_limiter = RateLimiter::new_with_db(Some(pool.clone())).await?;

// Register endpoint with custom limits
rate_limiter.register_endpoint(
    "/api/custom".to_string(),
    RateLimitConfig {
        requests_per_minute: 100,
        whitelist_ips: vec!["10.0.0.1".to_string()],
        client_limits: Some(ClientRateLimits {
            authenticated: 500,
            premium: 2000,
            anonymous: 100,
        }),
    },
).await;
```

## IP Whitelisting

Certain IPs can be whitelisted to bypass rate limiting:

```rust
RateLimitConfig {
    whitelist_ips: vec![
        "127.0.0.1".to_string(),      // Localhost
        "10.0.0.0/8".to_string(),     // Internal network (future)
    ],
    // ...
}
```

Whitelisted IPs:
- Bypass all rate limit checks
- Receive `is_whitelisted: true` in rate limit info
- Useful for internal services and monitoring

## Testing

Run the rate limiting test suite:

```bash
cd backend
cargo test rate_limit
```

Key test scenarios:
- Anonymous client rate limiting
- Authenticated client higher limits
- Different clients are independent
- Different endpoints have separate limits
- IP whitelist bypass
- Rate limit info includes client ID

## Future Enhancements

### Premium Tier Detection

Currently, all authenticated clients receive the same tier. Future enhancement:

```rust
async fn get_client_tier(&self, client: &ClientIdentifier) -> ClientTier {
    match client {
        ClientIdentifier::ApiKey(key_id) => {
            // Query database for subscription tier
            if self.is_premium_subscriber(key_id).await {
                ClientTier::Premium
            } else {
                ClientTier::Authenticated
            }
        }
        // ...
    }
}
```

### Dynamic Rate Limit Adjustment

- Adjust limits based on system load
- Burst allowance for short spikes
- Time-of-day based limits

### Rate Limit Analytics

- Track rate limit hits per client
- Identify clients approaching limits
- Alert on suspicious patterns

### GraphQL Rate Limiting

- Query complexity-based rate limiting
- Depth and breadth limits
- Cost-based rate limiting

## Security Considerations

1. **API Key Security**: Keys are hashed before storage, never logged in plain text
2. **Fail Closed**: If Redis is unavailable, falls back to memory store (not fail open)
3. **Client Isolation**: Each client's rate limit is independent
4. **Header Sanitization**: Client IDs in headers are sanitized to prevent injection

## Monitoring

Monitor rate limiting effectiveness:

```bash
# Check Redis rate limit keys
redis-cli KEYS "ratelimit:*"

# Monitor rate limit hits
grep "Rate limit exceeded" /var/log/stellar-insights/app.log

# Check rate limiter metrics (if Prometheus enabled)
curl http://localhost:8080/metrics | grep rate_limit
```

## Troubleshooting

### Issue: Rate limits too restrictive

**Solution**: Authenticate with an API key to receive higher limits

### Issue: Rate limit not resetting

**Check**: Redis connection and TTL settings
```bash
redis-cli TTL "ratelimit:/api/anchors:ip:192.168.1.1"
```

### Issue: Different limits on different servers

**Cause**: Using memory fallback instead of Redis
**Solution**: Ensure Redis is running and `REDIS_URL` is configured

## References

- [API Key Management](./src/api/api_keys.rs)
- [Rate Limiter Implementation](./src/rate_limit.rs)
- [Authentication Middleware](./src/auth_middleware.rs)
- [Security Audit](./SECURITY_AUDIT.md)
