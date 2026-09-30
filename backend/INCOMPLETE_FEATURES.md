# Incomplete Features Tracking (Issue #2326)

This document tracks features that are partially implemented or use mock/simplified implementations that need to be completed for production readiness.

## High Priority - Core Functionality

### 1. Transaction Signing and Submission
**File**: `backend/src/api/transactions.rs` (lines ~377-383)
**Status**: Mock implementation  
**Impact**: Users cannot submit actual transactions to Stellar network; only simulated responses

**Current Implementation**:
- Returns mocked transaction hash (random UUID)
- Does not unpack XDR transaction format
- Does not attach signatures to transaction
- Does not submit to Stellar network

**Required Implementation**:
- [ ] Unpack XDR transaction envelope using stellar-sdk
- [ ] Attach collected signatures to transaction
- [ ] Submit to Stellar network via HTTP RPC calls
- [ ] Return actual Stellar transaction hash on success
- [ ] Handle network submission failures with retry logic

**Dependencies**: `stellar-sdk` crate, error handling for network failures

---

### 2. SEP-10 Authentication - Challenge Generation
**File**: `backend/src/auth/sep10_simple.rs` (lines ~125-170)
**Status**: Simplified implementation without proper Stellar transactions  
**Impact**: Authentication does not use cryptographically secure Stellar transaction format

**Current Implementation**:
- Creates JSON challenge structure instead of Stellar transaction envelope
- Base64 encodes JSON instead of XDR transaction
- Does not sign with server key

**Required Implementation**:
- [ ] Build proper Stellar transaction envelope using stellar-sdk
- [ ] Include ManageData operations with challenge nonce
- [ ] Sign transaction with server keypair
- [ ] Return base64-encoded XDR (not JSON)

**Dependencies**: `stellar-sdk` crate for transaction construction

---

### 3. SEP-10 Authentication - Signature Verification
**File**: `backend/src/auth/sep10_simple.rs` (lines ~172-210)
**Status**: Simplified - only validates nonce, not signatures  
**Impact**: Security risk - invalid signatures are accepted if nonce is valid

**Current Implementation**:
- Only validates challenge nonce and expiration
- Does not verify Ed25519 signatures
- Does not validate transaction envelope format

**Required Implementation**:
- [ ] Decode and validate Stellar transaction envelope format
- [ ] Verify Ed25519 signature against server public key
- [ ] Validate ManageData operations contain expected nonce
- [ ] Ensure transaction structure matches SEP-10 specification

**Dependencies**: `stellar-sdk` crate for signature verification

---

## Medium Priority - Accuracy and Reliability

### 4. Slippage Calculation with Market Data
**File**: `backend/src/api/cost_calculator.rs` (lines ~289-292)
**Status**: Static fee-based calculation  
**Impact**: Slippage estimates are inaccurate; users may experience worse rates than quoted

**Current Implementation**:
- Uses hardcoded base slippage (8-12 basis points)
- Uses hardcoded variable slippage per 10k source amount (2-4 bps)
- Does not consider order book depth
- Does not account for market volatility

**Required Implementation**:
- [ ] Query real-time order book depth from Stellar CLOB
- [ ] Calculate market impact based on trade size vs. liquidity
- [ ] Factor in recent volatility to adjust slippage estimate
- [ ] Cache order book data with configurable TTL
- [ ] Fall back to static estimates if order book unavailable

**Dependencies**: Real-time price/liquidity data source

---

### 5. Anchor Monitor - Configurable Alert Thresholds
**File**: `backend/src/services/anchor_monitor.rs` (lines ~139-162)
**Status**: Hardcoded thresholds  
**Impact**: Alert sensitivity cannot be tuned; may miss critical issues or spam with false alerts

**Current Implementation**:
- Success rate drop threshold: hardcoded 10%
- Latency increase threshold: hardcoded 50% (1.5x multiplier)
- No way to adjust thresholds without code changes
- No per-anchor threshold customization

**Required Implementation**:
- [ ] Load alert thresholds from configuration file or database
- [ ] Support global default thresholds
- [ ] Allow per-anchor threshold overrides
- [ ] Reload thresholds without service restart
- [ ] Log threshold changes for audit trail

**Dependencies**: Configuration system (environment variables or config file)

---

### 6. Premium Tier Detection - Subscription Caching
**File**: `backend/src/rate_limit.rs` (lines ~288-313)
**Status**: Queries database on every rate limit check  
**Impact**: Performance degradation; database query on every API request for authenticated users

