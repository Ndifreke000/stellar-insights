# Stellar Insights - Comprehensive Project Issues

**Generated:** 2026-03-26  
**Scope:** Frontend (Next.js/React), Backend (Rust/Axum), Contracts (Soroban)  
**Status:** Complete project scan with compilation errors, warnings, and code quality issues

---

## ⚠️ CRITICAL POLICY

**Before pushing any fix:**
1. **Backend:** Run `cargo clippy --all-targets --all-features` and `cargo test --all-features`
2. **Contracts:** Run `cargo clippy --all-targets --all-features` and `cargo test --all-features`
3. **Frontend:** Run `npm run lint` and `npm run type-check` and `npm test`
4. **DO NOT push standalone .md files** - issues must be tracked in GitHub Issues
5. All tests must pass before merging

---

## 🔴 CRITICAL COMPILATION ERRORS (Must Fix Immediately)

### ERROR-001: Backend - Missing Module `metrics_cached`
**Priority:** CRITICAL (Blocks Compilation)  
**Category:** Build Error  
**File:** `backend/src/api/mod.rs:20`

**Error:**
```
error[E0583]: file not found for module `metrics_cached`
  --> src/api/mod.rs:20:1
   |
20 | pub mod metrics_cached;
   | ^^^^^^^^^^^^^^^^^^^^^^^
```

**Fix:**
Either create the missing file or remove the module declaration:

**Option 1 - Create the file:**
```bash
touch backend/src/api/metrics_cached.rs
```

**Option 2 - Remove the declaration:**
```rust
// backend/src/api/mod.rs
// Remove or comment out:
// pub mod metrics_cached;
```

**Estimated Effort:** 5 minutes

---

### ERROR-002: Backend - Mismatched Closing Delimiter in database.rs
**Priority:** CRITICAL (Blocks Compilation)  
**Category:** Syntax Error  
**File:** `backend/src/database.rs:1515`

**Error:**
```
error: mismatched closing delimiter: `)`
    --> src/database.rs:1515:56
     |
1504 |         self.execute_with_timing("validate_api_key", async {
     |                                 - closing delimiter possibly meant for this
...
1515 |             if let Some(ref expires_at) = k.expires_at {
     |                                                        ^ unclosed delimiter
...
1572 |         })
     |          ^ mismatched closing delimiter
```

**Fix:**
Check the `execute_with_timing` call around line 1504 and ensure proper bracket matching. The async block is not properly closed.

**Verification:**
```bash
cd backend
cargo check
```

**Estimated Effort:** 10 minutes

---

### ERROR-003: Contracts - Unclosed Delimiter in analytics contract
**Priority:** CRITICAL (Blocks Compilation)  
**Category:** Syntax Error  
**File:** `contracts/analytics/src/lib.rs`

**Error:**
```
error: this file contains an unclosed delimiter
error: could not compile `analytics` (lib) due to 1 previous error
```

**Fix:**
Review the analytics contract for unclosed braces, brackets, or parentheses. Use an IDE with bracket matching or run:

```bash
cd contracts/analytics
cargo check
```

**Estimated Effort:** 15 minutes

---

### ERROR-004: Contracts - Multiple Compilation Errors in stellar-insights contract
**Priority:** CRITICAL (Blocks Compilation)  
**Category:** Build Error  
**File:** `contracts/stellar_insights/src/lib.rs`

**Error:**
```
error: could not compile `stellar-insights` (lib) due to 18 previous errors; 14 warnings emitted
```

**Fix:**
Run detailed clippy to see all errors:

```bash
cd contracts
cargo clippy --package stellar-insights --all-targets 2>&1 | less
```

**Estimated Effort:** 1-2 hours

---

### ERROR-005: Contracts - Access Control Compilation Errors
**Priority:** CRITICAL (Blocks Compilation)  
**Category:** Build Error  
**File:** `contracts/access-control/src/lib.rs`

**Error:**
```
error: could not compile `access-control` (lib) due to 9 previous errors
```

**Fix:**
Run detailed clippy:

```bash
cd contracts
cargo clippy --package access-control --all-targets 2>&1 | less
```

**Estimated Effort:** 1 hour

---

## 🔴 FRONTEND CRITICAL ISSUES

### ERROR-F001: React Effect Causing Cascading Renders
**Priority:** CRITICAL  
**Category:** Performance / React Best Practices  
**File:** `frontend/src/app/[locale]/quests/page.tsx:33`

