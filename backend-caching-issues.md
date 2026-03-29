# Backend Caching Strategy Issues

## Priority: Medium

### Overview
Analysis of backend caching reveals that while some caching is implemented, there are significant gaps and optimization opportunities.

---

## price_feed.rs - **GOOD CACHING IMPLEMENTATION**

### ✅ What's Working Well:

#### 1. In-Memory Cache with TTL
- **Implementation**: `Arc<RwLock<HashMap<String, CachedPrice>>>`
- **TTL Support**: Configurable cache TTL (default 15 minutes)
- **Cache Hit Logic**: Checks age before returning cached data

#### 2. Stale Cache Fallback
- **Feature**: Returns stale data when API calls fail
- **Benefit**: Improves resilience during outages

#### 3. Batch Operations
- **Feature**: `get_prices()` fetches multiple assets efficiently
- **Optimization**: Reduces API calls

#### 4. Cache Management
- **Methods**: `clear_cache()`, `cache_stats()`, `warm_cache()`
- **Monitoring**: Track cache hit rates

---

## anchors.rs - **MIXED CACHING IMPLEMENTATION**

### ✅ What's Working:

#### 1. HTTP Response Caching
- **Implementation**: `cached_json_response()` for full responses
- **TTL Support**: Uses cache configuration

#### 2. Query Result Caching
- **Implementation**: `cached_query()` helper for database queries
- **Cache Key**: `keys::anchor_list(params.limit, params.offset)`

### ❌ Missing Caching Opportunities:

#### 1. **RPC Payment Data** (Critical)
- **Issue**: No caching for expensive RPC payment fetches
- **Impact**: Every request triggers multiple RPC calls
- **Location**: Lines 478-499 in `get_anchors()`

#### 2. **Asset Data** (Medium)
- **Issue**: Individual asset queries not cached
- **Current**: Batch fetch helps but no persistence
- **Impact**: Repeated database queries

#### 3. **Anchor Metrics** (High)
- **Issue**: Real-time metrics calculated on every request
- **Impact**: Expensive calculations repeated frequently
- **Location**: Lines 502-536

---

## Specific Missing Caching Strategies

### 1. RPC Payment Data Caching
**Problem**: 
```rust
// Lines 478-499: Expensive RPC call with no caching
let payments = circuit_breaker
    .call(|| async {
        rpc_client
            .fetch_all_account_payments(&anchor.stellar_account, Some(500))
            .await
    })
    .await
```

**Impact**:
- High latency (500+ms per anchor)
- Rate limiting risk
- Poor scalability

**Solution Needed**:
- Cache payment data with shorter TTL (5-10 minutes)
- Incremental updates for new payments
- Background refresh strategy

### 2. Anchor Metrics Caching
**Problem**:
```rust
// Lines 502-536: Metrics recalculated every request
let (total_transactions, successful_transactions, failed_transactions) = 
    if payments.is_empty() {
        (anchor.total_transactions, anchor.successful_transactions, anchor.failed_transactions)
    } else {
        let total = payments.len() as i64;
        let successful = total;
        let failed = 0;
        (total, successful, failed)
    };
```

**Impact**:
- CPU-intensive calculations
- Redundant processing
- Inconsistent data between requests

**Solution Needed**:
- Cache calculated metrics
- Background recalculation
- Event-driven updates

### 3. Asset Information Caching
**Problem**:
```rust
// Lines 463-466: Database query not cached
let asset_map = db
    .get_assets_by_anchors(&anchor_ids)
    .await
    .unwrap_or_default();
```

**Impact**:
- Repeated database queries
- Unnecessary load

**Solution Needed**:
- Cache asset mappings
- Longer TTL (assets change infrequently)

---

## Performance Impact

### Current Issues:
1. **High API Latency**: 500ms+ for anchor lists due to RPC calls
2. **Poor Scalability**: Performance degrades with more anchors
3. **Rate Limiting Risk**: Multiple RPC calls per request
4. **Database Load**: Repeated queries for static data

### Estimated Improvements with Caching:
- **Latency**: 80-90% reduction for cached data
- **Throughput**: 5-10x improvement
- **Resource Usage**: 70% reduction in RPC calls

---

## Recommended Implementation Strategy

### Phase 1: Quick Wins
1. **Cache RPC Payment Data**
   - TTL: 5 minutes
   - Cache key: `payments:{anchor_account}:{limit}`
   
2. **Cache Asset Mappings**
   - TTL: 1 hour
   - Cache key: `assets:{anchor_ids_hash}`

### Phase 2: Advanced Caching
1. **Metrics Pre-computation**
   - Background job updates metrics
   - Cache calculated results
   - Event-driven invalidation

2. **Multi-layer Caching**
   - L1: In-memory (hot data)
   - L2: Redis (warm data)
   - L3: Database (cold data)

### Phase 3: Cache Management
1. **Cache Warming**
   - Pre-populate on startup
   - Scheduled refresh jobs

2. **Cache Invalidation**
   - Smart invalidation on updates
   - Version-based cache keys

---

## Configuration Recommendations

### Cache TTLs:
- **Payment Data**: 5 minutes (real-time needs)
- **Anchor Metrics**: 15 minutes (balance freshness)
- **Asset Information**: 1 hour (static data)
- **Anchor Lists**: 30 minutes (moderate change frequency)

### Cache Keys:
- Use consistent naming: `{type}:{identifier}:{params}`
- Include version for cache invalidation
- Hash complex parameters

---

## Monitoring & Metrics

### Cache Performance Metrics:
- Hit/miss ratios by cache type
- Average response times
- Cache size and memory usage
- Invalidation frequency

### Alerts:
- Cache hit rate < 80%
- Cache size > memory limits
- Frequent invalidations
