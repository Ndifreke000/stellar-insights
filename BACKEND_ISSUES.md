# Backend Issues - Stellar Insights

This document contains 30 detailed, actionable issues for the Stellar Insights backend. Each issue includes clear descriptions, affected files, acceptance criteria, and implementation guidance.

---

## 🔴 Critical Priority Issues

### Issue #1: Horizon API Parser Fails with New Soroban-Compatible Response Format

**Priority:** Critical  
**Type:** Bug  
**Labels:** `bug`, `critical`, `rpc`, `stellar`

**Description:**
The current `Payment` struct in the RPC client cannot parse the new Horizon API response format that includes Soroban-compatible fields. The new format uses `asset_balance_changes` instead of individual `destination`, `amount`, `asset_code` fields, causing all real RPC calls to fail with "Failed to parse payments response" errors.

**Current Behavior:**
- All RPC payment fetching fails when `RPC_MOCK_MODE=false`
- Error: "Failed to parse payments response"
- System falls back to empty data arrays
- Frontend displays no corridor or anchor data

**Expected Behavior:**
- Successfully parse Horizon API responses with new format
- Extract payment information from `asset_balance_changes` array
- Maintain backward compatibility if possible

**Affected Files:**
- `backend/src/rpc/stellar.rs` (lines 60-75, Payment struct)
- `backend/src/api/anchors_cached.rs` (uses fetch_account_payments)
- `backend/src/api/corridors_cached.rs` (uses fetch_payments)

**Technical Details:**
New Horizon API format includes:
```json
{
  "asset_balance_changes": [
    {
      "asset_type": "credit_alphanum4",
      "asset_code": "USDC",
      "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      "type": "transfer",
      "from": "GXXXXXXX...",
      "to": "GDYYYYYY...",
      "amount": "100.0000000"
    }
  ]
}
```

**Acceptance Criteria:**
- [ ] Update `Payment` struct to include `asset_balance_changes` field
- [ ] Create parser to extract payment info from balance changes
- [ ] Add unit tests for new format parsing
- [ ] Test with real Horizon API (testnet)
- [ ] Update mock data to match new format
- [ ] Document the new structure in code comments

**Implementation Steps:**
1. Add new struct `AssetBalanceChange` with required fields
2. Add `asset_balance_changes: Option<Vec<AssetBalanceChange>>` to `Payment`
3. Create helper method `Payment::get_destination()` that checks both old and new format
4. Create helper method `Payment::get_amount()` with same logic
5. Update all call sites to use helper methods
6. Add integration test with real Horizon API call

**References:**
- Horizon API Documentation: https://developers.stellar.org/docs/data/horizon
- Related: Issue documented in `docs/RPC_INTEGRATION_SUMMARY.md`

---

### Issue #2: Corridor Detail Endpoint Returns 404 - Stub Implementation

**Priority:** Critical  
**Type:** Bug  
**Labels:** `bug`, `critical`, `api`, `corridors`

**Description:**
The `get_corridor_detail()` function in the corridors API is currently a stub that always returns a 404 NotFound error. This breaks the corridor detail page in the frontend, preventing users from viewing detailed metrics for specific payment corridors.

**Current Behavior:**
```rust
pub async fn get_corridor_detail(...) -> ApiResult<Json<CorridorDetailResponse>> {
    Err(crate::handlers::ApiError::NotFound(
        "Corridor detail endpoint not yet implemented with RPC".to_string()
    ))
}
```

**Expected Behavior:**
- Fetch detailed payment and trade data for specific corridor from RPC
- Calculate historical success rates over time periods
- Generate latency distribution buckets
- Compute liquidity trends
- Return comprehensive corridor metrics

**Affected Files:**
- `backend/src/api/corridors_cached.rs` (lines 180-186)
- Frontend: `frontend/src/app/corridors/[id]/page.tsx`

**Technical Requirements:**

1. Parse corridor_key parameter (format: "ASSET1:ISSUER1->ASSET2:ISSUER2")
2. Fetch payments for the specific asset pair from RPC
3. Calculate metrics:
   - Historical success rate (30-day buckets)
   - Latency distribution (100ms, 250ms, 500ms, 1s, 2s+ buckets)
   - Liquidity trends (daily snapshots)
   - Volume history (daily aggregates)
   - Slippage data (if available from order books)
4. Find related corridors (same source or destination asset)
5. Cache results with appropriate TTL

**Acceptance Criteria:**
- [ ] Implement full `get_corridor_detail()` function
- [ ] Fetch RPC data for specific asset pair
- [ ] Calculate all required metrics
- [ ] Return properly formatted `CorridorDetailResponse`
- [ ] Add caching with 5-minute TTL
- [ ] Handle invalid corridor IDs gracefully
- [ ] Add integration tests
- [ ] Update API documentation

**Implementation Steps:**
1. Parse and validate corridor_key parameter
2. Extract source and destination asset info
3. Call `rpc_client.fetch_payments()` with appropriate filters
4. Implement `calculate_historical_success_rate()` helper
5. Implement `calculate_latency_distribution()` helper
6. Implement `calculate_liquidity_trends()` helper
7. Query related corridors from cache or RPC
8. Assemble and return response
9. Add error handling for each step

**Data Structures Needed:**
```rust
struct CorridorDetailResponse {
    corridor: CorridorResponse,
    historical_success_rate: Vec<SuccessRateDataPoint>,
    latency_distribution: Vec<LatencyDataPoint>,
    liquidity_trends: Vec<LiquidityDataPoint>,
    historical_volume: Vec<VolumeDataPoint>,
    historical_slippage: Vec<SlippageDataPoint>,
    related_corridors: Option<Vec<CorridorResponse>>,
}
```

---

### Issue #3: Missing Price Feed Integration for USD Conversions

**Priority:** Critical  
**Type:** Feature  
**Labels:** `enhancement`, `critical`, `external-api`, `pricing`

**Description:**
The backend currently has no integration with price feed APIs (CoinGecko, CoinMarketCap, etc.) to convert asset values to USD. All USD values are either hardcoded, estimated, or missing. This is documented as a critical need in `docs/EXTERNAL_DATA_SOURCES.md`.

**Current Behavior:**
- USD values are calculated incorrectly or missing
- No real-time price data for assets
- Liquidity and volume metrics are inaccurate
- Cannot compare corridors in common currency

**Expected Behavior:**
- Integrate with CoinGecko or CoinMarketCap API
- Fetch real-time prices for all Stellar assets
- Cache prices with appropriate TTL (5-15 minutes)
- Convert all asset amounts to USD for display
- Handle rate limiting and API failures gracefully