**Error:**
```
Error: Calling setState synchronously within an effect can trigger cascading renders
```

**Current Code:**
```typescript
useEffect(() => {
  checkPathCompletion(pathname);
  setProgress(getProgress());  // ❌ Synchronous setState in effect
}, [pathname]);
```

**Fix:**
```typescript
// Option 1: Move state update to separate effect
useEffect(() => {
  checkPathCompletion(pathname);
}, [pathname]);

useEffect(() => {
  setProgress(getProgress());
}, [pathname]);

// Option 2: Use callback pattern
useEffect(() => {
  checkPathCompletion(pathname);
  // Only update if needed
  const newProgress = getProgress();
  setProgress(prev => prev !== newProgress ? newProgress : prev);
}, [pathname]);

// Option 3: Derive state instead
const progress = useMemo(() => getProgress(), [pathname]);
```

**Estimated Effort:** 30 minutes

---

## 🟠 HIGH PRIORITY ISSUES

### ISSUE-F002: TypeScript `any` Type Usage (7 instances)
**Priority:** High  
**Category:** Type Safety  
**Files:**
- `frontend/src/__tests__/api-client.test.ts` (7 instances)
- `frontend/src/__tests__/csrf.test.ts` (1 instance)

**Error:**
```
error  Unexpected any. Specify a different type  @typescript-eslint/no-explicit-any
```

**Fix:**
```typescript
// ❌ BAD
const mockFetch = vi.fn() as any;

// ✅ GOOD
const mockFetch = vi.fn() as unknown as typeof fetch;

// Or define proper type
interface MockResponse {
  ok: boolean;
  json: () => Promise<unknown>;
  status: number;
}

const mockFetch = vi.fn<[], Promise<MockResponse>>();
```

**Estimated Effort:** 1-2 hours

---

### ISSUE-F003: Forbidden `require()` Imports (4 instances)
**Priority:** High  
**Category:** Modern JavaScript / ES Modules  
**Files:**
- `frontend/next.config.ts:6`
- `frontend/scripts/replace-console-statements.js` (3 instances)

**Error:**
```
error  A `require()` style import is forbidden  @typescript-eslint/no-require-imports
```

**Fix:**
```typescript
// ❌ BAD
const path = require('path');

// ✅ GOOD
import path from 'path';

// For next.config.ts
import type { NextConfig } from 'next';

const config: NextConfig = {
  // ...
};

export default config;
```

**Estimated Effort:** 30 minutes

---

### ISSUE-F004: Unused Variables and Imports (20+ instances)
**Priority:** High  
**Category:** Code Quality  
**Files:** Multiple frontend files

**Examples:**
```typescript
// frontend/src/__tests__/CorridorComparison.test.tsx:2
import { render, screen, waitFor } from '@testing-library/react';  // waitFor unused

// frontend/src/app/[locale]/analytics/page.tsx:5-9
import { TrendingUp, Activity, Download } from 'lucide-react';  // All unused

// frontend/src/app/[locale]/corridors/page.tsx:17-18
import { MainLayout, SkeletonCorridorCard } from '@/components';  // Both unused
```

**Fix:**
Remove unused imports and variables or use them:

```typescript
// ✅ Remove if truly unused
import { render, screen } from '@testing-library/react';

// ✅ Or use them
<TrendingUp className="w-4 h-4" />
```

**Estimated Effort:** 2-3 hours

---

### ISSUE-F005: Missing React Hook Dependencies
**Priority:** High  
**Category:** React Hooks / Potential Bugs  
**File:** `frontend/src/app/[locale]/dashboard/page.tsx:85`

**Warning:**
```
React Hook useCallback has a missing dependency: 't'. Either include it or remove the dependency array
```

**Current Code:**
```typescript
const handleAction = useCallback(() => {
  console.log(t('some.key'));
}, []);  // ❌ Missing 't' dependency
```

**Fix:**
```typescript
const handleAction = useCallback(() => {
  console.log(t('some.key'));
}, [t]);  // ✅ Include 't' in dependencies
```

**Estimated Effort:** 15 minutes

---

## 🟡 MEDIUM PRIORITY ISSUES

### ISSUE-C001: Contracts - Profile Configuration Warnings (3 instances)
**Priority:** Medium  
**Category:** Build Configuration  
**Files:**
- `contracts/access-control/Cargo.toml`
- `contracts/stellar_insights/Cargo.toml`
- `contracts/governance/Cargo.toml`

