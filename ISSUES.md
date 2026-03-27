# Stellar Insights - Issues Tracker

**Last Updated:** 2024-01-15  
**Total Issues:** 30  
**Priority Breakdown:** Critical (8) | High (12) | Medium (7) | Low (3)

---

## 🔴 Critical Issues (Must Fix Immediately)

### ISSUE-001: React Hook setState in Effect Violations
**Priority:** Critical  
**Category:** Frontend / React Hooks  
**Files Affected:**
- `src/app/[locale]/quests/page.tsx:33`
- `src/components/AlertNotifications.tsx:28`
- `src/components/CsrfTokenProvider.tsx:21`
- `src/components/lib/notification-context.tsx:100`
- `src/contexts/ThemeContext.tsx:77`
- `src/components/ui/TimeRangeSelector.tsx:21`

**Description:**  
Multiple components are calling `setState` synchronously within `useEffect` bodies, which causes cascading renders and hurts performance. This violates React's rules of hooks and can lead to infinite render loops.

**Root Cause:**  
Direct state updates in effects without proper dependency management or async handling.

**Solution Required:**
1. Move state initialization outside effects where possible
2. Use `useLayoutEffect` for synchronous DOM updates
3. Wrap state updates in async callbacks for external system synchronization
4. Add proper dependency arrays to prevent unnecessary re-renders

**Example Fix:**
```typescript
// ❌ Bad
useEffect(() => {
  setToken(getCookie('csrf-token'));
}, []);

// ✅ Good
useEffect(() => {
  const token = getCookie('csrf-token');
  if (token) {
    setToken(token);
  }
}, []);

// ✅ Better - use useState initializer
const [token, setToken] = useState(() => getCookie('csrf-token'));
```

---

### ISSUE-002: React Hook Immutability Violations
**Priority:** Critical  
**Category:** Frontend / React Hooks  
**Files Affected:**
- `src/components/layout/notification-center.tsx:82`
- `src/hooks/useWebSocket.ts:105`

**Description:**  
Code is modifying external variables (`window.location.href`) and accessing variables before declaration within React hooks, violating React's purity requirements.

**Root Cause:**
- Direct mutation of browser globals in render phase
- Incorrect closure scoping in `useCallback` hooks

**Solution Required:**
1. Move side effects (navigation) to event handlers or effects
2. Restructure `useCallback` dependencies to avoid forward references
3. Use `useRef` for mutable values that don't trigger re-renders

**Example Fix:**
```typescript
// ❌ Bad - mutating in render
const handleAction = () => {
  window.location.href = notification.actionLink;
};

// ✅ Good - use router
const router = useRouter();
const handleAction = () => {
  router.push(notification.actionLink);
};
```

---

### ISSUE-003: React Hook Purity Violations
**Priority:** Critical  
**Category:** Frontend / React Hooks  
**Files Affected:**
- `src/components/layout/notification-center.tsx:37`

**Description:**  
Calling impure function `Date.now()` during render phase, which produces unstable results when component re-renders.

**Root Cause:**  
Time-based calculations in render without memoization.

**Solution Required:**
1. Move `Date.now()` calls to effects or event handlers
2. Use `useMemo` with proper dependencies for time-based calculations
3. Consider using a time-update effect with intervals

**Example Fix:**
```typescript
// ❌ Bad
const formatDistanceToNow = (timestamp: number) => {
  const diff = Date.now() - timestamp;
  return formatDiff(diff);
};

// ✅ Good
const [now, setNow] = useState(Date.now());
useEffect(() => {
  const interval = setInterval(() => setNow(Date.now()), 60000);
  return () => clearInterval(interval);
}, []);

const formatDistanceToNow = useMemo(() => (timestamp: number) => {
  const diff = now - timestamp;
  return formatDiff(diff);
}, [now]);
```

---

