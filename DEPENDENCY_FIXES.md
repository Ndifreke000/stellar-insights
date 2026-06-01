# Dependency Build Fixes

## Summary
Fixed critical build failures in the backend due to missing and incorrect dependencies.

## Issues Fixed

### 1. ✅ Removed Non-Existent Dependency
**File:** `backend/apm/Cargo.toml`
- **Issue:** `opentelemetry-attribute-utils` crate does not exist on crates.io
- **Fix:** Removed the dependency (it was not used in the code)

### 2. ✅ Fixed Typo in Dependency Name
**File:** `backend/Cargo.toml`
- **Issue:** `sprometheus` (typo) instead of `prometheus`
- **Fix:** Corrected to `prometheus = "0.13"`

### 3. ✅ Added Missing Dependencies
**File:** `backend/Cargo.toml`

Added the following missing dependencies:
- `redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }`
- `rand = "0.8"`
- `urlencoding = "2.1"`
- `futures = "0.3"`
- `utoipa = { version = "4.0", features = ["axum_extras"] }`
- `url = "2.5"`
- `aes-gcm = "0.10"`
- `base64 = "0.22"`
- `lettre = "0.11"`
- `jsonwebtoken = "9.2"`
- `hmac = "0.12"`
- `data-encoding = "2.5"`
- `async-lock = "3.3"`
- `dashmap = "5.5"`
- `lazy_static = "1.4"`

### 4. ✅ Fixed OpenTelemetry API Compatibility
**File:** `backend/apm/src/lib.rs`
- **Issue:** OpenTelemetry 0.21 API changes - `Counter`, `Histogram`, `Gauge` are no longer traits
- **Fix:** Removed invalid trait implementations and unused imports

### 5. ✅ Fixed Import Issues
- Added missing `HashMap` import in `backend/src/api/corridors_cached.rs`
- Fixed `AppError` → `ApiError` in multiple files:
  - `backend/src/gdpr/service.rs`
  - `backend/src/gdpr/handlers.rs`
  - `backend/src/api/replay_handlers.rs`

### 6. ✅ Fixed Code Issues
- Fixed duplicate `is_retryable` method in `backend/src/rpc/error.rs`
- Fixed borrow checker errors in:
  - `backend/src/ingestion/ledger.rs` (added `.clone()`)
  - `backend/src/replay/state_builder.rs` (added `.clone()`)
- Fixed type mismatch in `backend/src/api/corridors_cached.rs` (Duration → usize)
- Fixed method name: `internal_server_error` → `internal` in `backend/src/api/corridors_cached.rs`
- Removed duplicate `monitor` module declaration in `backend/src/lib.rs`

### 7. ⚠️ Temporarily Disabled GDPR Module
**Files:** `backend/src/lib.rs`, `backend/src/main.rs`
- **Issue:** GDPR module depends on `actix_web` framework (project uses `axum`)
- **Fix:** Commented out GDPR module and routes temporarily
- **TODO:** Port GDPR module to use Axum instead of Actix-web

### 8. ⚠️ Made APM Module Optional
**File:** `backend/Cargo.toml`
- **Issue:** APM module has OpenTelemetry API compatibility issues
- **Fix:** Made `stellar-insights-apm` an optional dependency
- **TODO:** Update APM module to work with OpenTelemetry 0.21 API

## Remaining Issue

### SQLx Compile-Time Verification
**Status:** ⚠️ Requires Database Setup

The project uses `sqlx::query!` macros which perform compile-time SQL validation. This requires either:

1. **Option A (Recommended):** Set up database and generate cache
   ```bash
   # Start PostgreSQL
   docker run --name stellar-postgres \
     -e POSTGRES_PASSWORD=password \
     -e POSTGRES_DB=stellar_insights \
     -p 5432:5432 -d postgres:14
   
   # Set DATABASE_URL
   export DATABASE_URL="postgresql://postgres:password@localhost:5432/stellar_insights"
   
   # Run migrations (if migration tool exists)
   # sqlx migrate run
   
   # Generate SQLx cache
   cargo sqlx prepare
   ```

2. **Option B:** Use `SQLX_OFFLINE=true` with pre-generated cache
   - Requires `.sqlx/query-metadata.json` to be committed to git
   - Currently missing from the repository

3. **Option C:** Convert `sqlx::query!` to `sqlx::query` (runtime queries)
   - Loses compile-time SQL validation
   - More error-prone

## Testing

To verify fixes without database:
```bash
cd backend

# Check syntax (will fail on SQLx macros)
cargo check

# With database setup:
export DATABASE_URL="postgresql://postgres:password@localhost:5432/stellar_insights"
cargo build
cargo clippy --all-targets --all-features
cargo test
```

## CI/CD Impact

The GitHub Actions workflows should pass once:
1. Database is set up in CI (or)
2. SQLx prepared data is committed to the repository

Current CI workflows:
- `clippy.yml` - Will need database or SQLx cache
- `deploy.yml` - Will need database or SQLx cache

## Files Modified

1. `backend/apm/Cargo.toml` - Removed non-existent dependency
2. `backend/Cargo.toml` - Fixed typo, added missing dependencies, made APM optional
3. `backend/apm/src/lib.rs` - Fixed OpenTelemetry API compatibility
4. `backend/src/lib.rs` - Removed duplicate module, commented out GDPR
5. `backend/src/main.rs` - Commented out GDPR service and routes
6. `backend/src/api/corridors_cached.rs` - Added HashMap import, fixed method names, fixed types
7. `backend/src/gdpr/service.rs` - Fixed AppError → ApiError
8. `backend/src/gdpr/handlers.rs` - Fixed AppError → ApiError
9. `backend/src/api/replay_handlers.rs` - Fixed AppError → ApiError
10. `backend/src/rpc/error.rs` - Removed duplicate method
11. `backend/src/ingestion/ledger.rs` - Fixed borrow checker error
12. `backend/src/replay/state_builder.rs` - Fixed borrow checker error
13. `backend/.env` - Created with DATABASE_URL (for local development)

## Next Steps

1. **Immediate:** Set up PostgreSQL database and run migrations
2. **Short-term:** Generate and commit SQLx prepared data
3. **Medium-term:** Fix APM module OpenTelemetry compatibility
4. **Medium-term:** Port GDPR module from Actix-web to Axum
5. **Long-term:** Consider if compile-time SQL validation is worth the complexity