**Warning:**
```
warning: profiles for the non root package will be ignored, specify profiles at the workspace root
```

**Fix:**
Remove `[profile.*]` sections from individual contract Cargo.toml files. Keep only in workspace root `contracts/Cargo.toml`.

**Estimated Effort:** 15 minutes

---

### ISSUE-C002: Contracts - Unreachable Patterns (14 warnings)
**Priority:** Medium  
**Category:** Code Quality  
**File:** `contracts/stellar_insights/src/lib.rs`

**Warning:**
```
warning: unreachable pattern
```

**Fix:**
Review match statements for unreachable patterns:

```rust
// ❌ BAD
match value {
    _ => {},  // Catches everything
    Some(x) => {},  // Unreachable!
}

// ✅ GOOD
match value {
    Some(x) => {},
    None => {},
}
```

**Estimated Effort:** 1-2 hours

---

### ISSUE-F006: Unused Helper Functions (6 instances)
**Priority:** Medium  
**Category:** Dead Code  
**Files:**
- `frontend/src/app/[locale]/anchors/page.tsx` (2 functions)
- `frontend/src/app/[locale]/corridors/page.tsx` (3 functions)
- `frontend/src/app/[locale]/corridors/[pair]/page.tsx` (1 variable)

**Examples:**
```typescript
// Defined but never used
const getHealthStatusColor = (status: string) => { /* ... */ };
const getHealthStatusIcon = (status: string) => { /* ... */ };
const corridorUpdates = useWebSocket('/ws/corridors');
```

**Fix:**
Either use them or remove them:

```typescript
// ✅ Use them
<Badge className={getHealthStatusColor(anchor.status)}>
  {getHealthStatusIcon(anchor.status)}
  {anchor.status}
</Badge>

// ✅ Or remove if not needed
// const getHealthStatusColor = ...
```

**Estimated Effort:** 1-2 hours

---

## 📊 SUMMARY BY AREA

### Backend Issues
- **Critical:** 2 compilation errors (ERROR-001, ERROR-002)
- **High:** TBD (need successful compilation first)
- **Medium:** TBD
- **Total Estimated:** 15 minutes + additional time after compilation fixes

### Contracts Issues
- **Critical:** 3 compilation errors (ERROR-003, ERROR-004, ERROR-005)
- **High:** TBD (need successful compilation first)
- **Medium:** 2 issues (profile warnings, unreachable patterns)
- **Total Estimated:** 2-4 hours + additional time after compilation fixes

### Frontend Issues
- **Critical:** 1 React effect issue (ERROR-F001)
- **High:** 4 issues (any types, require imports, unused vars, hook dependencies)
- **Medium:** 1 issue (unused helper functions)
- **Total Estimated:** 5-8 hours

### Overall Priority
1. **Fix all compilation errors first** (ERROR-001 through ERROR-005)
2. **Fix React effect cascading renders** (ERROR-F001)
3. **Address TypeScript type safety** (ISSUE-F002, ISSUE-F003)
4. **Clean up unused code** (ISSUE-F004, ISSUE-F006)
5. **Fix React hooks** (ISSUE-F005)
6. **Address contract warnings** (ISSUE-C001, ISSUE-C002)

---

## 🔧 RECOMMENDED FIX ORDER

### Phase 1: Compilation Fixes (CRITICAL - 2-4 hours)
1. Fix ERROR-001: Backend missing module
2. Fix ERROR-002: Backend syntax error
3. Fix ERROR-003: Contracts analytics syntax
4. Fix ERROR-004: Contracts stellar-insights errors
5. Fix ERROR-005: Contracts access-control errors

### Phase 2: Re-scan After Compilation (1-2 hours)
Once all code compiles, run full clippy scans again to identify remaining issues:
```bash
cargo clippy --manifest-path=backend/Cargo.toml --all-targets --all-features
cargo clippy --manifest-path=contracts/Cargo.toml --all-targets --all-features
```

### Phase 3: Frontend Critical (1 hour)
1. Fix ERROR-F001: React effect issue

### Phase 4: Frontend High Priority (4-6 hours)
1. Fix ISSUE-F002: Remove `any` types
2. Fix ISSUE-F003: Convert require() to imports
3. Fix ISSUE-F004: Remove unused variables
4. Fix ISSUE-F005: Fix hook dependencies