### ISSUE-004: React Compiler Memoization Preservation Failures
**Priority:** Critical  
**Category:** Frontend / React Compiler  
**Files Affected:**
- `src/components/notifications/NotificationCenter.tsx:83`
- `src/components/notifications/NotificationCenter.tsx:89`

**Description:**  
React Compiler cannot preserve existing manual memoization (`useMemo`), causing optimization failures. This happens when memoized values depend on external services not tracked in dependencies.

**Root Cause:**  
Missing `notificationService` in dependency arrays of `useMemo` hooks.

**Solution Required:**
1. Add all external dependencies to `useMemo` dependency arrays
2. Wrap service instances in `useMemo` or move to context
3. Consider removing manual memoization if React Compiler can auto-optimize

**Example Fix:**
```typescript
// ❌ Bad
const analytics = useMemo(() => 
  notificationService.generateAnalytics(notifications),
  [notifications]
);

// ✅ Good
const analytics = useMemo(() => 
  notificationService.generateAnalytics(notifications),
  [notifications, notificationService]
);
```

---

### ISSUE-005: JSX Syntax Error in EnhancedNotificationCenter
**Priority:** Critical  
**Category:** Frontend / TypeScript  
**Files Affected:**
- `src/components/notifications/EnhancedNotificationCenter.tsx:596`

**Description:**  
Parsing error due to incorrect JSX syntax when accessing component from object dynamically.

**Root Cause:**  
Missing curly braces around dynamic component reference.

**Solution Required:**  
Wrap dynamic component access in JSX expression braces.

**Status:** ✅ FIXED

---

### ISSUE-006: Backend Unused Async Functions
**Priority:** Critical  
**Category:** Backend / Rust  
**Files Affected:**
- `src/api/export.rs` (multiple functions)
- `src/services/alert_service.rs:64`
- `src/services/asset_verifier.rs:244`
- `src/services/realtime_broadcaster.rs:274`
- `src/services/verification_rewards.rs:323`
- `src/websocket.rs:128,229`

**Description:**  
21+ async functions contain no await statements, causing unnecessary async overhead and potential runtime issues.

**Root Cause:**  
Functions marked `async` but performing only synchronous operations.

**Solution Required:**
1. Remove `async` keyword from functions with no await
2. Update function signatures in trait implementations
3. Adjust callers to not await synchronous functions

**Example Fix:**
```rust
// ❌ Bad
pub async fn send_alert(&self, alert: Alert) -> Result<()> {
    error!("Alert: {:?}", alert);
    Ok(())
}

// ✅ Good
pub fn send_alert(&self, alert: Alert) -> Result<()> {
    error!("Alert: {:?}", alert);
    Ok(())
}
```

---

### ISSUE-007: Backend Dead Code Warnings
**Priority:** Critical  
**Category:** Backend / Rust  
**Files Affected:**
- `src/vault/mod.rs:50-51` (VaultSecretResponse fields)
- `src/telegram/commands.rs:11` (cache field)
- Multiple other structs with unused fields

**Description:**  
27 warnings about unused struct fields and dead code, indicating incomplete implementations or unnecessary code.

**Root Cause:**  
Structs defined for future use but not currently utilized.

**Solution Required:**
1. Remove unused fields or mark with `#[allow(dead_code)]`
2. Implement missing functionality that uses these fields
3. Add `#[cfg(test)]` for test-only fields

---

### ISSUE-008: TypeScript Explicit Any Types
**Priority:** Critical  
**Category:** Frontend / TypeScript  
**Files Affected:**
- `src/__tests__/api-client.test.ts` (7 instances)
- `src/__tests__/csrf.test.ts:10`
- `src/app/api/network-graph/route.ts:39,55`
- `src/components/OnChainVerification.tsx:61,172`
- `src/components/ExportDialog.tsx:157,187`
- `src/lib/api-client.ts` (8 instances)
- `src/services/sep10Auth.ts` (8 instances)
- `src/types/network-graph.ts` (3 instances)
- And 15+ more files