**Affected Files:**
- `backend/src/api/corridors_cached.rs` (all USD calculations)
- `backend/src/api/anchors_cached.rs` (volume calculations)
- New file needed: `backend/src/services/price_feed.rs`

**Technical Requirements:**

1. Create `PriceFeedClient` service
2. Support multiple providers (CoinGecko primary, CoinMarketCap fallback)
3. Implement asset ID mapping (Stellar asset codes to API IDs)
4. Add Redis/in-memory caching for prices
5. Handle API rate limits (CoinGecko: 10-50 calls/min)
6. Implement fallback strategies when API unavailable

**API Options:**
- **CoinGecko** (Recommended): Free tier, 10-50 calls/min
  - Endpoint: `/simple/price`
  - Supports multiple currencies
  - Good Stellar asset coverage
  
- **CoinMarketCap**: Requires API key, 333 calls/day free
  - More comprehensive data
  - Better for institutional use

**Acceptance Criteria:**
- [ ] Create `PriceFeedClient` struct with trait-based design
- [ ] Implement CoinGecko provider
- [ ] Add price caching (15-minute TTL)
- [ ] Create asset mapping configuration
- [ ] Handle API failures with stale data fallback
- [ ] Add rate limiting protection
- [ ] Update all USD calculation functions
- [ ] Add unit and integration tests
- [ ] Document API key setup in README

**Implementation Steps:**
1. Add dependencies: `reqwest`, `serde_json`
2. Create `backend/src/services/price_feed.rs`
3. Define `PriceFeedProvider` trait
4. Implement `CoinGeckoProvider`
5. Create asset mapping config file
6. Add caching layer with TTL
7. Integrate into main app state
8. Update corridor and anchor endpoints
9. Add monitoring/logging for API calls
10. Create fallback mechanism

**Configuration Example:**
```toml
# backend/.env
PRICE_FEED_PROVIDER=coingecko
PRICE_FEED_API_KEY=optional
PRICE_FEED_CACHE_TTL_SECONDS=900
```

**Asset Mapping Example:**
```json
{
  "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN": "usd-coin",
  "XLM:native": "stellar",
  "EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2": "euro-coin"
}
```

**References:**
- CoinGecko API: https://www.coingecko.com/en/api/documentation
- Documented in: `docs/EXTERNAL_DATA_SOURCES.md`

---

### Issue #4: Incomplete Corridor Asset Pair Detection Logic

**Priority:** High  
**Type:** Bug  
**Labels:** `bug`, `high`, `corridors`, `rpc`

**Description:**
The current corridor detection logic in `list_corridors()` assumes the destination asset is always XLM (native). This is a known limitation documented in the code with a TODO comment. This causes incorrect corridor identification and missing data for non-XLM destination corridors.

**Current Code:**
```rust
// Line 76-79 in backend/src/api/corridors_cached.rs
// For now, assume destination is XLM (we'd need more data to determine actual destination asset)
let asset_to = "XLM:native".to_string();
```

**Current Behavior:**
- All corridors show XLM as destination
- Missing actual cross-asset corridors (USDC->EUR, etc.)
- Inaccurate corridor metrics
- Cannot track real payment paths

**Expected Behavior:**
- Detect actual destination asset from payment data
- Support all asset pair combinations
- Accurately identify corridor paths
- Handle path payments with intermediate assets

**Affected Files:**
- `backend/src/api/corridors_cached.rs` (lines 60-110)
- `backend/src/rpc/stellar.rs` (Payment struct)

**Root Cause:**
The Horizon API payment response doesn't always include explicit destination asset information. Need to:
1. Parse `asset_balance_changes` to find receiving account's asset
2. Handle path payments that go through multiple assets
3. Track both direct and indirect corridors

**Acceptance Criteria:**
- [ ] Parse destination asset from `asset_balance_changes`
- [ ] Detect all unique asset pairs from payment data
- [ ] Handle path payments correctly
- [ ] Support native XLM and issued assets
- [ ] Create corridors for all detected pairs
- [ ] Add tests with various asset combinations
- [ ] Update documentation

**Implementation Steps:**
1. Update `Payment` struct to include destination asset info
2. Create helper function `extract_asset_pairs()` from payment
3. Handle multiple balance changes in single payment
4. Identify source and destination from balance change types
5. Update corridor grouping logic
6. Add validation for asset pair format
7. Test with real multi-asset payments

**Technical Approach:**
```rust
fn extract_corridor_from_payment(payment: &Payment) -> Option<(Asset, Asset)> {
    // Parse asset_balance_changes
    // Find 'transfer' type changes
    // Identify source (negative amount) and destination (positive amount)
    // Return (source_asset, dest_asset) tuple
}
```

**Test Cases Needed:**
- Direct XLM payment
- Direct issued asset payment (USDC->USDC)
- Cross-asset payment (USDC->EUR)
- Path payment through multiple assets
- Failed payment handling

---

### Issue #5: Database Credentials Exposed in Version Control

**Priority:** Critical  
**Type:** Security  
**Labels:** `security`, `critical`, `configuration`

**Description:**
The `backend/.env` file containing database connection strings and potentially sensitive configuration is committed to version control. This is a security risk and violates best practices for secrets management.

**Current Behavior:**
- `.env` file is tracked in git
- Database URL visible in repository
- No secrets management strategy
- Risk of credential exposure

**Expected Behavior:**
- `.env` file in `.gitignore`
- `.env.example` template without secrets
- Documentation for local setup
- Use environment variables in production
- Consider secrets management tools

**Affected Files:**
- `backend/.env` (should be removed from git)
- `backend/.gitignore` (should include .env)
- `.env.example` (should be created)
- `backend/README.md` (setup documentation)

**Security Risks:**
1. Database credentials exposed
2. API keys potentially visible
3. Internal URLs/endpoints revealed
4. Configuration details leaked

**Acceptance Criteria:**
- [ ] Remove `backend/.env` from git history
- [ ] Add `.env` to `.gitignore`
- [ ] Create `backend/.env.example` template
- [ ] Update README with setup instructions
- [ ] Document environment variables
- [ ] Add validation for required env vars on startup
- [ ] Consider using `dotenv` crate properly

**Implementation Steps:**
1. Create `.env.example` with placeholder values
2. Add `.env` to `.gitignore`
3. Remove `.env` from git history:
   ```bash
   git rm --cached backend/.env
   git commit -m "Remove .env from version control"
   ```
