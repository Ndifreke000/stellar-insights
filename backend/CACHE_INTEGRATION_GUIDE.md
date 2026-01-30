# Cache Integration Guide

## Quick Start

### 1. Prerequisites

- Redis server running (or will gracefully degrade without it)
- Rust 1.70+
- Tokio async runtime

### 2. Installation

Redis is already in `Cargo.toml` dependencies. No additional setup needed.

### 3. Environment Setup

```bash
# Optional: Set Redis URL (defaults to localhost:6379)
export REDIS_URL=redis://127.0.0.1:6379

# Run the backend
cargo run
```

### 4. Verify Caching is Working

```bash
# Check cache statistics
curl http://localhost:8080/api/cache/stats

# Expected response:
# {
#   "hits": 0,
#   "misses": 0,
#   "invalidations": 0,
#   "hit_rate_percent": 0.0,
#   "total_requests": 0
# }

# Make a request to populate cache
curl http://localhost:8080/api/anchors

# Check stats again - should show cache miss
curl http://localhost:8080/api/cache/stats

# Make the same request again - should show cache hit
curl http://localhost:8080/api/anchors
curl http://localhost:8080/api/cache/stats
```

## Implementation Details

### Cache Manager Initialization

In `main.rs`, the cache is initialized at startup:

```rust
let cache = match CacheManager::new(cache_config).await {
    Ok(cache) => {
        tracing::info!("Cache manager initialized successfully");
        Arc::new(cache)
    }
    Err(e) => {
        tracing::warn!("Failed to initialize cache manager: {}", e);
        Arc::new(CacheManager::new(CacheConfig::default()).await?)
    }
};
```

### Cache Invalidation Integration

The cache invalidation service is integrated with the data ingestion pipeline:

```rust
// After metrics sync completes successfully
if let Err(e) = cache_invalidation_clone.invalidate_anchors().await {
    tracing::warn!("Failed to invalidate anchor caches: {}", e);
}
if let Err(e) = cache_invalidation_clone.invalidate_corridors().await {
    tracing::warn!("Failed to invalidate corridor caches: {}", e);
}
if let Err(e) = cache_invalidation_clone.invalidate_metrics().await {
    tracing::warn!("Failed to invalidate metrics caches: {}", e);
}
```

This ensures caches are refreshed every 5 minutes when metrics are synced.

## API Endpoints

### Cached Endpoints

#### 1. List Anchors
```
GET /api/anchors?limit=50&offset=0
Cache TTL: 10 minutes
Cache Key: anchor:list:50:0
```

#### 2. List Corridors
```
GET /api/corridors?limit=50&offset=0&time_period=30d
Cache TTL: 5 minutes
Cache Key: corridor:list:50:0:{filters}
```

#### 3. Corridor Details
```
GET /api/corridors/EURC:issuer->USDC:issuer
Cache TTL: 5 minutes
Cache Key: corridor:detail:EURC:issuer->USDC:issuer
```

#### 4. Metrics Overview
```
GET /api/metrics/overview
Cache TTL: 1 minute
Cache Key: dashboard:stats
```

### Monitoring Endpoints

#### Get Cache Statistics
```
GET /api/cache/stats
Returns: {hits, misses, invalidations, hit_rate_percent, total_requests}
```

#### Reset Cache Statistics
```
POST /api/cache/reset
Returns: {status: "success", message: "Cache statistics reset"}
```

## Code Examples

### Using Cache in a New Endpoint

```rust
use crate::cache::{keys, CacheManager};
use crate::cache_middleware::CacheAware;

pub async fn my_endpoint(
    State((db, cache)): State<(Arc<Database>, Arc<CacheManager>)>,
) -> ApiResult<Json<MyResponse>> {
    let cache_key = keys::my_custom_key();
    
    let response = <()>::get_or_fetch(
        &cache,
        &cache_key,
        cache.config.anchor_data_ttl,  // or other TTL
        async {
            // Fetch from database
            let data = db.fetch_data().await?;
            Ok(data)
        },
    )
    .await?;

    Ok(Json(response))
}
```

### Manual Cache Invalidation

```rust
use crate::cache_invalidation::CacheInvalidationService;

let invalidation = CacheInvalidationService::new(cache.clone());

// Invalidate specific anchor
invalidation.invalidate_anchor("anchor-id").await?;

// Invalidate all anchors
invalidation.invalidate_anchors().await?;

// Invalidate specific corridor
invalidation.invalidate_corridor("corridor-key").await?;

// Full invalidation
invalidation.invalidate_all().await?;
```

### Adding Custom Cache Keys

In `src/cache.rs`, add to the `keys` module:

```rust
pub mod keys {
    // ... existing keys ...
    
    pub fn my_custom_key(id: &str) -> String {
        format!("my:custom:{}", id)
    }
    
    pub fn my_custom_pattern() -> String {
        "my:custom:*".to_string()
    }
}
```

## Performance Tuning

### Adjusting TTL Values

Edit `CacheConfig::default()` in `src/cache.rs`:

```rust
impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            corridor_metrics_ttl: 300,   // 5 minutes
            anchor_data_ttl: 600,        // 10 minutes
            dashboard_stats_ttl: 60,     // 1 minute
        }
    }
}
```

### Monitoring Cache Performance

```bash
# Watch cache statistics in real-time
watch -n 1 'curl -s http://localhost:8080/api/cache/stats | jq'

# Or with a script
while true; do
  echo "=== Cache Stats ==="
  curl -s http://localhost:8080/api/cache/stats | jq
  sleep 5
done
```

### Redis Memory Management

```bash
# Check Redis memory usage
redis-cli info memory

# Set max memory policy
redis-cli CONFIG SET maxmemory-policy allkeys-lru

# Monitor Redis commands
redis-cli monitor
```

## Troubleshooting

### Cache Not Working

1. **Check Redis Connection**
   ```bash
   redis-cli ping
   # Should return: PONG
   ```

2. **Check Logs**
   ```bash
   RUST_LOG=backend=debug cargo run
   # Look for "Connected to Redis for caching"
   ```

3. **Verify Cache Keys**
   ```bash
   redis-cli KEYS "*"
   # Should show cache keys like "anchor:list:50:0"
   ```

### Low Hit Rate

1. **Check TTL Values**
   - Are they too short?
   - Are requests coming faster than TTL?

2. **Check Request Patterns**
   - Are requests using different parameters?
   - Each unique parameter combination creates a new cache key

3. **Monitor Invalidations**
   - Are caches being invalidated too frequently?
   - Check `invalidations` count in `/api/cache/stats`

### Redis Connection Issues

```bash
# Test connection
redis-cli -h localhost -p 6379 ping

# Check Redis logs
tail -f /var/log/redis/redis-server.log

# Restart Redis
sudo systemctl restart redis-server
```

## Testing

### Unit Tests

```bash
# Run cache tests
cargo test cache

# Run with output
cargo test cache -- --nocapture
```

### Integration Tests

```bash
# Start Redis
redis-server &

# Run all tests
cargo test

# Stop Redis
redis-cli shutdown
```

### Load Testing

```bash
# Install Apache Bench
sudo apt-get install apache2-utils

# Run load test
ab -n 10000 -c 100 http://localhost:8080/api/anchors

# Check cache stats
curl http://localhost:8080/api/cache/stats
```

## Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y redis-server
COPY --from=builder /app/target/release/backend /usr/local/bin/
EXPOSE 8080 6379
CMD ["sh", "-c", "redis-server --daemonize yes && backend"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  backend:
    build: .
    ports:
      - "8080:8080"
    environment:
      - REDIS_URL=redis://redis:6379
      - DATABASE_URL=sqlite:stellar_insights.db
    depends_on:
      - redis
  
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
```

### Environment Variables

```bash
# Production settings
REDIS_URL=redis://redis-host:6379
DATABASE_URL=sqlite:stellar_insights.db
RUST_LOG=backend=info
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
```

## Monitoring & Observability

### Prometheus Metrics (Future)

```rust
// Planned: Export cache metrics to Prometheus
// - cache_hits_total
// - cache_misses_total
// - cache_invalidations_total
// - cache_hit_rate
```

### Logging

Cache operations are logged at DEBUG level:

```bash
# Enable debug logging
RUST_LOG=backend=debug cargo run

# Filter to cache operations only
RUST_LOG=backend::cache=debug cargo run
```

### Health Checks

```bash
# Check if cache is working
curl http://localhost:8080/api/cache/stats

# Check if Redis is accessible
redis-cli ping
```

## Best Practices

1. **Always use cache key builders** from `cache::keys` module
2. **Handle cache errors gracefully** - cache is optional
3. **Invalidate related caches** when data changes
4. **Monitor hit rates** regularly
5. **Adjust TTL based on data freshness requirements**
6. **Test cache behavior** under load
7. **Document cache keys** for new endpoints
8. **Use appropriate TTL values** for different data types

## Support

For issues or questions:
1. Check logs: `RUST_LOG=backend=debug cargo run`
2. Verify Redis: `redis-cli ping`
3. Check cache stats: `curl http://localhost:8080/api/cache/stats`
4. Review documentation: `CACHING_IMPLEMENTATION.md`