**Description:**  
60+ instances of explicit `any` types, defeating TypeScript's type safety and making code prone to runtime errors.

**Root Cause:**  
Quick implementations without proper type definitions.

**Solution Required:**
1. Define proper interfaces for all data structures
2. Use generic types where appropriate
3. Use `unknown` instead of `any` for truly dynamic data
4. Add type guards for runtime type checking

**Example Fix:**
```typescript
// ❌ Bad
const handleResponse = (data: any) => {
  return data.result;
};

// ✅ Good
interface ApiResponse<T> {
  result: T;
  status: string;
}

const handleResponse = <T>(data: ApiResponse<T>): T => {
  return data.result;
};
```

---

## 🟠 High Priority Issues (Fix Soon)

### ISSUE-009: Missing React Hook Dependencies
**Priority:** High  
**Category:** Frontend / React Hooks  
**Files Affected:**
- `src/app/[locale]/dashboard/page.tsx:85`
- `src/app/[locale]/sep6/page.tsx:150,155`
- `src/components/Sep24Flow.tsx:110,118`
- `src/components/Sep31PaymentFlow.tsx:135,140`
- `src/contexts/NotificationContext.tsx:291`
- `src/components/notifications/EnhancedToastNotification.tsx:65,75,91`

**Description:**  
Multiple `useEffect` and `useCallback` hooks are missing dependencies, causing stale closures and potential bugs.

**Root Cause:**  
Incomplete dependency arrays in hooks.

**Solution Required:**
1. Add all referenced variables to dependency arrays
2. Use `useCallback` for function dependencies
3. Consider using `useRef` for values that shouldn't trigger re-renders

---

### ISSUE-010: Unused Variables and Imports
**Priority:** High  
**Category:** Frontend / Code Quality  
**Files Affected:**
- 145+ warnings across multiple files
- Common patterns: unused imports, unused destructured variables, unused function parameters

**Description:**  
Extensive unused code throughout the frontend, indicating incomplete refactoring or over-importing.

**Root Cause:**  
Development artifacts and incomplete cleanup.

**Solution Required:**
1. Run ESLint auto-fix: `npm run lint -- --fix`
2. Remove unused imports manually
3. Prefix intentionally unused parameters with underscore: `_param`
4. Remove unused helper functions

---

### ISSUE-011: Next.js HTML Link Violations
**Priority:** High  
**Category:** Frontend / Next.js  
**Files Affected:**
- `src/components/__tests__/accessibility.a11y.test.tsx:138` (4 instances)

**Description:**  
Using `<a>` tags for internal navigation instead of Next.js `<Link>` component, causing full page reloads.

**Root Cause:**  
Incorrect component usage in tests.

**Solution Required:**
```typescript
// ❌ Bad
<a href="/">Home</a>

// ✅ Good
import Link from 'next/link';
<Link href="/">Home</Link>
```

---

### ISSUE-012: Next.js Image Optimization Warnings
**Priority:** High  
**Category:** Frontend / Next.js  
**Files Affected:**
- `src/components/__tests__/accessibility.a11y.test.tsx:169,178`
- `src/components/charts/ChartExportButton.tsx:79`

**Description:**  
Using `<img>` tags instead of Next.js `<Image>` component, resulting in slower LCP and higher bandwidth usage.

**Root Cause:**  
Not leveraging Next.js image optimization.

**Solution Required:**
```typescript
// ❌ Bad
<img src="/logo.png" />

// ✅ Good
import Image from 'next/image';
<Image src="/logo.png" alt="Logo" width={100} height={100} />
```

---

### ISSUE-013: React Unescaped Entities
**Priority:** High  
**Category:** Frontend / React  
**Files Affected:**
- `src/app/settings/gdpr/page.tsx:235` (2 instances)

**Description:**  
Unescaped quotes in JSX text causing potential rendering issues.