4. Update README with environment setup section
5. Add startup validation for required variables
6. Document all environment variables

**Environment Variables to Document:**
```bash
# Database
DATABASE_URL=sqlite:./stellar_insights.db

# Server
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
RUST_LOG=info

# Redis
REDIS_URL=redis://127.0.0.1:6379

# Stellar RPC
RPC_MOCK_MODE=false
STELLAR_RPC_URL=https://stellar.api.onfinality.io/public
STELLAR_HORIZON_URL=https://horizon.stellar.org

# Price Feed (optional)
PRICE_FEED_API_KEY=your_api_key_here
```

---

## 🟡 High Priority Issues

### Issue #6: No Pagination Support for RPC Data Fetching

**Priority:** High  
**Type:** Enhancement  
**Labels:** `enhancement`, `high`, `rpc`, `performance`

**Description:**
The RPC client currently fetches fixed limits (200 payments, 200 trades) without proper pagination support. This limits data completeness and prevents fetching historical data beyond the initial batch.

**Current Behavior:**
```rust
let payments = rpc_client.fetch_payments(200, None).await?;
let trades = rpc_client.fetch_trades(200, None).await?;
```
- Only fetches first 200 records
- No cursor-based pagination
- Cannot retrieve complete historical data
- Misses older transactions

**Expected Behavior:**
- Implement cursor-based pagination
- Fetch all available data or up to configurable limit
- Support time-range queries
- Handle pagination automatically in background

**Affected Files:**
- `backend/src/rpc/stellar.rs` (fetch methods)
- `backend/src/api/corridors_cached.rs` (uses fetch_payments)
- `backend/src/api/anchors_cached.rs` (uses fetch_account_payments)

**Acceptance Criteria:**
- [ ] Add pagination support to all fetch methods
- [ ] Implement cursor tracking
- [ ] Add configurable max records limit
- [ ] Support time-range filtering
- [ ] Add background pagination for large datasets
- [ ] Handle rate limiting during pagination
- [ ] Add tests for pagination logic
- [ ] Document pagination behavior

**Implementation Steps:**
1. Update fetch methods to accept `max_records` parameter
2. Implement cursor extraction from responses
3. Create pagination loop with rate limiting
4. Add progress logging for large fetches
5. Implement time-range filters
6. Add configuration for pagination limits
7. Test with large datasets

**Configuration:**
```toml
[rpc]
max_payments_per_request = 200
max_total_payments = 10000
pagination_delay_ms = 100
```

---

### Issue #7: Insufficient Error Handling and Retry Logic for RPC Failures

**Priority:** High  
**Type:** Enhancement  
**Labels:** `enhancement`, `high`, `rpc`, `reliability`

**Description:**
The current RPC error handling silently falls back to empty arrays when API calls fail. There's basic exponential backoff but no sophisticated retry logic, circuit breakers, or error categorization.

**Current Behavior:**
```rust
let payments = match rpc_client.fetch_payments(200, None).await {
    Ok(p) => p,
    Err(e) => {
        tracing::error!("Failed to fetch payments from RPC: {}", e);
        return Ok(vec![]);  // Silent failure
    }
};
```

**Problems:**
- Silent failures hide issues
- No distinction between transient and permanent errors
- No circuit breaker for failing endpoints
- No alerting on repeated failures
- Users see empty data without explanation

**Expected Behavior:**
- Categorize errors (network, rate limit, server error, etc.)
- Implement smart retry with backoff
- Add circuit breaker pattern
- Return meaningful errors to frontend
- Log structured error data
- Alert on repeated failures

**Affected Files:**
- `backend/src/rpc/stellar.rs` (all fetch methods)
- `backend/src/api/corridors_cached.rs`
- `backend/src/api/anchors_cached.rs`
- New file: `backend/src/rpc/error.rs`

**Acceptance Criteria:**
- [ ] Create custom error types for RPC failures
- [ ] Implement circuit breaker pattern
- [ ] Add error categorization
- [ ] Implement smart retry logic
- [ ] Add structured error logging
- [ ] Return errors to frontend when appropriate
- [ ] Add metrics for error rates
- [ ] Document error handling strategy

**Implementation Steps:**
1. Create `RpcError` enum with variants:
   - `NetworkError`
   - `RateLimitError`
   - `ServerError`
   - `ParseError`
   - `TimeoutError`
2. Implement circuit breaker with states (Closed, Open, HalfOpen)
3. Add retry logic with exponential backoff
4. Create error middleware
5. Add structured logging
6. Implement error metrics
7. Update all call sites

**Circuit Breaker Configuration:**
```rust
struct CircuitBreakerConfig {
    failure_threshold: u32,      // 5 failures
    success_threshold: u32,      // 2 successes to close
    timeout_duration: Duration,  // 30 seconds
    half_open_max_calls: u32,   // 3 test calls
}
```

---

### Issue #8: Missing Observability - Metrics, Tracing, and Structured Logging

**Priority:** High  
**Type:** Enhancement  
**Labels:** `enhancement`, `high`, `observability`, `monitoring`

**Description:**
The backend lacks comprehensive observability infrastructure. There's basic logging but no metrics collection, distributed tracing, or structured logging for production debugging.

**Current State:**
- Basic `tracing` crate usage
- No metrics (Prometheus, StatsD)
- No distributed tracing (Jaeger, Zipkin)
- Inconsistent log formatting
- No request ID tracking
- No performance monitoring

**Expected Behavior:**
- Prometheus metrics endpoint
- Distributed tracing with OpenTelemetry
- Structured JSON logging
- Request ID propagation
- Performance metrics
- Business metrics (corridors tracked, RPC calls, etc.)

**Metrics to Track:**
- HTTP request duration/count by endpoint
- RPC call duration/count by method
- Cache hit/miss rates
- Error rates by type
- Active connections
- Database query duration
- Background job metrics

**Affected Files:**
- `backend/src/main.rs` (add metrics middleware)
- New file: `backend/src/observability/metrics.rs`
- New file: `backend/src/observability/tracing.rs`
- All API handlers (add instrumentation)

**Acceptance Criteria:**
- [ ] Add Prometheus metrics endpoint at `/metrics`
- [ ] Implement OpenTelemetry tracing
- [ ] Add structured JSON logging
- [ ] Create request ID middleware
- [ ] Add metrics for all key operations
- [ ] Add tracing spans to all handlers
- [ ] Create Grafana dashboard examples
- [ ] Document observability setup