### Phase 5: Cleanup (2-3 hours)
1. Fix ISSUE-F006: Remove unused functions
2. Fix ISSUE-C001: Profile configuration
3. Fix ISSUE-C002: Unreachable patterns

---

## 📋 VERIFICATION CHECKLIST

### Backend
- [ ] `cargo check --manifest-path=backend/Cargo.toml` passes
- [ ] `cargo clippy --manifest-path=backend/Cargo.toml --all-targets --all-features` returns 0 errors
- [ ] `cargo test --manifest-path=backend/Cargo.toml --all-features` passes
- [ ] `cargo fmt --manifest-path=backend/Cargo.toml --check` passes

### Contracts
- [ ] `cargo check --manifest-path=contracts/Cargo.toml` passes
- [ ] `cargo clippy --manifest-path=contracts/Cargo.toml --all-targets --all-features` returns 0 errors
- [ ] `cargo test --manifest-path=contracts/Cargo.toml --all-features` passes
- [ ] `cargo fmt --manifest-path=contracts/Cargo.toml --check` passes

### Frontend
- [ ] `npm run lint` returns 0 errors
- [ ] `npm run type-check` passes
- [ ] `npm test` passes
- [ ] `npm run build` succeeds

---

**NEXT STEPS:**
1. Fix all compilation errors (Phase 1)
2. Re-run scans to get complete issue list
3. Create GitHub issues for each problem
4. Prioritize and assign to team members

**NOTE:** This document will need to be updated after compilation errors are fixed to include the full list of clippy warnings and code quality issues that are currently hidden by compilation failures.


---

## 🟠 ADDITIONAL HIGH PRIORITY ISSUES (20 More Issues)

### ISSUE-B006: Backend - unwrap() in Production Code (Risk of Panics)
**Priority:** High  
**Category:** Error Handling / Reliability  
**Files:** Multiple backend files

**Problem:**
Using `unwrap()` in production code can cause panics and crash the server.

**Examples:**
```rust
// backend/src/main.rs:34
let cache = Arc::new(CacheManager::new(CacheConfig::default()).await.unwrap());

// backend/src/main.rs:51
let rate_limiter = Arc::new(RateLimiter::new().await.unwrap());
```

**Fix:**
```rust
// ✅ Proper error handling
let cache = Arc::new(
    CacheManager::new(CacheConfig::default())
        .await
        .context("Failed to initialize cache manager")?
);

let rate_limiter = Arc::new(
    RateLimiter::new()
        .await
        .context("Failed to initialize rate limiter")?
);
```

**Estimated Effort:** 4-6 hours

---

### ISSUE-C003: Contracts - unwrap() in Production Contract Code
**Priority:** High  
**Category:** Error Handling / Security  
**Files:**
- `contracts/analytics/src/lib.rs:21`
- `contracts/access-control/src/lib.rs:23`

**Problem:**
Using `unwrap()` in smart contracts causes panics and unpredictable behavior.

**Current Code:**
```rust
// contracts/analytics/src/lib.rs:21
pub fn get_admin(env: Env) -> Address {
    let key = "admin";
    env.storage().instance().get(&key).unwrap()  // ❌ PANIC RISK
}
```

**Fix:**
```rust
pub fn get_admin(env: Env) -> Result<Address, Error> {
    let key = "admin";
    env.storage()
        .instance()
        .get(&key)
        .ok_or(Error::AdminNotSet)  // ✅ PROPER ERROR
}
```

**Estimated Effort:** 2-3 hours

---

### ISSUE-B007: Backend - TODO Comments Indicate Incomplete Features (10 instances)
**Priority:** High  
**Category:** Implementation / Technical Debt  

**Files Affected:**
1. `backend/src/rate_limit.rs:201` - Premium tier detection not implemented
2. `backend/src/services/alert_service.rs:86` - Alert delivery not implemented
3. `backend/src/services/anchor_monitor.rs:49-51` - Metrics calculation incomplete
4. `backend/src/services/aggregation.rs:204` - Slippage calculation missing
5. `backend/src/services/aggregation.rs:429` - Tests commented out
6. `backend/src/services/contract_listener.rs:371,381` - Alert sending not implemented
7. `backend/src/services/contract.rs:308` - Transaction signing not implemented

**Example:**
```rust
// backend/src/rate_limit.rs:201
// TODO: Implement premium tier detection from database
match client {
    ClientIdentifier::ApiKey(_) => ClientTier::Authenticated,
    _ => ClientTier::Free,  // ❌ Always returns Free
}
```