**Solution Required:**
```typescript
// ❌ Bad
<p>User's "data" is protected</p>

// ✅ Good
<p>User&apos;s &quot;data&quot; is protected</p>
// or
<p>{`User's "data" is protected`}</p>
```

---

### ISSUE-014: TypeScript Require Imports
**Priority:** High  
**Category:** Frontend / TypeScript  
**Files Affected:**
- `next.config.ts:6`
- `scripts/replace-console-statements.js:9,10,11`

**Description:**  
Using CommonJS `require()` in TypeScript/ES6 modules.

**Solution Required:**
```typescript
// ❌ Bad
const plugin = require('plugin-name');

// ✅ Good
import plugin from 'plugin-name';
```

---

### ISSUE-015: Backend Similar Variable Names
**Priority:** High  
**Category:** Backend / Rust  
**Files Affected:**
- `src/analytics/corridor.rs:105`
- `src/api/corridors.rs:284`

**Description:**  
Variables `asset_a_parts` and `asset_b_parts` are too similar, reducing code readability.

**Root Cause:**  
Clippy pedantic lint violation.

**Solution Required:**  
Rename to more descriptive names or add `#[allow(clippy::similar_names)]`.

**Status:** ✅ FIXED

---

### ISSUE-016: Backend Unreadable Numeric Literals
**Priority:** High  
**Category:** Backend / Rust  
**Files Affected:**
- `src/api/corridors_cached.rs` (6 instances)
- `src/api/cost_calculator.rs:58`
- `src/api/export.rs:115,282`

**Description:**  
Large numeric literals without separators are hard to read (e.g., `1500000.0` vs `1_500_000.0`).

**Solution Required:**  
Add underscores to improve readability.

**Status:** ✅ FIXED

---

### ISSUE-017: Backend Too Many Function Arguments
**Priority:** High  
**Category:** Backend / Rust  
**Files Affected:**
- Multiple functions across the codebase

**Description:**  
Functions with 8+ parameters are hard to maintain and error-prone.

**Solution Required:**
1. Group related parameters into structs
2. Use builder pattern for complex configurations
3. Consider using `impl Trait` for flexibility

**Example Fix:**
```rust
// ❌ Bad
fn process(a: i32, b: i32, c: String, d: bool, e: f64, f: Vec<u8>, g: Option<String>, h: Arc<Db>) {}

// ✅ Good
struct ProcessParams {
    a: i32,
    b: i32,
    config: ProcessConfig,
    db: Arc<Db>,
}

fn process(params: ProcessParams) {}
```

---

### ISSUE-018: Backend Cognitive Complexity
**Priority:** High  
**Category:** Backend / Rust  
**Files Affected:**
- 22+ functions with high cognitive complexity

**Description:**  
Functions with deeply nested logic, multiple branches, and complex control flow.

**Solution Required:**
1. Extract nested logic into helper functions
2. Use early returns to reduce nesting
3. Replace complex conditionals with match expressions
4. Consider state machines for complex workflows

---

### ISSUE-019: Backend Significant Drop Tightening
**Priority:** High  
**Category:** Backend / Rust  
**Files Affected:**
- 22+ instances across codebase

**Description:**  
Variables with significant drop implementations (locks, file handles) held longer than necessary.

**Solution Required:**
1. Scope variables tightly with blocks
2. Drop explicitly when done: `drop(lock);`
3. Use RAII patterns for automatic cleanup

**Example Fix:**
```rust
// ❌ Bad
let lock = mutex.lock().unwrap();
// ... 100 lines of code ...
process_data();

// ✅ Good
{
    let lock = mutex.lock().unwrap();
    let data = lock.clone();
} // lock dropped here
process_data();
```

---

### ISSUE-020: Backend Format Push String
**Priority:** High  
**Category:** Backend / Rust  
**Files Affected:**
- 28+ instances

**Description:**  
Using `format!` with `push_str` causes unnecessary allocations.