**Implementation Steps:**
1. Add dependencies:
   ```toml
   prometheus = "0.13"
   opentelemetry = "0.20"
   tracing-opentelemetry = "0.21"
   tracing-subscriber = { version = "0.3", features = ["json"] }
   ```
2. Create metrics registry
3. Add metrics middleware to Axum
4. Implement custom metrics
5. Setup OpenTelemetry exporter
6. Add tracing spans
7. Configure structured logging
8. Create example dashboards

**Metrics Example:**
```rust
lazy_static! {
    static ref HTTP_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["method", "endpoint", "status"]
    ).unwrap();
    
    static ref RPC_CALLS: HistogramVec = register_histogram_vec!(
        "rpc_call_duration_seconds",
        "RPC call duration",
        &["method", "status"]
    ).unwrap();
}
```

---

### Issue #9: No Rate Limiting Protection for Stellar RPC/Horizon API

**Priority:** High  
**Type:** Enhancement  
**Labels:** `enhancement`, `high`, `rpc`, `rate-limiting`

**Description:**
The backend has no rate limiting protection when calling Stellar Horizon API. This can lead to:
- API rate limit violations (429 errors)
- Service degradation
- Potential IP bans
- Unpredictable behavior under load

**Horizon API Limits:**
- Public Horizon: ~100 requests per minute
- OnFinality: Varies by plan
- Rate limit headers: `X-Ratelimit-Limit`, `X-Ratelimit-Remaining`

**Current Behavior:**
- No rate limit tracking
- No request throttling
- No respect for rate limit headers
- Parallel requests can exceed limits

**Expected Behavior:**
- Track rate limit headers from responses
- Implement token bucket algorithm
- Queue requests when approaching limits
- Respect Retry-After headers
- Provide backpressure to callers

**Affected Files:**
- `backend/src/rpc/stellar.rs` (all HTTP calls)
- New file: `backend/src/rpc/rate_limiter.rs`

**Acceptance Criteria:**
- [ ] Implement token bucket rate limiter
- [ ] Parse and respect rate limit headers
- [ ] Add request queuing
- [ ] Handle 429 responses gracefully
- [ ] Add rate limit metrics
- [ ] Make limits configurable
- [ ] Add tests for rate limiting
- [ ] Document rate limit behavior

**Implementation Steps:**
1. Create `RateLimiter` struct with token bucket
2. Add rate limit header parsing
3. Implement request queue
4. Add backoff on 429 responses
5. Integrate into RPC client
6. Add configuration
7. Add monitoring metrics
8. Test under load

**Configuration:**
```toml
[rpc.rate_limit]
requests_per_minute = 90  # Leave buffer
burst_size = 10
queue_size = 100
```

**Rate Limiter Implementation:**
```rust
pub struct RateLimiter {
    tokens: Arc<Mutex<f64>>,
    capacity: f64,
    refill_rate: f64,  // tokens per second
    last_refill: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    pub async fn acquire(&self) -> Result<()> {
        // Token bucket algorithm
    }
}
```

---

### Issue #10: Remove Dead Code and Fix Compiler Warnings

**Priority:** High  
**Type:** Refactor  
**Labels:** `refactor`, `high`, `code-quality`

**Description:**
The codebase has several compiler warnings for unused variables, dead code, and unused struct fields. This reduces code quality and can hide real issues.

**Current Warnings:**
```
warning: unused variable: `submission`
 --> src/services/snapshot.rs:309:9
  |
309 |         submission: &SubmissionResult,
  |         ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_submission`

warning: fields `transaction_hash`, `source_account`, `destination_account`, and `asset_type` are never read
 --> src/db/aggregation.rs:329:5
  |