**Fix:**
```rust
// ✅ Implement database lookup
async fn get_client_tier(&self, client: &ClientIdentifier) -> ClientTier {
    match client {
        ClientIdentifier::ApiKey(key) => {
            if let Ok(Some(api_key)) = self.db.get_api_key(key).await {
                return api_key.tier;
            }
            ClientTier::Free
        }
        _ => ClientTier::Free,
    }
}
```

**Estimated Effort:** 12-16 hours (multiple features to implement)

---

### ISSUE-F007: Frontend - console.log Statements in Production (30+ instances)
**Priority:** High  
**Category:** Production Readiness / Performance  

**Files Affected:**
- `frontend/src/hooks/useChartExport.ts`
- `frontend/src/app/alerts/page.tsx` (8 instances)
- `frontend/src/app/api/network-graph/route.ts` (2 instances)
- `frontend/src/components/OnChainVerification.tsx`
- `frontend/src/components/keyboard-shortcuts/ShortcutExample.tsx` (3 instances)
- `frontend/src/components/governance/ProposalDetail.tsx` (2 instances)
- `frontend/src/lib/logger.ts` (multiple instances)
- And 10+ more files

**Problem:**
Console statements leak information and impact performance in production.

**Current Code:**
```typescript
// frontend/src/app/alerts/page.tsx:32
catch (err) {
    console.error("Failed to fetch alerts data", err);  // ❌ Direct console
}

// frontend/src/components/keyboard-shortcuts/ShortcutExample.tsx:86
handler: () => {
    console.log(`Deleting item ${selectedItem}`);  // ❌ Debug log
}
```

**Fix:**
```typescript
// ✅ Use proper logger
import { logger } from '@/lib/logger';

catch (err) {
    logger.error("Failed to fetch alerts data", err);
}

// ✅ Remove debug logs or use logger
handler: () => {
    logger.debug(`Deleting item ${selectedItem}`);
}
```

**Estimated Effort:** 3-4 hours

---

### ISSUE-F008: Frontend - Hardcoded API URLs (5 instances)
**Priority:** High  
**Category:** Configuration / Deployment  

**Files Affected:**
- `frontend/src/components/CostCalculator.tsx:44`
- `frontend/src/services/sep10Auth.ts:10`
- `backend/examples/ml_test.rs:10,26`

**Current Code:**
```typescript
// frontend/src/components/CostCalculator.tsx:44
const DEFAULT_API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";  // ❌ Hardcoded fallback

// frontend/src/services/sep10Auth.ts:10
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";  // ❌ Hardcoded
```

**Fix:**
```typescript
// ✅ Use environment variable without hardcoded fallback
const DEFAULT_API_BASE = process.env.NEXT_PUBLIC_API_URL;

if (!DEFAULT_API_BASE) {
  throw new Error('NEXT_PUBLIC_API_URL environment variable is required');
}

// Or use a config file
import { config } from '@/config';
const DEFAULT_API_BASE = config.apiUrl;
```

**Estimated Effort:** 1-2 hours

---

### ISSUE-B008: Backend - Missing Error Context in Database Operations
**Priority:** High  
**Category:** Debugging / Error Handling  

**Problem:**
Database errors don't provide context about which operation failed.

**Current Code:**
```rust
pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
    let anchor = sqlx::query_as::<_, Anchor>(
        "SELECT * FROM anchors WHERE id = $1"
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;  // ❌ NO CONTEXT
    
    Ok(anchor)
}
```

**Fix:**
```rust
pub async fn get_anchor_by_id(&self, id: Uuid) -> Result<Option<Anchor>> {
    let anchor = sqlx::query_as::<_, Anchor>(
        "SELECT * FROM anchors WHERE id = $1"
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await
    .context(format!("Failed to fetch anchor with id: {}", id))?;  // ✅ CONTEXT
    
    Ok(anchor)
}
```

**Estimated Effort:** 4-5 hours (apply to all database functions)

---

### ISSUE-F009: Frontend - Missing Error Boundaries (Multiple Components)
**Priority:** High  
**Category:** Error Handling / UX  

**Problem:**
Many components lack error boundaries, causing entire app crashes.

