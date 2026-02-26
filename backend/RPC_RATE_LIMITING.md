# RPC/Horizon Outbound Rate Limiting

The backend now applies outbound throttling to every Stellar RPC/Horizon HTTP call made by `StellarRpcClient`.

## Behavior

- Token-bucket limiter with configurable refill rate and burst capacity.
- Bounded request queue (`RPC_RATE_LIMIT_QUEUE_SIZE`) for backpressure.
- Automatic parsing of `X-RateLimit-Limit`, `X-RateLimit-Remaining`, and `Retry-After` headers.
- Automatic 429 handling with wait/backoff before retry.
- Internal metrics counters for total requests, throttled requests, rejected (queue-full) requests, and observed 429 responses.

## Environment Variables

```env
RPC_RATE_LIMIT_REQUESTS_PER_MINUTE=90
RPC_RATE_LIMIT_BURST_SIZE=10
RPC_RATE_LIMIT_QUEUE_SIZE=100
```