**Solution Required:**
```rust
// ❌ Bad
let mut s = String::new();
s.push_str(&format!("Value: {}", value));

// ✅ Good
use std::fmt::Write;
let mut s = String::new();
write!(s, "Value: {}", value).unwrap();
```

---

## 🟡 Medium Priority Issues (Address When Possible)

### ISSUE-021: RPC Data Flow Inconsistencies
**Priority:** Medium  
**Category:** Backend / RPC Integration  
**Files Affected:**
- `src/rpc/mod.rs`
- `src/api/corridors_cached.rs`
- `src/ingestion/`

**Description:**  
RPC data fetching is not flowing perfectly through the system. Issues include:
- Inconsistent error handling between RPC calls
- Missing retry logic in some endpoints
- Circuit breaker not applied uniformly
- Pagination not handled consistently

**Root Cause:**  
Incremental development without unified RPC client wrapper.

**Solution Required:**
1. Create unified RPC client trait with consistent error handling
2. Apply circuit breaker pattern to all RPC calls
3. Implement consistent pagination strategy
4. Add comprehensive logging for RPC operations
5. Create RPC health check endpoint

**Example Fix:**
```rust
// Create unified RPC trait
#[async_trait]
pub trait RpcClient {
    async fn fetch_with_retry<T>(&self, operation: impl Fn() -> Future<Output = Result<T>>) -> Result<T>;
}

// Apply to all RPC operations
let payments = rpc_client
    .fetch_with_retry(|| rpc_client.fetch_payments(limit, cursor))
    .await?;
```

---

### ISSUE-022: Frontend Build Performance
**Priority:** Medium  
**Category:** Frontend / Build  

**Description:**  
Frontend build times are longer than optimal due to:
- Large bundle sizes
- Unoptimized imports
- Missing code splitting
- No tree shaking for some libraries

**Solution Required:**
1. Implement dynamic imports for large components
2. Use barrel exports carefully
3. Enable SWC minification
4. Analyze bundle with `@next/bundle-analyzer`

---

### ISSUE-023: Missing Alt Text on Images
**Priority:** Medium  
**Category:** Frontend / Accessibility  
**Files Affected:**
- `src/components/charts/ChartExportButton.tsx:79`

**Description:**  
Images without alt text fail accessibility standards.

**Solution Required:**
```typescript
// ❌ Bad
<Image src="/chart.png" />

// ✅ Good
<Image src="/chart.png" alt="Corridor performance chart" />
```

---

### ISSUE-024: Backend Module Name Repetitions
**Priority:** Medium  
**Category:** Backend / Rust  

**Description:**  
Module names repeated in type names (e.g., `corridor::CorridorMetrics`).

**Solution Required:**  
Acceptable pattern in Rust, but can add `#[allow(clippy::module_name_repetitions)]` if desired.

---

### ISSUE-025: Inconsistent Error Messages
**Priority:** Medium  
**Category:** Backend / API  

**Description:**  
API error responses lack consistent structure and helpful messages.

**Solution Required:**
1. Standardize error response format
2. Add error codes for client handling
3. Include helpful debug information in development
4. Implement proper error logging

---

### ISSUE-026: Missing API Rate Limit Headers
**Priority:** Medium  
**Category:** Backend / API  

**Description:**  
Rate limit middleware doesn't return standard headers (`X-RateLimit-Remaining`, etc.).

**Solution Required:**  
Add rate limit headers to all responses for client awareness.

---

### ISSUE-027: Incomplete Test Coverage
**Priority:** Medium  
**Category:** Testing  

**Description:**  
Many components and functions lack unit tests, especially:
- React hooks
- API endpoints
- RPC client methods
- Error handling paths

**Solution Required:**
1. Add tests for critical paths
2. Achieve 80%+ coverage for core modules
3. Add integration tests for RPC flows

---

## 🟢 Low Priority Issues (Nice to Have)

### ISSUE-028: Console Statements in Production
**Priority:** Low  
**Category:** Frontend / Code Quality  