327 | struct PaymentRecordRow {
  |        ---------------- fields in this struct
328 |     id: String,
329 |     transaction_hash: String,
  |     ^^^^^^^^^^^^^^^^
```

**Affected Files:**
- `backend/src/services/snapshot.rs:309`
- `backend/src/db/aggregation.rs:327-332`
- Potentially others throughout codebase

**Acceptance Criteria:**
- [ ] Fix all compiler warnings
- [ ] Remove truly unused code
- [ ] Prefix intentionally unused params with `_`
- [ ] Remove unused struct fields or mark with `#[allow(dead_code)]`
- [ ] Run `cargo clippy` and fix all warnings
- [ ] Enable `#![deny(warnings)]` in CI
- [ ] Document why code is kept if marked as allowed

**Implementation Steps:**
1. Run `cargo build` and collect all warnings
2. For each warning, determine if code is:
   - Truly unused → Remove it
   - Temporarily unused → Prefix with `_` or add TODO
   - Intentionally unused → Add `#[allow(dead_code)]` with comment
3. Run `cargo clippy` for additional suggestions
4. Fix all clippy warnings
5. Update CI to fail on warnings
6. Document any allowed warnings

**Clippy Configuration:**
```toml
# Cargo.toml
[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

---

### Issue #11: Add Health Check and Readiness Endpoints

**Priority:** High  
**Type:** Enhancement  
**Labels:** `enhancement`, `high`, `api`, `devops`

**Description:**
The backend lacks health check and readiness endpoints, making it difficult to:
- Monitor service health in production
- Implement proper Kubernetes liveness/readiness probes
- Verify dependencies (database, Redis, RPC) are accessible
- Automate deployment health checks

**Current Behavior:**
- No `/health` endpoint
- No `/ready` endpoint
- Cannot verify service status programmatically
- Deployment tools can't check service health

**Expected Behavior:**
- `/health` - Basic liveness check (is process running?)
- `/ready` - Readiness check (are dependencies available?)
- Return structured health status
- Check database connectivity
- Check Redis connectivity
- Check RPC endpoint accessibility
- Return appropriate HTTP status codes

**Affected Files:**
- `backend/src/main.rs` (add routes)
- New file: `backend/src/api/health.rs`

**Acceptance Criteria:**
- [ ] Implement `/health` endpoint (liveness)
- [ ] Implement `/ready` endpoint (readiness)
- [ ] Check database connection
- [ ] Check Redis connection
- [ ] Check RPC endpoint
- [ ] Return structured JSON response
- [ ] Use appropriate HTTP status codes
- [ ] Add tests for health checks
- [ ] Document endpoints in API docs

**Implementation Steps:**
1. Create `backend/src/api/health.rs`
2. Implement `health_check()` handler
3. Implement `readiness_check()` handler
4. Add dependency checks
5. Add routes to main.rs
6. Add integration tests
7. Document in README

**Response Format:**
```json
{
  "status": "healthy",
  "timestamp": "2026-02-02T15:00:00Z",
  "version": "0.1.0",
  "checks": {
    "database": {
      "status": "up",
      "latency_ms": 2
    },
    "redis": {
      "status": "up",
      "latency_ms": 1
    },
    "rpc": {
      "status": "up",
      "latency_ms": 150
    }
  }
}
```

**HTTP Status Codes:**
- 200: All checks passed
- 503: One or more checks failed
- 500: Health check itself failed

---

### Issue #12: Implement Proper Cache Invalidation Strategy

**Priority:** High  
**Type:** Enhancement  
**Labels:** `enhancement`, `high`, `caching`, `performance`

**Description:**
The current caching implementation uses only TTL-based expiration. There's no event-driven invalidation, cache warming, or smart invalidation strategies. This leads to stale data and cache misses.

**Current Behavior:**
- TTL-based expiration only
- No cache warming on startup
- No invalidation on data updates
- No cache preloading
- Cold start performance issues

**Expected Behavior:**
- Event-driven cache invalidation
- Cache warming on startup
- Selective invalidation by key patterns
- Cache preloading for common queries
- Metrics for cache performance

**Affected Files:**
- `backend/src/cache.rs` (cache manager)
- `backend/src/api/corridors_cached.rs`
- `backend/src/api/anchors_cached.rs`
- New file: `backend/src/cache/invalidation.rs`

**Cache Invalidation Triggers:**
1. New payment detected → Invalidate affected corridor
2. Anchor status change → Invalidate anchor cache
3. Manual admin trigger → Invalidate specific keys
4. Time-based → Invalidate old entries
5. Memory pressure → LRU eviction

**Acceptance Criteria:**
- [ ] Implement event-driven invalidation
- [ ] Add cache warming on startup
- [ ] Support pattern-based invalidation
- [ ] Add cache preloading
- [ ] Implement LRU eviction
- [ ] Add cache metrics
- [ ] Add admin invalidation endpoint
- [ ] Document cache strategy

**Implementation Steps:**
1. Create cache invalidation event system
2. Add cache warming on startup
3. Implement pattern matching for keys
4. Add preloading for top corridors
5. Implement LRU eviction
6. Add metrics tracking
7. Create admin endpoint
8. Add tests

**Cache Warming Example:**
```rust
async fn warm_cache(db: &Database, cache: &CacheManager, rpc: &StellarRpcClient) {
    // Preload top 10 corridors
    let top_corridors = db.get_top_corridors(10).await?;
    for corridor in top_corridors {
        let data = fetch_corridor_data(rpc, &corridor).await?;
        cache.set(&corridor.id, data, TTL).await?;
    }
}
```

---

## 🟢 Medium Priority Issues

### Issue #13: Add OpenAPI/Swagger Documentation

**Priority:** Medium  
**Type:** Documentation  
**Labels:** `documentation`, `medium`, `api`

**Description:**
The API lacks formal documentation. Frontend developers and external integrators have no clear specification of endpoints, request/response formats, or error codes.

**Current State:**
- No API documentation
- No OpenAPI/Swagger spec
- Endpoints documented only in code
- No interactive API explorer

**Expected Behavior:**
- OpenAPI 3.0 specification
- Swagger UI at `/api/docs`
- Auto-generated from code annotations
- Example requests/responses
- Error code documentation

**Affected Files:**
- All files in `backend/src/api/`
- New file: `backend/openapi.yaml` (or generated)
- `backend/src/main.rs` (add Swagger UI route)

**Acceptance Criteria:**
- [ ] Generate OpenAPI 3.0 spec
- [ ] Add Swagger UI endpoint
- [ ] Document all endpoints
- [ ] Include request/response examples
- [ ] Document error codes
- [ ] Add authentication docs (if applicable)
- [ ] Keep spec in sync with code
- [ ] Add to CI validation

**Implementation Options:**
1. **utoipa** crate (recommended for Axum)
2. Manual OpenAPI YAML
3. **aide** crate

**Implementation Steps:**
1. Add `utoipa` and `utoipa-swagger-ui` dependencies
2. Add `#[utoipa::path]` annotations to handlers
3. Create OpenAPI struct with all paths
4. Add Swagger UI route
5. Document all models
6. Add examples
7. Test documentation
8. Add CI check for spec validity

**Example:**
```rust
#[utoipa::path(
    get,
    path = "/api/corridors",
    responses(
        (status = 200, description = "List of corridors", body = Vec<CorridorResponse>),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of results"),
        ("offset" = Option<i64>, Query, description = "Pagination offset")
    )
)]
async fn list_corridors(...) -> ApiResult<Json<Vec<CorridorResponse>>> {
    // ...
}
```

---

### Issue #14: Add Request Validation Middleware

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `security`, `validation`

**Description:**
The API lacks comprehensive input validation. Query parameters, path parameters, and request bodies are not validated before processing, leading to potential security issues and poor error messages.

**Current Behavior:**
- Minimal validation
- Type coercion errors not handled gracefully
- No sanitization of inputs
- Potential for injection attacks
- Poor error messages for invalid input

**Expected Behavior:**
- Validate all inputs
- Sanitize user-provided data
- Return clear validation errors
- Prevent injection attacks
- Use validation middleware

**Affected Files:**
- All API handlers in `backend/src/api/`
- New file: `backend/src/middleware/validation.rs`

**Validation Needed:**
1. Query parameters (limit, offset, filters)
2. Path parameters (IDs, keys)
3. Request bodies (if any POST/PUT endpoints)
4. Header validation
5. Content-Type validation

**Acceptance Criteria:**
- [ ] Add validation middleware
- [ ] Validate all query parameters
- [ ] Validate all path parameters
- [ ] Sanitize string inputs
- [ ] Return structured validation errors
- [ ] Add validation tests
- [ ] Document validation rules
- [ ] Add rate limiting per IP

**Implementation Steps:**
1. Add `validator` crate dependency
2. Create validation middleware
3. Add validation to all handlers
4. Create custom validation rules
5. Implement error responses
6. Add tests
7. Document validation

**Example:**
```rust
#[derive(Debug, Deserialize, Validate)]
pub struct ListCorridorsQuery {
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<i64>,
    
    #[validate(range(min = 0))]
    pub offset: Option<i64>,
    
    #[validate(length(min = 1, max = 50))]
    pub asset_code: Option<String>,
    
    #[validate(range(min = 0.0, max = 100.0))]
    pub success_rate_min: Option<f64>,
}
```

---

### Issue #15: Implement Graceful Shutdown

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `reliability`

**Description:**
The backend doesn't handle shutdown signals gracefully. When receiving SIGTERM or SIGINT, it terminates immediately without:
- Finishing in-flight requests
- Closing database connections
- Flushing caches
- Cleaning up resources

**Current Behavior:**
- Immediate termination on signal
- In-flight requests dropped
- Connections not closed properly
- Potential data loss

**Expected Behavior:**
- Catch SIGTERM/SIGINT signals
- Stop accepting new requests
- Wait for in-flight requests (with timeout)
- Close database connections
- Flush caches
- Clean shutdown

**Affected Files:**
- `backend/src/main.rs`
- New file: `backend/src/shutdown.rs`

**Acceptance Criteria:**
- [ ] Handle SIGTERM and SIGINT
- [ ] Implement graceful shutdown
- [ ] Add configurable timeout
- [ ] Close all connections
- [ ] Flush caches
- [ ] Log shutdown process
- [ ] Test shutdown behavior
- [ ] Document shutdown process

**Implementation Steps:**
1. Add `tokio::signal` handling
2. Create shutdown channel
3. Implement graceful server shutdown
4. Add connection cleanup
5. Add cache flushing
6. Add timeout handling
7. Add logging
8. Test with signals

**Implementation:**
```rust
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
```

---

### Issue #16: Add Database Connection Pooling Configuration

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `database`, `performance`

**Description:**
Database connection pool settings are not configurable and use defaults. This can lead to performance issues under load or resource exhaustion.

**Current Behavior:**
- Default SQLx pool settings
- No configuration options
- Potential connection exhaustion
- No connection timeout settings
- No idle connection management

**Expected Behavior:**
- Configurable pool size
- Connection timeout settings
- Idle connection timeout
- Connection lifetime limits
- Pool monitoring

**Affected Files:**
- `backend/src/database.rs`
- `backend/.env.example`
- `backend/src/main.rs`

**Acceptance Criteria:**
- [ ] Add pool configuration options
- [ ] Make settings configurable via env vars
- [ ] Add connection timeout
- [ ] Add idle timeout
- [ ] Add max lifetime
- [ ] Add pool metrics
- [ ] Document configuration
- [ ] Add tests

**Configuration Options:**
```toml
# .env
DB_POOL_MAX_CONNECTIONS=10
DB_POOL_MIN_CONNECTIONS=2
DB_POOL_CONNECT_TIMEOUT_SECONDS=30
DB_POOL_IDLE_TIMEOUT_SECONDS=600
DB_POOL_MAX_LIFETIME_SECONDS=1800
```

**Implementation:**
```rust
let pool = SqlitePoolOptions::new()
    .max_connections(config.max_connections)
    .min_connections(config.min_connections)
    .connect_timeout(Duration::from_secs(config.connect_timeout))
    .idle_timeout(Duration::from_secs(config.idle_timeout))
    .max_lifetime(Duration::from_secs(config.max_lifetime))
    .connect(&config.database_url)
    .await?;
```

---

### Issue #17: Add Background Job System for Data Refresh

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `background-jobs`

**Description:**
There's no background job system to periodically refresh data, update caches, or perform maintenance tasks. All data fetching happens on-demand, leading to slow first requests.

**Current Behavior:**
- All data fetched on request
- No background updates
- Cold cache on startup
- Slow initial responses

**Expected Behavior:**
- Background job scheduler
- Periodic data refresh
- Cache warming
- Maintenance tasks
- Configurable schedules

**Use Cases:**
1. Refresh corridor data every 5 minutes
2. Update anchor metrics every 10 minutes
3. Fetch new payments every minute
4. Clean up old cache entries
5. Update price feeds

**Affected Files:**
- New file: `backend/src/jobs/mod.rs`
- New file: `backend/src/jobs/scheduler.rs`
- New file: `backend/src/jobs/corridor_refresh.rs`
- `backend/src/main.rs`

**Acceptance Criteria:**
- [ ] Implement job scheduler
- [ ] Add corridor refresh job
- [ ] Add anchor refresh job
- [ ] Add price feed update job
- [ ] Add cache cleanup job
- [ ] Make schedules configurable
- [ ] Add job monitoring
- [ ] Add error handling
- [ ] Document job system

**Implementation Steps:**
1. Choose job library (`tokio-cron-scheduler` or custom)
2. Create job scheduler
3. Implement individual jobs
4. Add configuration
5. Add monitoring
6. Add error handling
7. Test jobs
8. Document

**Example:**
```rust
use tokio_cron_scheduler::{JobScheduler, Job};

async fn setup_jobs(
    scheduler: &JobScheduler,
    db: Arc<Database>,
    cache: Arc<CacheManager>,
    rpc: Arc<StellarRpcClient>,
) -> Result<()> {
    // Refresh corridors every 5 minutes
    scheduler.add(Job::new_async("0 */5 * * * *", move |_uuid, _l| {
        Box::pin(async move {
            refresh_corridors(&db, &cache, &rpc).await;
        })
    })?)?;
    
    Ok(())
}
```

---

### Issue #18: Add Request/Response Compression

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `performance`

**Description:**
The API doesn't compress responses, leading to larger payload sizes and slower response times, especially for large corridor/anchor lists.

**Current Behavior:**
- No compression
- Large JSON responses
- Higher bandwidth usage
- Slower response times

**Expected Behavior:**
- Gzip/Brotli compression
- Automatic compression for large responses
- Configurable compression level
- Accept-Encoding header support

**Affected Files:**
- `backend/src/main.rs` (add compression middleware)

**Acceptance Criteria:**
- [ ] Add compression middleware
- [ ] Support gzip and brotli
- [ ] Make compression configurable
- [ ] Set appropriate headers
- [ ] Add compression metrics
- [ ] Test with large responses
- [ ] Document compression

**Implementation:**
```rust
use tower_http::compression::CompressionLayer;

let app = Router::new()
    .route("/api/corridors", get(list_corridors))
    .layer(CompressionLayer::new());
```

---

### Issue #19: Implement API Versioning Strategy

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `api`, `architecture`

**Description:**
The API has no versioning strategy. Future breaking changes will affect all clients without migration path.

**Current Behavior:**
- No API versioning
- All endpoints at `/api/*`
- No backward compatibility strategy
- Breaking changes affect all clients

**Expected Behavior:**
- Version prefix in URLs (`/api/v1/*`)
- Support multiple versions simultaneously
- Deprecation warnings
- Migration guides
- Version negotiation

**Affected Files:**
- `backend/src/main.rs` (route structure)
- All API handlers
- New file: `backend/src/api/v1/mod.rs`

**Acceptance Criteria:**
- [ ] Add version prefix to routes
- [ ] Support v1 API
- [ ] Add version negotiation
- [ ] Add deprecation headers
- [ ] Document versioning strategy
- [ ] Create migration guide template
- [ ] Add version to health endpoint

**Implementation:**
```rust
let v1_routes = Router::new()
    .route("/corridors", get(v1::list_corridors))
    .route("/anchors", get(v1::list_anchors));

let app = Router::new()
    .nest("/api/v1", v1_routes)
    .route("/health", get(health_check));
```

---

### Issue #20: Add CORS Configuration

**Priority:** Medium  
**Type:** Enhancement  
**Labels:** `enhancement`, `medium`, `security`, `api`

**Description:**
CORS is not properly configured, which could cause issues when the frontend is deployed to a different domain or when external clients try to access the API.

**Current Behavior:**
- No CORS middleware
- Cross-origin requests may fail
- No preflight handling
- Potential security issues

**Expected Behavior:**
- Proper CORS configuration
- Configurable allowed origins
- Support for preflight requests
- Secure default settings
- Development vs production configs

**Affected Files:**
- `backend/src/main.rs`
- `backend/.env.example`

**Acceptance Criteria:**
- [ ] Add CORS middleware
- [ ] Make origins configurable
- [ ] Support preflight requests
- [ ] Add secure defaults
- [ ] Different configs for dev/prod
- [ ] Document CORS setup
- [ ] Test cross-origin requests

**Implementation:**
```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin(config.allowed_origins.parse::<HeaderValue>()?)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers(Any)
    .max_age(Duration::from_secs(3600));

let app = Router::new()
    .route("/api/corridors", get(list_corridors))
    .layer(cors);
```

**Configuration:**
```bash
# Development
CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001

# Production
CORS_ALLOWED_ORIGINS=https://stellar-insights.com
```

---

## 🔵 Low Priority Issues

### Issue #21: Add Request ID Tracking

**Priority:** Low  
**Type:** Enhancement  
**Labels:** `enhancement`, `low`, `observability`

**Description:**
No request ID tracking makes it difficult to trace requests through logs and correlate errors across services.

**Expected Behavior:**
- Generate unique request ID
- Add to response headers
- Include in all logs
- Propagate to downstream services

**Implementation:**
```rust
use uuid::Uuid;

async fn request_id_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let request_id = Uuid::new_v4().to_string();
    req.extensions_mut().insert(RequestId(request_id.clone()));
    
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "X-Request-ID",
        HeaderValue::from_str(&request_id).unwrap()
    );
    response
}
```

---

### Issue #22: Add SQL Query Logging

**Priority:** Low  
**Type:** Enhancement  
**Labels:** `enhancement`, `low`, `database`, `debugging`

**Description:**
Database queries are not logged, making it difficult to debug performance issues or identify slow queries.

**Expected Behavior:**
- Log all SQL queries in development
- Log slow queries in production
- Include query duration
- Configurable logging level

**Implementation:**
```rust
let pool = SqlitePoolOptions::new()
    .after_connect(|conn, _meta| {
        Box::pin(async move {
            conn.execute("PRAGMA journal_mode=WAL").await?;
            Ok(())
        })
    })
    .connect(&database_url)
    .await?;
```

---

### Issue #23: Add Database Migration Rollback Support

**Priority:** Low  
**Type:** Enhancement  
**Labels:** `enhancement`, `low`, `database`

**Description:**
SQLx migrations don't have rollback scripts. Failed migrations or schema changes can't be easily reverted.

**Expected Behavior:**
- Add down migrations
- Support rollback command
- Test migrations both ways
- Document migration process

**Files to Create:**
- `backend/migrations/*_down.sql` for each migration

---

### Issue #24: Add API Response Caching Headers

**Priority:** Low  
**Type:** Enhancement  
**Labels:** `enhancement`, `low`, `caching`, `performance`

**Description:**
API responses don't include caching headers (Cache-Control, ETag, Last-Modified), preventing client-side caching.

**Expected Behavior:**
- Add Cache-Control headers
- Implement ETag support
- Add Last-Modified headers
- Support conditional requests (304 Not Modified)

**Implementation:**
```rust
async fn add_cache_headers(
    response: Response,
    ttl: Duration,
) -> Response {
    let mut response = response;
    response.headers_mut().insert(
        "Cache-Control",
        HeaderValue::from_str(&format!("public, max-age={}", ttl.as_secs())).unwrap()
    );
    response
}
```

---

### Issue #25: Add Structured Error Responses

**Priority:** Low  
**Type:** Enhancement  
**Labels:** `enhancement`, `low`, `api`

**Description:**
Error responses are inconsistent and don't follow a standard format, making it hard for clients to handle errors.

**Expected Behavior:**
- Consistent error format
- Error codes
- Helpful error messages
- Stack traces in development only

**Error Format:**
```json
{
  "error": {
    "code": "CORRIDOR_NOT_FOUND",
    "message": "Corridor with ID 'USDC-XLM' not found",
    "details": {
      "corridor_id": "USDC-XLM"
    },
    "request_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

---

### Issue #26: Add Performance Benchmarks

**Priority:** Low  
**Type:** Testing  
**Labels:** `testing`, `low`, `performance`

**Description:**
No performance benchmarks to track API performance over time or identify regressions.

**Expected Behavior:**
- Benchmark critical endpoints
- Track performance over time
- Identify regressions
- Compare with baselines

**Implementation:**
```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_list_corridors(c: &mut Criterion) {
        c.bench_function("list_corridors", |b| {
            b.iter(|| {
                // Benchmark code
            });
        });
    }

    criterion_group!(benches, bench_list_corridors);
    criterion_main!(benches);
}
```

---

### Issue #27: Add Load Testing Suite

**Priority:** Low  
**Type:** Testing  
**Labels:** `testing`, `low`, `performance`

**Description:**
No load testing to verify the backend can handle expected traffic levels.

**Expected Behavior:**
- Load test critical endpoints
- Test with realistic data volumes
- Identify bottlenecks
- Document performance limits

**Tools:**
- k6
- Apache Bench
- wrk

**Example k6 Script:**
```javascript
import http from 'k6/http';
import { check } from 'k6';

