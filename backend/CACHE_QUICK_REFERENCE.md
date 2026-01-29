# Cache Quick Reference

## Endpoints

### Cached Data Endpoints

| Endpoint | Method | TTL | Cache Key Pattern |
|----------|--------|-----|-------------------|
| `/api/anchors` | GET | 10 min | `anchor:list:{limit}:{offset}` |
| `/api/corridors` | GET | 5 min | `corridor:list:{limit}:{offset}:{filters}` |
| `/api/corridors/:key` | GET | 5 min | `corridor:detail:{key}` |
| `/api/metrics/overview` | GET | 1 min | `dashboard:stats` |

### Monitoring Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/cache/stats` | GET | Get cache hit rate and statistics |
| `/api/cache/reset` | POST | Reset cache statistics counters |

## Cache Statistics Response

```json
{
  "hits": 1250,
  "misses": 250,
  "invalidations": 45,
  "hit_rate_percent": 83.33,
  "total_requests": 1500
}
```

## Environment Variables

```bash
# Redis connection URL (optional)
REDIS_URL=redis://127.0.0.1:6379

# Logging
RUST_LOG=backend=debug  # Enable debug logging for cache operations
```

## Common Commands

### Check Cache Status
```bash
curl http://localhost:8080/api/cache/stats
```

### Reset Cache Statistics
```bash
curl -X POST http://localhost:8080/api/cache/stats
```

### Test Redis Connection
```bash
redis-cli ping
# Should return: PONG
```

### View Cache Keys
```bash
redis-cli KEYS "*"
```

### Clear All Cache
```bash
redis-cli FLUSHDB
```

### Monitor Redis Commands
```bash
redis-cli monitor
```

## Code Snippets

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
        cache.config.anchor_data_ttl,
        async { db.fetch_data().await }
    ).await?;

    Ok(Json(response))
}
```

### Manual Cache Invalidation

```rust
use crate::cache_invalidation::CacheInvalidationService;

let invalidation = CacheInvalidationService::new(cache.clone());
invalidation.invalidate_anchors().await?;
```

### Adding Custom Cache Key

```rust
// In src/cache.rs, keys module
pub fn my_key(id: &str) -> String {
    format!("my:prefix:{}", id)
}
```

## TTL Values

| Data Type | TTL | Reason |
|-----------|-----|--------|
| Anchor Metrics | 10 min | Updated every 5 min, allow some staleness |
| Corridor Metrics | 5 min | Frequently changing, need freshness |
| Dashboard Stats | 1 min | Real-time data, very fresh |

## Troubleshooting

### Cache Not Working?

1. **Check Redis is running**
   ```bash
   redis-cli ping
   ```

2. **Check logs**
   ```bash
   RUST_LOG=backend=debug cargo run
   ```

3. **Verify cache keys exist**
   ```bash
   redis-cli KEYS "anchor:*"
   ```

### Low Hit Rate?

1. Check if TTL is too short
2. Check if requests use different parameters
3. Monitor invalidation frequency
4. Check request patterns

### Redis Connection Issues?

```bash
# Test connection
redis-cli -h localhost -p 6379 ping

# Check Redis status
redis-cli info server

# Restart Redis
sudo systemctl restart redis-server
```

## Performance Metrics

### Expected Hit Rates
- Anchor Metrics: 80-90%
- Corridor Metrics: 75-85%
- Dashboard Stats: 85-95%

### Response Time Improvement
- Cache hit: ~5-10ms
- Cache miss: ~50-200ms
- Average improvement: 60-80%

### Database Load Reduction
- Anchor queries: ~90%
- Corridor queries: ~80%
- Dashboard queries: ~90%

## Cache Invalidation

### Automatic (Every 5 minutes)
- After data ingestion completes
- Clears: anchors, corridors, metrics

### Manual
```rust
invalidation.invalidate_anchor(id).await?;
invalidation.invalidate_corridor(key).await?;
invalidation.invalidate_all().await?;
```

## Testing

### Run Cache Tests
```bash
cargo test cache
```

### Load Test
```bash
ab -n 10000 -c 100 http://localhost:8080/api/anchors
curl http://localhost:8080/api/cache/stats
```

### Integration Test
```bash
redis-server &
cargo test
redis-cli shutdown
```

## Files Reference

| File | Purpose |
|------|---------|
| `src/cache.rs` | Core cache manager |
| `src/cache_middleware.rs` | Cache-aware trait |
| `src/cache_invalidation.rs` | Invalidation service |
| `src/api/cache_stats.rs` | Monitoring endpoints |
| `src/api/anchors_cached.rs` | Cached anchor endpoints |
| `src/api/corridors_cached.rs` | Cached corridor endpoints |
| `src/api/metrics_cached.rs` | Cached metrics endpoints |

## Documentation

| Document | Content |
|----------|---------|
| `CACHING_IMPLEMENTATION.md` | Complete architecture & design |
| `CACHE_INTEGRATION_GUIDE.md` | Integration & deployment guide |
| `REDIS_CACHING_SUMMARY.md` | Implementation summary |
| `CACHE_QUICK_REFERENCE.md` | This quick reference |

## Key Concepts

### Cache Hit
Request served from Redis cache (~5-10ms)

### Cache Miss
Request served from database, then cached (~50-200ms)

### Cache Invalidation
Removing stale data from cache to ensure freshness

### TTL (Time To Live)
How long data stays in cache before expiring

### Graceful Degradation
System works without Redis, just slower

## Best Practices

1. ✓ Always use `keys::*` builders for cache keys
2. ✓ Handle cache errors gracefully
3. ✓ Invalidate related caches on updates
4. ✓ Monitor hit rates regularly
5. ✓ Adjust TTL based on data freshness
6. ✓ Test cache behavior under load
7. ✓ Document cache keys for new endpoints
8. ✓ Use appropriate TTL for data type

## Support Resources

- **Logs**: `RUST_LOG=backend=debug cargo run`
- **Stats**: `curl http://localhost:8080/api/cache/stats`
- **Redis CLI**: `redis-cli`
- **Documentation**: See `CACHING_IMPLEMENTATION.md`