**Files Missing Error Boundaries:**
- `frontend/src/app/[locale]/corridors/page.tsx`
- `frontend/src/app/[locale]/anchors/page.tsx`
- `frontend/src/app/[locale]/analytics/page.tsx`
- `frontend/src/app/[locale]/governance/page.tsx`

**Fix:**
```typescript
// ✅ Add error boundary
import { ErrorBoundary } from '@/components/ErrorBoundary';

export default function CorridorsPage() {
  return (
    <ErrorBoundary fallback={<CorridorsErrorFallback />}>
      <CorridorsContent />
    </ErrorBoundary>
  );
}
```

**Estimated Effort:** 2-3 hours

---

### ISSUE-B009: Backend - No Request Timeout Configuration
**Priority:** High  
**Category:** Performance / Reliability  

**File:** `backend/src/main.rs`

**Problem:**
Requests can hang indefinitely without timeout.

**Current Code:**
```rust
let listener = tokio::net::TcpListener::bind(&addr).await?;
axum::serve(listener, app).await?;  // ❌ NO TIMEOUT
```

**Fix:**
```rust
use tower_http::timeout::TimeoutLayer;
use std::time::Duration;

let timeout_seconds = std::env::var("REQUEST_TIMEOUT_SECONDS")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(30);

let app = routes(/* ... */)
    .layer(TimeoutLayer::new(Duration::from_secs(timeout_seconds)));  // ✅ TIMEOUT

info!("Request timeout set to {} seconds", timeout_seconds);
```

**Estimated Effort:** 1-2 hours

---

### ISSUE-B010: Backend - CORS Allows All Origins (Security Risk)
**Priority:** High  
**Category:** Security  

**File:** `backend/src/main.rs:54`

**Current Code:**
```rust
let cors = CorsLayer::new()
    .allow_origin(Any)      // ❌ INSECURE
    .allow_methods(Any)     // ❌ TOO PERMISSIVE
    .allow_headers(Any);    // ❌ TOO PERMISSIVE
```

**Fix:**
```rust
use tower_http::cors::{CorsLayer, AllowOrigin};

let allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
    .unwrap_or_else(|_| "http://localhost:3000".to_string());

let origins: Vec<HeaderValue> = allowed_origins
    .split(',')
    .filter_map(|origin| origin.trim().parse().ok())
    .collect();

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::list(origins))  // ✅ WHITELIST
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .allow_credentials(true);
```

**Estimated Effort:** 1-2 hours

---

### ISSUE-F010: Frontend - Missing Loading States (Multiple Pages)
**Priority:** Medium  
**Category:** UX / Accessibility  

**Files Affected:**
- `frontend/src/app/[locale]/corridors/page.tsx`
- `frontend/src/app/[locale]/anchors/page.tsx`
- `frontend/src/app/[locale]/analytics/page.tsx`

**Problem:**
Pages don't show loading indicators during data fetching.

**Fix:**
```typescript
export default function CorridorsPage() {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState(null);

  useEffect(() => {
    fetchData().finally(() => setLoading(false));
  }, []);

  if (loading) {
    return <SkeletonLoader />;  // ✅ LOADING STATE
  }

  return <CorridorsContent data={data} />;
}
```

**Estimated Effort:** 2-3 hours

---

### ISSUE-B011: Backend - No Graceful Shutdown
**Priority:** Medium  
**Category:** Operations / Reliability  

**File:** `backend/src/main.rs:74`

**Problem:**
Server doesn't handle shutdown signals gracefully.

**Current Code:**
```rust
let listener = tokio::net::TcpListener::bind(&addr).await?;
axum::serve(listener, app).await?;  // ❌ NO GRACEFUL SHUTDOWN
```

**Fix:**
```rust
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
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
}

let listener = tokio::net::TcpListener::bind(&addr).await?;
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())  // ✅ GRACEFUL SHUTDOWN
    .await?;
```

**Estimated Effort:** 2-3 hours

---

### ISSUE-F011: Frontend - Missing Accessibility Labels (ARIA)
**Priority:** Medium  
**Category:** Accessibility / Compliance  

**Files Affected:** Multiple component files

**Problem:**
Interactive elements lack proper ARIA labels.

**Examples:**
```typescript
// ❌ Missing aria-label
<button onClick={handleClick}>
  <Icon />
</button>

// ❌ Missing role
<div onClick={handleClick}>
  Click me
</div>
```

