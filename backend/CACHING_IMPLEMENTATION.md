# Redis Caching Implementation

## Overview

This document describes the Redis caching layer implemented for the Stellar Insights backend to reduce database load and improve API response times.

## Architecture

### Components

1. **CacheManager** (`src/cache.rs`)
   - Core caching abstraction with Redis backend
   - Graceful fallback when Redis is unavailable
   - TTL-based cache expiration
   - Cache statistics tracking (hits, misses, invalidations)

2. **CacheInvalidationService** (`src/cache_invalidation.rs`)
   - Manages cache invalidation on data updates
   - Pattern-based invalidation for related caches
   - Integrated with data ingestion pipeline

3. **CacheAware Trait** (`src/cache_middleware.rs`)
   - Helper trait for cache-aware operations
   - Implements get-or-fetch pattern
   - Automatic cache population on miss

4. **Cache Statistics Endpoint** (`src/api/cache_stats.rs`)
   - Monitoring endpoint: `GET /api/cache/stats`
   - Reset endpoint: `POST /api/cache/reset`
   - Real-time hit rate monitoring

### Cached Endpoints

#### 1. Anchor Metrics (10 min TTL)
- **Endpoint**: `GET /api/anchors`
- **Cache Key**: `anchor:list:{limit}:{offset}`
- **TTL**: 600 seconds (10 minutes)
- **Invalidation**: On anchor metrics update, every 5 minutes during sync

**Response Structure**:
```json
{
  "anchors": [
    {
      "id": "uuid",
      "name": "Anchor Name",
      "stellar_account": "GA...",
      "reliability_score": 95.5,
      "asset_coverage": 3,
      "failure_rate": 5.0,
      "total_transactions": 1000,
      "successful_transactions": 950,
      "failed_transactions": 50,
      "status": "green"
    }
  ],
  "total": 1
}
```

#### 2. Corridor Metrics (5 min TTL)
- **Endpoint**: `GET /api/corridors`
- **Cache Key**: `corridor:list:{limit}:{offset}:{filters}`
- **TTL**: 300 seconds (5 minutes)
- **Invalidation**: On corridor metrics update, every 5 minutes during sync

**Response Structure**:
```json
[
  {
    "id": "EURC:issuer->USDC:issuer",
    "source_asset": "EURC",
    "destination_asset": "USDC",
    "success_rate": 95.0,
    "total_attempts": 1000,
    "successful_payments": 950,
    "failed_payments": 50,
    "average_latency_ms": 400.0,
    "median_latency_ms": 300.0,
    "p95_latency_ms": 1000.0,
    "p99_latency_ms": 1600.0,
    "liquidity_depth_usd": 1000000.0,
    "liquidity_volume_24h_usd": 100000.0,
    "liquidity_trend": "stable",
    "health_score": 95.0,
    "last_updated": "2026-01-28T10:00:00Z"
  }
]
```

#### 3. Corridor Details (5 min TTL)
- **Endpoint**: `GET /api/corridors/:corridor_key`
- **Cache Key**: `corridor:detail:{corridor_key}`
- **TTL**: 300 seconds (5 minutes)
- **Invalidation**: On corridor metrics update

**Response Structure**:
```json
{
  "corridor": { /* CorridorResponse */ },
  "historical_success_rate": [
    {
      "timestamp": "2026-01-28",
      "success_rate": 95.0,
      "attempts": 1000
    }
  ],
  "latency_distribution": [
    {
      "latency_bucket_ms": 100,
      "count": 250,
      "percentage": 15.0
    }
  ],
  "liquidity_trends": [
    {
      "timestamp": "2026-01-28",
      "liquidity_usd": 1000000.0,
      "volume_24h_usd": 100000.0
    }
  ],
  "related_corridors": [ /* CorridorResponse[] */ ]
}
```

#### 4. Dashboard Stats (1 min TTL)
- **Endpoint**: `GET /api/metrics/overview`
- **Cache Key**: `dashboard:stats`
- **TTL**: 60 seconds (1 minute)
- **Invalidation**: On metrics update

**Response Structure**:
```json
{
  "total_volume": 1234567.89,
  "total_transactions": 98765,
  "active_users": 4321,
  "average_transaction_value": 28.56,
  "corridor_count": 12
}
```

### Cache Statistics Endpoint

#### GET /api/cache/stats
Returns real-time cache performance metrics.

**Response**:
```json
{
  "hits": 1250,
  "misses": 250,
  "invalidations": 45,
  "hit_rate_percent": 83.33,
  "total_requests": 1500
}
```

#### POST /api/cache/reset
Resets cache statistics counters.

**Response**:
```json
{
  "status": "success",
  "message": "Cache statistics reset"
}
```

## Configuration

### Environment Variables

```bash
# Redis connection URL (optional, defaults to redis://127.0.0.1:6379)
REDIS_URL=redis://localhost:6379

# Cache TTL settings (in seconds, can be customized)
# Default values:
# - Corridor metrics: 300 (5 minutes)
# - Anchor data: 600 (10 minutes)
# - Dashboard stats: 60 (1 minute)
```

### Cache Configuration

Edit `CacheConfig` in `src/cache.rs` to customize TTL values:

```rust
pub struct CacheConfig {
    pub corridor_metrics_ttl: usize,    // 5 minutes
    pub anchor_data_ttl: usize,         // 10 minutes
    pub dashboard_stats_ttl: usize,     // 1 minute
}
```