**Description:**  
Some `console.log` statements may still exist in production code.

**Solution Required:**  
Use the existing `replace-console-statements.js` script or remove manually.

---

### ISSUE-029: Outdated Dependencies
**Priority:** Low  
**Category:** Dependencies  

**Description:**  
Some dependencies may have newer versions with bug fixes and features.

**Solution Required:**
1. Run `npm outdated` and `cargo outdated`
2. Update non-breaking changes
3. Test thoroughly before updating major versions

---

### ISSUE-030: Documentation Gaps
**Priority:** Low  
**Category:** Documentation  

**Description:**  
Some complex functions lack JSDoc/rustdoc comments explaining parameters and return values.

**Solution Required:**
1. Add JSDoc to all exported functions
2. Add rustdoc to public APIs
3. Include usage examples in docs

---

## 📊 Issue Statistics

### By Category
- **Frontend React Hooks:** 6 issues
- **Frontend TypeScript:** 4 issues
- **Frontend Next.js:** 3 issues
- **Backend Rust:** 10 issues
- **RPC Integration:** 1 issue
- **Build/Performance:** 2 issues
- **Testing:** 1 issue
- **Documentation:** 1 issue
- **Code Quality:** 2 issues

### By Status
- **Fixed:** 3 issues ✅
- **In Progress:** 0 issues 🔄
- **Not Started:** 27 issues ⏳

### Estimated Effort
- **Critical (8 issues):** ~16-24 hours
- **High (12 issues):** ~20-30 hours
- **Medium (7 issues):** ~10-15 hours
- **Low (3 issues):** ~3-5 hours
- **Total:** ~49-74 hours

---

## 🎯 Recommended Fix Order

### Phase 1: Critical Stability (Week 1)
1. ISSUE-001: React Hook setState violations
2. ISSUE-002: React Hook immutability violations
3. ISSUE-003: React Hook purity violations
4. ISSUE-008: TypeScript explicit any types (high-impact files)
5. ISSUE-006: Backend unused async functions

### Phase 2: High Priority Cleanup (Week 2)
6. ISSUE-009: Missing React Hook dependencies
7. ISSUE-010: Unused variables and imports (auto-fix)
8. ISSUE-014: TypeScript require imports
9. ISSUE-018: Backend cognitive complexity (top 5 functions)
10. ISSUE-019: Backend significant drop tightening

### Phase 3: Medium Priority Improvements (Week 3)
11. ISSUE-021: RPC data flow inconsistencies
12. ISSUE-022: Frontend build performance
13. ISSUE-025: Inconsistent error messages
14. ISSUE-027: Incomplete test coverage (critical paths)

### Phase 4: Polish and Optimization (Week 4)
15. Remaining medium and low priority issues
16. Documentation improvements
17. Performance optimization
18. Final testing and validation

---

## 🔧 Quick Wins (Can Fix in < 1 Hour Each)

- ✅ ISSUE-005: JSX syntax error (FIXED)
- ✅ ISSUE-015: Similar variable names (FIXED)
- ✅ ISSUE-016: Unreadable numeric literals (FIXED)
- ISSUE-011: Next.js HTML link violations
- ISSUE-012: Next.js image optimization
- ISSUE-013: React unescaped entities
- ISSUE-014: TypeScript require imports
- ISSUE-023: Missing alt text
- ISSUE-028: Console statements

---

## 📝 Notes

- All issues are documented with file locations and line numbers
- Example fixes provided for common patterns
- Issues are prioritized based on impact to stability, performance, and user experience
- Some issues (like clippy lints) are suppressed at crate level but documented for awareness
- RPC flow issues require architectural review before implementation

**Next Steps:**
1. Review and prioritize issues with team
2. Assign issues to developers
3. Create GitHub issues for tracking
4. Set up CI/CD checks to prevent regression
5. Schedule weekly review of progress

---

**Generated:** 2024-01-15  
**Version:** 1.0  
**Maintainer:** Development Team