**Fix:**
```typescript
// ✅ Proper ARIA labels
<button 
  onClick={handleClick}
  aria-label="Export chart as PNG"
>
  <Icon />
</button>

// ✅ Proper role
<button onClick={handleClick}>
  Click me
</button>
```

**Estimated Effort:** 4-6 hours

---

### ISSUE-B012: Backend - No Database Connection Pooling Configuration
**Priority:** Medium  
**Category:** Performance / Configuration  

**File:** `backend/src/main.rs:33`

**Problem:**
Database pool uses default settings, not configurable.

**Current Code:**
```rust
let pool = SqlitePool::connect(&db_url).await?;  // ❌ DEFAULTS
```

**Fix:**
```rust
use sqlx::pool::PoolOptions;

let pool = PoolOptions::new()
    .max_connections(
        std::env::var("DB_POOL_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20)
    )
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .connect(&db_url)
    .await?;
```

**Estimated Effort:** 2-3 hours

---

### ISSUE-F012: Frontend - No Input Validation on Forms
**Priority:** Medium  
**Category:** Security / UX  

**Files Affected:**
- `frontend/src/components/Sep24Flow.tsx`
- `frontend/src/components/Sep31PaymentFlow.tsx`
- `frontend/src/components/CostCalculator.tsx`

**Problem:**
Forms accept invalid input without validation.

**Current Code:**
```typescript
<input
  type="url"
  placeholder="https://api.anchor.example/sep24"
  value={customTransferServer}
  onChange={(e) => setCustomTransferServer(e.target.value)}  // ❌ NO VALIDATION
/>
```

**Fix:**
```typescript
const [error, setError] = useState('');

const validateUrl = (url: string) => {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
};

<input
  type="url"
  placeholder="https://api.anchor.example/sep24"
  value={customTransferServer}
  onChange={(e) => {
    const value = e.target.value;
    setCustomTransferServer(value);
    
    if (value && !validateUrl(value)) {
      setError('Please enter a valid URL');
    } else {
      setError('');
    }
  }}
  aria-invalid={!!error}
  aria-describedby={error ? 'url-error' : undefined}
/>
{error && <span id="url-error" className="text-red-500">{error}</span>}
```

**Estimated Effort:** 3-4 hours

---

### ISSUE-C004: Contracts - No Event Emission for Critical Operations
**Priority:** Medium  
**Category:** Observability / Audit Trail  

**Files Affected:** Multiple contract files

**Problem:**
Critical operations don't emit events for tracking.

**Example:**
```rust
pub fn set_admin(env: Env, new_admin: Address) {
    env.storage().instance().set(&DataKey::Admin, &new_admin);
    // ❌ NO EVENT EMITTED
}
```

**Fix:**
```rust
pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
    caller.require_auth();
    
    let old_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::AdminNotSet)?;
    
    env.storage().instance().set(&DataKey::Admin, &new_admin);
    
    // ✅ EMIT EVENT
    env.events().publish(
        (symbol_short!("admin"), new_admin.clone()),
        AdminChangedEvent {
            old_admin,
            new_admin,
            changed_by: caller,
            timestamp: env.ledger().timestamp(),
        },
    );
    
    Ok(())
}
```

**Estimated Effort:** 3-4 hours

---

### ISSUE-B013: Backend - No Rate Limiting on WebSocket Connections
**Priority:** Medium  
**Category:** Security / Resource Management  

**File:** `backend/src/websocket.rs`

**Problem:**
WebSocket connections have no rate limiting.

**Current Code:**
```rust
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<WsState>>,
) -> Response {
    // ❌ NO RATE LIMITING
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}
```

**Fix:**
```rust
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<WsState>>,
    State(rate_limiter): State<Arc<RateLimiter>>,
) -> Response {
    // ✅ CHECK RATE LIMIT
    const MAX_CONNECTIONS: usize = 1000;
    if state.connection_count() >= MAX_CONNECTIONS {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Server at capacity"})),
        ).into_response();
    }
    
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}
```

**Estimated Effort:** 3-4 hours

---

### ISSUE-F013: Frontend - Missing TypeScript Strict Mode
**Priority:** Medium  
**Category:** Type Safety / Code Quality  

**File:** `frontend/tsconfig.json`

**Problem:**
TypeScript strict mode not enabled, allowing unsafe code.

**Fix:**
```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "strictBindCallApply": true,
    "strictPropertyInitialization": true,
    "noImplicitThis": true,
    "alwaysStrict": true
  }
}
```

