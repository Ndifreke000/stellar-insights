---
title: "[BUG] Corridor Detail Endpoint Returns 404 - Stub Implementation"
labels: bug, critical, api, corridors, backend
assignees: ''
---

## 🐛 Description

The `get_corridor_detail()` function in the corridors API is currently a stub that always returns a 404 NotFound error. This breaks the corridor detail page in the frontend, preventing users from viewing detailed metrics for specific payment corridors.

## 📊 Current Behavior

```rust
pub async fn get_corridor_detail(...) -> ApiResult<Json<CorridorDetailResponse>> {
    Err(crate::handlers::ApiError::NotFound(
        "Corridor detail endpoint not yet implemented with RPC".to_string()
    ))
}
```

All requests to `/api/corridors/{corridor_id}` return 404.

## ✅ Expected Behavior

- Fetch detailed payment and trade data for specific corridor from RPC
- Calculate historical success rates over time periods
- Generate latency distribution buckets
- Compute liquidity trends
- Return comprehensive corridor metrics

## 📁 Affected Files

- `backend/src/api/corridors_cached.rs` (lines 180-186)
- Frontend: `frontend/src/app/corridors/[id]/page.tsx`

## 🔧 Technical Requirements

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

## 📋 Acceptance Criteria

- [ ] Implement full `get_corridor_detail()` function
- [ ] Fetch RPC data for specific asset pair
- [ ] Calculate all required metrics
- [ ] Return properly formatted `CorridorDetailResponse`
- [ ] Add caching with 5-minute TTL
- [ ] Handle invalid corridor IDs gracefully
- [ ] Add integration tests
- [ ] Update API documentation

## 🚀 Implementation Steps

1. Parse and validate corridor_key parameter
2. Extract source and destination asset info
3. Call `rpc_client.fetch_payments()` with appropriate filters
4. Implement `calculate_historical_success_rate()` helper
5. Implement `calculate_latency_distribution()` helper
6. Implement `calculate_liquidity_trends()` helper
7. Query related corridors from cache or RPC
8. Assemble and return response
9. Add error handling for each step

## 📊 Data Structures Needed

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

## 🏷️ Priority

**Critical** - Frontend corridor detail page is completely broken

## 🔗 Related Issues

- Depends on #1 (Horizon API Parser) being fixed first