export let options = {
  stages: [
    { duration: '2m', target: 100 },
    { duration: '5m', target: 100 },
    { duration: '2m', target: 0 },
  ],
};

export default function () {
  let response = http.get('http://localhost:8080/api/corridors');
  check(response, {
    'status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });
}
```

---

### Issue #28: Add Integration Tests for RPC Client

**Priority:** Low  
**Type:** Testing  
**Labels:** `testing`, `low`, `rpc`

**Description:**
RPC client has unit tests with mocks but no integration tests against real Stellar testnet.

**Expected Behavior:**
- Integration tests against testnet
- Test all RPC methods
- Verify response parsing
- Test error handling

**Implementation:**
```rust
#[tokio::test]
#[ignore] // Run with --ignored flag
async fn test_fetch_payments_integration() {
    let client = StellarRpcClient::new(
        "https://horizon-testnet.stellar.org".to_string(),
        "https://soroban-testnet.stellar.org".to_string(),
        false,
    );
    
    let payments = client.fetch_payments(10, None).await.unwrap();
    assert!(!payments.is_empty());
}
```

---

### Issue #29: Add Docker Health Check

**Priority:** Low  
**Type:** DevOps  
**Labels:** `devops`, `low`, `docker`

**Description:**
Dockerfile doesn't include HEALTHCHECK instruction, preventing Docker from monitoring container health.

**Expected Behavior:**
- Add HEALTHCHECK to Dockerfile
- Use /health endpoint
- Configure appropriate intervals
- Document health check behavior

**Implementation:**
```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1
```

---

### Issue #30: Add Automated Dependency Updates

**Priority:** Low  
**Type:** DevOps  
**Labels:** `devops`, `low`, `maintenance`

**Description:**
No automated dependency update process. Dependencies can become outdated and vulnerable.

**Expected Behavior:**
- Automated dependency updates
- Security vulnerability scanning
- Automated PR creation
- CI validation of updates

**Tools:**
- Dependabot (GitHub)
- Renovate
- cargo-audit for security

**Dependabot Configuration:**
```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: "cargo"
    directory: "/backend"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 10
    reviewers:
      - "backend-team"
    labels:
      - "dependencies"
      - "rust"
```

---

## Summary Statistics

**Total Issues:** 30

**By Priority:**
- 🔴 Critical: 5 issues
- 🟡 High: 7 issues  
- 🟢 Medium: 8 issues
- 🔵 Low: 10 issues

**By Type:**
- Bug: 5 issues
- Enhancement: 20 issues
- Security: 2 issues
- Documentation: 1 issue
- Testing: 3 issues
- DevOps: 2 issues
- Refactor: 2 issues

**By Component:**
- RPC/Stellar: 8 issues
- API: 7 issues
- Caching: 3 issues
- Database: 3 issues
- Observability: 3 issues
- Security: 3 issues
- Testing: 3 issues
- DevOps: 2 issues
- Performance: 2 issues

**Recommended First Sprint (Top 10):**
1. Issue #1 - Horizon API Parser (Critical)
2. Issue #2 - Corridor Detail Endpoint (Critical)
3. Issue #3 - Price Feed Integration (Critical)
4. Issue #4 - Asset Pair Detection (High)
5. Issue #5 - Database Credentials Security (Critical)
6. Issue #6 - RPC Pagination (High)
7. Issue #8 - Observability/Metrics (High)
8. Issue #9 - Rate Limiting (High)
9. Issue #10 - Remove Dead Code (High)
10. Issue #11 - Health Check Endpoints (High)

---

## Contract-Specific Issues

For completeness, here are the contract-related issues mentioned earlier:

### Contract Issue #1: Add Contract Integration Tests
**Priority:** High  
**Type:** Testing

No tests connecting Soroban contracts to the backend. Need integration tests to verify:
- Snapshot contract deployment
- Contract invocation from backend
- Event listening
- Data synchronization

### Contract Issue #2: Document Contract Deployment Process
**Priority:** High  
**Type:** Documentation

Missing clear deployment instructions:
- Network configurations
- Contract addresses
- Deployment scripts
- Upgrade procedures

### Contract Issue #3: Add Contract Event Monitoring
**Priority:** Medium  
**Type:** Enhancement

Backend doesn't listen to contract events:
- No event subscription
- Manual sync required
- Potential data inconsistency

### Contract Issue #4: Implement Contract Upgrade Strategy
**Priority:** Medium  
**Type:** Architecture

No versioning or upgrade strategy:
- No migration plan
- No version tracking
- Risk of breaking changes

### Contract Issue #5: Consolidate Contract Examples
**Priority:** Low  
**Type:** Refactor

Multiple example contracts with unclear purpose:
- Remove unused examples
- Document remaining contracts
- Clear separation of examples vs production

---

## Infrastructure Issues

### Infra Issue #1: Frontend Environment Variable Not Documented
**Priority:** High  
**Type:** Bug

`NEXT_PUBLIC_API_URL` required but not in `.env.example`. Add to documentation and example files.

### Infra Issue #2: Add Docker Compose for Local Development
**Priority:** High  
**Type:** DevOps

No easy way to run full stack locally. Create `docker-compose.yml` with:
- Backend service
- Frontend service
- Redis
- Database volumes

### Infra Issue #3: GitHub Actions Missing Environment Secrets
**Priority:** Medium  
**Type:** CI/CD

Workflows reference undefined secrets. Document required secrets and add to repository settings.

### Infra Issue #4: Add Database Migration Strategy
**Priority:** Medium  
**Type:** Database

SQLite migrations not properly versioned:
- Duplicate migration files
- No rollback support
- Unclear migration order

---

## Next Steps

1. **Triage**: Review and prioritize issues with team
2. **Label**: Add appropriate labels in GitHub
3. **Assign**: Assign issues to team members
4. **Sprint Planning**: Select issues for first sprint
5. **Create Issues**: Convert this document to GitHub issues
6. **Track Progress**: Use GitHub Projects or similar tool

## Issue Creation Template

When creating GitHub issues from this document, use this format:

```markdown
## Description
[Copy from issue description]

## Current Behavior
[Copy from current behavior section]

## Expected Behavior
[Copy from expected behavior section]

## Affected Files
[List affected files]

## Acceptance Criteria
[Copy checklist]

## Implementation Steps
[Copy implementation steps]

## Technical Details
[Copy any code examples or technical details]

## Priority
[Critical/High/Medium/Low]

## Labels
[List appropriate labels]
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-02-02  
**Author:** AI Assistant  
**Status:** Ready for Review