**Estimated Effort:** 6-8 hours (will reveal many type errors to fix)

---

### ISSUE-B014: Backend - No Metrics Endpoint for Monitoring
**Priority:** Medium  
**Category:** Observability / Operations  

**Problem:**
No Prometheus metrics endpoint for monitoring.

**Fix:**
```rust
use prometheus::{Encoder, TextEncoder, Registry};

// Add metrics endpoint
async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap()
}

// Add to routes
let app = Router::new()
    .route("/metrics", get(metrics_handler))
    .nest("/api", api_routes);
```

**Estimated Effort:** 4-5 hours

---

### ISSUE-F014: Frontend - No Offline Support / Service Worker
**Priority:** Medium  
**Category:** PWA / User Experience  

**Problem:**
App doesn't work offline or cache resources.

**Fix:**
```typescript
// next.config.ts
import withPWA from 'next-pwa';

const config = withPWA({
  dest: 'public',
  register: true,
  skipWaiting: true,
  disable: process.env.NODE_ENV === 'development',
});

export default config;
```

**Estimated Effort:** 6-8 hours

---

### ISSUE-C005: Contracts - No Storage TTL Management
**Priority:** Medium  
**Category:** Storage / Cost Management  

**Problem:**
Persistent storage doesn't extend TTL, data will expire.

**Fix:**
```rust
pub fn submit_snapshot(env: Env, epoch: u64, hash: BytesN<32>) -> Result<u64, Error> {
    // ... existing logic ...
    
    env.storage().persistent().set(&DataKey::Snapshots, &snapshots);
    
    // ✅ EXTEND TTL
    const LEDGERS_TO_EXTEND: u32 = 518_400; // ~30 days
    env.storage().persistent().extend_ttl(
        &DataKey::Snapshots,
        LEDGERS_TO_EXTEND,
        LEDGERS_TO_EXTEND,
    );
    
    Ok(timestamp)
}
```

**Estimated Effort:** 3-4 hours

---

### ISSUE-B015: Backend - No Health Check Endpoint Details
**Priority:** Medium  
**Category:** Operations / Monitoring  

**Problem:**
Health check is too simple, doesn't check dependencies.

**Current Code:**
```rust
pub async fn health_check() -> &'static str {
    "OK"  // ❌ TOO SIMPLE
}
```

**Fix:**
```rust
#[derive(Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub cache: ComponentHealth,
    pub rpc: ComponentHealth,
}

pub async fn health_check(
    State(db): State<Arc<Database>>,
    State(cache): State<Arc<CacheManager>>,
) -> Json<HealthStatus> {
    let db_health = check_database(&db).await;
    let cache_health = check_cache(&cache).await;
    
    let overall = if db_health.healthy && cache_health.healthy {
        "healthy"
    } else {
        "degraded"
    };
    
    Json(HealthStatus {
        status: overall.to_string(),
        timestamp: Utc::now(),
        checks: HealthChecks {
            database: db_health,
            cache: cache_health,
            rpc: check_rpc().await,
        },
    })
}
```

**Estimated Effort:** 3-4 hours

---

## 📊 UPDATED SUMMARY

### Total Issues: 31 (11 Critical + 20 Additional)

### By Area:
- **Backend:** 15 issues (2 critical compilation + 13 high/medium)
- **Contracts:** 8 issues (3 critical compilation + 5 high/medium)
- **Frontend:** 8 issues (1 critical + 7 high/medium)

### By Priority:
- **Critical:** 6 issues (compilation blockers + React effect)
- **High:** 15 issues (security, error handling, incomplete features)
- **Medium:** 10 issues (observability, UX, configuration)

### Total Estimated Effort:
- **Critical fixes:** 2-4 hours
- **High priority:** 50-70 hours
- **Medium priority:** 40-60 hours
- **TOTAL:** 92-134 hours (2.5-3.5 weeks with 1 developer)

---

**UPDATED NEXT STEPS:**
1. Fix all 6 compilation errors (Phase 1 - 2-4 hours)
2. Fix React cascading renders (Phase 1 - 30 minutes)
3. Address security issues (CORS, unwrap, TODO features) (Phase 2 - 20-30 hours)
4. Clean up code quality (console.log, unused vars, error context) (Phase 3 - 15-20 hours)
5. Add observability and monitoring (Phase 4 - 15-20 hours)
6. Improve UX and accessibility (Phase 5 - 20-30 hours)