## Cache Invalidation Strategy

### Automatic Invalidation

1. **On Data Ingestion** (every 5 minutes)
   - Anchor caches invalidated after metrics sync
   - Corridor caches invalidated after metrics sync
   - Metrics caches invalidated

2. **Pattern-Based Invalidation**
   - `anchor:*` - All anchor-related caches
   - `corridor:*` - All corridor-related caches
   - `dashboard:*` - All dashboard caches

### Manual Invalidation

Use `CacheInvalidationService` for manual cache clearing:

```rust
let invalidation = CacheInvalidationService::new(cache.clone());

// Invalidate specific anchor
invalidation.invalidate_anchor("anchor-id").await?;

// Invalidate all anchors
invalidation.invalidate_anchors().await?;

// Invalidate specific corridor
invalidation.invalidate_corridor("corridor-key").await?;

// Full cache invalidation
invalidation.invalidate_all().await?;
```

## Fallback Behavior

### Redis Unavailable

If Redis is unavailable:
1. Cache operations return `Ok(None)` for reads
2. Cache operations return `Ok(())` for writes (no-op)
3. All requests fall back to database queries
4. No performance degradation, only cache benefits are lost
5. Warnings logged for monitoring

### Cache Miss

On cache miss:
1. Data fetched from database
2. Result automatically cached with TTL
3. Subsequent requests served from cache
4. Fallback to database on cache errors

## Performance Characteristics

### Expected Cache Hit Rates

- **Anchor Metrics**: 80-90% (10 min TTL, frequent requests)
- **Corridor Metrics**: 75-85% (5 min TTL, frequent requests)
- **Dashboard Stats**: 85-95% (1 min TTL, very frequent requests)

### Database Load Reduction

With caching enabled:
- **Anchor list queries**: ~90% reduction
- **Corridor queries**: ~80% reduction
- **Dashboard queries**: ~90% reduction

### Response Time Improvement

- **Cache hit**: ~5-10ms (Redis latency)
- **Cache miss**: ~50-200ms (database query)
- **Average improvement**: 60-80% with 80%+ hit rate

## Monitoring

### Cache Statistics

Monitor cache performance via `/api/cache/stats`:

```bash
curl http://localhost:8080/api/cache/stats
```

### Key Metrics

- **Hit Rate**: Percentage of requests served from cache
- **Total Requests**: Sum of hits and misses
- **Invalidations**: Number of cache entries cleared

### Logging

Cache operations are logged at DEBUG level:

```bash
# Enable debug logging
RUST_LOG=backend=debug cargo run
```

Log messages:
- `Cache hit for key: {key}`
- `Cache miss for key: {key}`
- `Cache set for key: {key} (TTL: {ttl}s)`
- `Cache invalidated for key: {key}`
- `Cache invalidated for pattern: {pattern}`

## Best Practices

### 1. Cache Key Consistency

Always use the `keys` module for cache key generation:

```rust
use crate::cache::keys;

let key = keys::anchor_list(limit, offset);
let key = keys::corridor_detail(corridor_key);
```

### 2. TTL Selection

- **Frequently changing data**: Shorter TTL (1-5 minutes)
- **Stable data**: Longer TTL (10-30 minutes)
- **Real-time data**: No caching or very short TTL

### 3. Cache Invalidation

Always invalidate related caches when data changes:

```rust
// After updating anchor metrics
invalidation.invalidate_anchor(anchor_id).await?;

// After updating corridor metrics
invalidation.invalidate_corridor(corridor_key).await?;
```

### 4. Error Handling

Cache errors are non-fatal:

```rust
// Cache errors don't break the request
let _ = cache.set(key, &data, ttl).await;

// Always fall back to database on cache miss
let data = <()>::get_or_fetch(&cache, key, ttl, async {
    db.fetch_data().await
}).await?;
```

## Troubleshooting

### Low Cache Hit Rate

1. Check TTL values - may be too short
2. Verify Redis connection: `redis-cli ping`
3. Check cache key consistency
4. Monitor request patterns

### Redis Connection Issues

```bash
# Test Redis connection
redis-cli ping

# Check Redis memory usage
redis-cli info memory

# Monitor Redis commands
redis-cli monitor
```

### Cache Invalidation Not Working

1. Verify `CacheInvalidationService` is called
2. Check pattern matching in `delete_pattern`
3. Ensure cache keys follow naming convention
4. Check logs for invalidation errors

## Future Enhancements

1. **Distributed Caching**: Multi-instance cache coordination
2. **Cache Warming**: Pre-populate cache on startup
3. **Adaptive TTL**: Adjust TTL based on hit rate
4. **Cache Compression**: Reduce memory usage for large objects
5. **Cache Metrics Export**: Prometheus metrics integration
6. **Cache Versioning**: Handle schema changes gracefully

## Testing

### Unit Tests

```bash
cargo test cache
```

### Integration Tests

```bash
# Start Redis
redis-server

# Run tests
cargo test --test '*' -- --nocapture
```

### Load Testing

```bash
# Monitor cache hit rate under load
ab -n 10000 -c 100 http://localhost:8080/api/anchors
curl http://localhost:8080/api/cache/stats
```

## References

- [Redis Documentation](https://redis.io/documentation)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [Tokio Async Runtime](https://tokio.rs/)
