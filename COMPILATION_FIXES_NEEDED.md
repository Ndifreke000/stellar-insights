# Backend Compilation Fixes Needed

## Summary
101 compilation errors need to be fixed. These are related to:
- Prometheus metrics API changes
- Database Clone trait
- Async trait compatibility
- Type serialization issues

## Critical Fixes Required

### 1. Database Clone Issue (2 errors)
**Files**: `backend/src/api/v1/mod.rs:33`, `backend/src/database.rs`
**Problem**: `Database` doesn't implement `Clone`
**Solution**: Wrap Database in `Arc<Database>` throughout the codebase

### 2. Prometheus Metrics API Changes (40+ errors)
**Files**: `backend/src/observability/metrics.rs`, `backend/src/observability/job_metrics.rs`
**Problem**: `label_names()` method doesn't exist in prometheus 0.13
**Solution**: Use the new API - create metric families with labels differently

Example old code:
```rust
Opts::new("metric_name", "description")
    .label_names(vec!["label1", "label2"])
```

New API (prometheus 0.13):
```rust
// Use IntCounterVec, IntGaugeVec, HistogramVec instead
let counter = IntCounterVec::new(
    Opts::new("metric_name", "description"),
    &["label1", "label2"]
).unwrap();
```

### 3. AlertHandler Trait Not Dyn-Compatible (2 errors)
**File**: `backend/src/observability/job_alerts.rs:128,134`
**Problem**: Async trait methods can't be used with `dyn Trait`
**Solution**: Use `async_trait` macro or enum dispatch

```rust
#[async_trait]
pub trait AlertHandler: Send + Sync {
    async fn send_alert(&self, alert: &JobAlert);
}
```

### 4. Sha256 Digest Formatting (2 errors)
**Files**: `backend/src/cache/helpers.rs:158`, `backend/src/http_cache.rs:130`
**Problem**: `Sha256::digest()` result doesn't implement `LowerHex`
**Solution**: Use hex encoding

```rust
use hex;
let digest = Sha256::digest(&body);
let etag = format!("\"{}\"", hex::encode(digest));
```

### 5. Instant Serialization (4 errors)
**File**: `backend/src/observability/job_metrics.rs:82-83`
**Problem**: `std::time::Instant` doesn't implement Serialize/Deserialize
**Solution**: Use `chrono::DateTime<Utc>` or remove Serialize/Deserialize

```rust
pub started_at: DateTime<Utc>,  // Instead of Instant
pub completed_at: Option<DateTime<Utc>>,
```

### 6. HMAC Version Mismatch (2 errors)
**File**: `backend/src/webhooks/mod.rs:24`
**Problem**: Multiple versions of `digest` crate causing conflicts
**Solution**: Update dependencies to use consistent versions

### 7. IntoResponse Not in Scope (1 error)
**File**: `backend/src/validation.rs:29`
**Solution**: Add import
```rust
use axum::response::IntoResponse;
```

## Quick Fix Priority

1. **Fix imports** (5 minutes)
   - Add missing `use` statements
   - Fix `IntoResponse` import

2. **Fix Prometheus metrics** (2-3 hours)
   - Update all metric definitions to use Vec types
   - Update all `with_label_values()` calls

3. **Fix Database Clone** (30 minutes)
   - Wrap Database in Arc throughout

4. **Fix async trait** (30 minutes)
   - Add `#[async_trait]` to AlertHandler

5. **Fix serialization** (30 minutes)
   - Change Instant to DateTime<Utc>
   - Fix Sha256 hex encoding

## Related GitHub Issues

These compilation errors are covered by the issues we just created:
- **Issue #1178**: Input sanitization (validation.rs)
- **Issue #1186**: API response times (metrics need to work)
- **Issue #1213**: Background job monitoring (job_metrics.rs)

## Estimated Time to Fix All
**Total**: 4-5 hours

## Commands to Test

```bash
# Check compilation
cd backend && cargo check

# Run clippy
cargo clippy --all-targets

# Run tests after fixes
cargo test
```