**Current Implementation**:
- Database query on every rate limit check
- No caching of subscription status
- No fallback if database is unavailable
- Potential rate limit queries during database outages

**Required Implementation**:
- [ ] Cache subscription tier with configurable TTL (recommended: 5-10 minutes)
- [ ] Implement cache invalidation on subscription changes
- [ ] Use in-memory cache layer before database lookup
- [ ] Graceful fallback to Authenticated tier on cache miss + DB error
- [ ] Log cache hit rates for monitoring

**Dependencies**: Existing cache infrastructure (appears to be available from anchor_monitor.rs)

---

## Lower Priority - Analytics and ML

### 7. ML Training Data - Use Real Historical Data
**File**: `backend/src/ml.rs` (lines ~104-125)
**Status**: Synthetic data generation  
**Impact**: ML predictions trained on artificial patterns, not reflective of real payment behavior

**Current Implementation**:
- Generates 1000 synthetic training samples
- Uses formula-based features, not real transaction data
- Does not incorporate actual success/failure outcomes

**Required Implementation**:
- [ ] Query historical transactions from database (last 30-90 days)
- [ ] Extract real features: amount, corridor, time-of-day, success/failure
- [ ] Normalize feature distributions
- [ ] Retrain model periodically with fresh data
- [ ] Handle insufficient data gracefully

**Dependencies**: Transaction history queries, data normalization

---

### 8. ML Model Features - Real Corridor Liquidity
**File**: `backend/src/ml.rs` (lines ~165-172)
**Status**: Mock data based on corridor name  
**Impact**: Model features do not reflect real market conditions

**Current Implementation**:
- `get_corridor_liquidity()`: returns `(corridor.len() * 100) + 1000` (fake formula)
- `get_recent_success_rate()`: returns formula based on corridor name length
- No actual order book or transaction history queried

**Required Implementation**:
- [ ] Query Stellar order book for corridor asset pairs
- [ ] Sum liquidity across all market makers
- [ ] Query transaction success/failure rate from recent history
- [ ] Cache these metrics with 1-5 minute TTL
- [ ] Fall back to default values if data unavailable

**Dependencies**: Order book data source, transaction history queries

---

### 9. Analytics Dashboard - Real Time Series Data
**File**: `backend/src/api/analytics_dashboard.rs` (lines ~91-115)
**Status**: Hardcoded mock time series  
**Impact**: Dashboard displays example data only; network trends not visible to users

**Current Implementation**:
- Returns 7 hardcoded data points (midnight through 23:59)
- Fixed volumes: 45k to 72k USD
- Fixed corridor/anchor counts
- Same data on every request

**Required Implementation**:
- [ ] Query transactions aggregated by hour
- [ ] Calculate active corridor count per hour
- [ ] Calculate active anchor count per hour
- [ ] Sum transaction volumes per hour
- [ ] Return data for last 24-48 hours
- [ ] Cache results with 5-minute TTL

**Dependencies**: Aggregation queries on transaction history

---

### 10. Alert Delivery - Retry and Persistence
**File**: `backend/src/services/alert_service.rs` (lines ~77-153)
**Status**: Fire-and-forget delivery, no retry  
**Impact**: Alert delivery failures are silent; critical alerts may not reach operators

**Current Implementation**:
- Sends alerts to configured channels
- If channel is unavailable or service is down, alert is lost
- No persistence of failed alerts
- No retry mechanism

**Required Implementation**:
- [ ] Store alerts in database before delivery
- [ ] Implement retry queue for failed deliveries
- [ ] Exponential backoff for retries (max 5 retries over 24 hours)
- [ ] Log delivery attempts and outcomes
- [ ] Alert if critical alerts fail to deliver

**Dependencies**: Database for alert persistence, job queue for retries

---

## Implementation Priority

1. **Critical (blocking production)**: #1-3 (Transaction signing, SEP-10 auth)
2. **High (significant functionality gaps)**: #4-6 (Slippage, thresholds, caching)
3. **Medium (analytics/insights)**: #7-10 (ML, dashboard, alert delivery)

## Configuration Requirements

Features #5-10 require configuration system support:
- Environment variables for thresholds and TTLs
- Database schema for per-anchor overrides
- Cache configuration for TTL values

## Testing Requirements

Each feature requires:
- Unit tests with real data (not mocks)
- Integration tests with actual Stellar testnet for #1-3
- Performance benchmarks for #6 (caching impacts)
- Dashboard accuracy tests for #9

---

*This tracking document was created to address issue #2326 and should be updated as each feature is completed.*
