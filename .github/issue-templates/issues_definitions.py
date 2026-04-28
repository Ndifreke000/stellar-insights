"""
Complete definitions for all 70 GitHub issues
Organized by priority: Critical (10), High (25), Medium (25), Low (10)
"""

ISSUES = [
    # ========================================================================
    # CRITICAL ISSUES (10) - P0 Priority
    # ========================================================================
    
    {
        "number": 1,
        "title": "🔴 CRITICAL: Missing Module metrics_cached Blocks Compilation",
        "labels": ["critical", "bug", "backend", "compilation-error", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/api/mod.rs:20",
        "estimate": "5-10 minutes",
        "description": "The backend fails to compile due to missing module declaration. File backend/src/api/mod.rs:20 references 'pub mod metrics_cached;' but the file doesn't exist.",
        "impact": "❌ Blocks all backend compilation\n❌ Prevents development and deployment\n❌ CI/CD pipeline fails",
        "solution": "**Option 1:** Create backend/src/api/metrics_cached.rs with basic implementation\n**Option 2:** Remove the module declaration from mod.rs if not needed",
        "verification": "cd backend && cargo check && cargo build"
    },
    
    {
        "number": 2,
        "title": "🔴 CRITICAL: Mismatched Closing Delimiter in database.rs",
        "labels": ["critical", "bug", "backend", "compilation-error", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/database.rs:1515",
        "estimate": "10-15 minutes",
        "description": "Syntax error with unclosed async block in execute_with_timing call around line 1504. Error: 'mismatched closing delimiter'",
        "impact": "❌ Blocks backend compilation\n❌ Prevents all development and testing",
        "solution": "1. Locate execute_with_timing call around line 1504\n2. Ensure all braces, brackets, and parentheses are properly matched\n3. Use IDE bracket matching or cargo fmt",
        "verification": "cd backend && cargo check && cargo clippy"
    },
    
    {
        "number": 3,
        "title": "🔴 CRITICAL: Unclosed Delimiter in Analytics Contract",
        "labels": ["critical", "bug", "contracts", "compilation-error", "P0"],
        "priority": "P0 - Critical",
        "area": "Contracts",
        "file": "contracts/analytics/src/lib.rs",
        "estimate": "15-20 minutes",
        "description": "Contract contains unclosed delimiter preventing compilation. Error: 'this file contains an unclosed delimiter'",
        "impact": "❌ Blocks analytics contract deployment\n❌ Prevents contract testing",
        "solution": "Review contract for unclosed braces, brackets, or parentheses using IDE bracket matching or cargo check",
        "verification": "cd contracts/analytics && cargo check"
    },
    
    {
        "number": 4,
        "title": "🔴 CRITICAL: Multiple Compilation Errors in stellar-insights Contract",
        "labels": ["critical", "bug", "contracts", "compilation-error", "P0"],
        "priority": "P0 - Critical",
        "area": "Contracts",
        "file": "contracts/stellar_insights/src/lib.rs",
        "estimate": "1-2 hours",
        "description": "18 compilation errors preventing contract build. Error: 'could not compile stellar-insights (lib) due to 18 previous errors'",
        "impact": "❌ Blocks main contract deployment\n❌ Prevents integration testing",
        "solution": "Run cargo clippy --package stellar-insights --all-targets to see detailed errors and fix systematically",
        "verification": "cd contracts && cargo clippy --package stellar-insights --all-targets"
    },
    
    {
        "number": 5,
        "title": "🔴 CRITICAL: Access Control Contract Compilation Errors",
        "labels": ["critical", "bug", "contracts", "compilation-error", "P0"],
        "priority": "P0 - Critical",
        "area": "Contracts",
        "file": "contracts/access-control/src/lib.rs",
        "estimate": "1 hour",
        "description": "9 compilation errors in access control contract. Error: 'could not compile access-control (lib) due to 9 previous errors'",
        "impact": "❌ Blocks access control functionality\n❌ Security features unavailable",
        "solution": "Run cargo clippy --package access-control --all-targets and fix errors systematically",
        "verification": "cd contracts && cargo clippy --package access-control --all-targets"
    },
    
    {
        "number": 6,
        "title": "🔴 CRITICAL: React Effect Causing Cascading Renders",
        "labels": ["critical", "bug", "frontend", "performance", "P0"],
        "priority": "P0 - Critical",
        "area": "Frontend",
        "file": "frontend/src/app/[locale]/quests/page.tsx:33",
        "estimate": "30 minutes",
        "description": "Synchronous setState within useEffect triggers cascading renders causing performance degradation",
        "impact": "❌ Performance degradation\n❌ Poor user experience\n❌ Potential infinite render loops",
        "solution": "Split into separate effects OR use useMemo to derive state OR use callback pattern with conditional updates",
        "verification": "npm run dev && check browser console for warnings"
    },
    
    {
        "number": 7,
        "title": "🔴 CRITICAL: Hardcoded JWT Secret Placeholder Not Validated",
        "labels": ["critical", "security", "backend", "configuration", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/.env.example:51",
        "estimate": "1 hour",
        "description": "JWT_SECRET placeholder 'CHANGE_ME_generate_with_openssl_rand_base64_48' is not validated on startup, allowing insecure deployments",
        "impact": "🔒 Critical security vulnerability\n🔒 Allows production deployment with default secrets\n🔒 JWT tokens can be forged",
        "solution": "Add startup validation to ensure JWT_SECRET is not the placeholder value and meets minimum length requirements (32+ characters)",
        "verification": "Start server with placeholder secret and verify it fails with clear error message"
    },
    
    {
        "number": 8,
        "title": "🔴 CRITICAL: All-Zeros Encryption Key in Example Config",
        "labels": ["critical", "security", "backend", "configuration", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/.env.example:48",
        "estimate": "1 hour",
        "description": "ENCRYPTION_KEY in .env.example is all zeros (64 zeros), no validation prevents using this in production",
        "impact": "🔒 Critical security vulnerability\n🔒 Data encrypted with known key\n🔒 Complete data exposure risk",
        "solution": "Add startup validation to ensure ENCRYPTION_KEY is not all zeros and is properly generated (32-byte hex)",
        "verification": "Start server with all-zeros key and verify it fails with security error"
    },
    
    {
        "number": 9,
        "title": "🔴 CRITICAL: CORS Allows All Origins (Security Risk)",
        "labels": ["critical", "security", "backend", "cors", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/main.rs:54",
        "estimate": "1-2 hours",
        "description": "CORS configuration uses 'Any' for origins, methods, and headers, allowing any website to make requests",
        "impact": "🔒 Security vulnerability\n🔒 CSRF attacks possible\n🔒 Data leakage to malicious sites",
        "solution": "Implement whitelist-based CORS with environment variable CORS_ALLOWED_ORIGINS, restrict methods to GET/POST/PUT/DELETE, limit headers to AUTHORIZATION and CONTENT_TYPE",
        "verification": "Test CORS with curl from different origins and verify only whitelisted origins work"
    },
    
    {
        "number": 10,
        "title": "🔴 CRITICAL: Silent .env Loading Failure",
        "labels": ["critical", "bug", "backend", "configuration", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/main.rs:40",
        "estimate": "15 minutes",
        "description": "dotenvy::dotenv() errors are silently ignored with 'let _ = ', hiding configuration loading failures",
        "impact": "❌ Configuration errors hidden\n❌ Server runs with wrong/missing config\n❌ Debugging nightmare",
        "solution": "Log .env loading status (success/failure) and warn if file not found. Consider failing fast if required variables are missing.",
        "verification": "Remove .env file and verify server logs warning about missing configuration"
    },
    
    # ========================================================================
    # HIGH PRIORITY ISSUES (25) - P1 Priority
    # ========================================================================
    
    {
        "number": 11,
        "title": "🟠 HIGH: Extensive unwrap() Usage in Backend (50+ instances)",
        "labels": ["high-priority", "bug", "backend", "error-handling", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "Multiple files",
        "estimate": "6-8 hours",
        "description": "50+ instances of .unwrap() in production code causing panic risks. Examples: main.rs:34 (cache), main.rs:51 (rate limiter), rpc/stellar.rs:492 (HTTP client)",
        "impact": "❌ Server crashes on errors\n❌ Poor error messages\n❌ No graceful degradation",
        "solution": "Replace all unwrap() with proper error handling using ? operator and context(). Priority: main.rs initialization, then RPC client, then database operations",
        "verification": "grep -r '\\.unwrap()' backend/src/ --exclude-dir=tests && cargo clippy"
    },
    
    {
        "number": 12,
        "title": "🟠 HIGH: unwrap() in Production Contract Code",
        "labels": ["high-priority", "bug", "contracts", "error-handling", "P1"],
        "priority": "P1 - High",
        "area": "Contracts",
        "file": "contracts/analytics/src/lib.rs:21, contracts/access-control/src/lib.rs:23",
        "estimate": "2-3 hours",
        "description": "unwrap() calls in smart contracts cause panics and unpredictable behavior. Example: env.storage().instance().get(&key).unwrap()",
        "impact": "❌ Contract panics\n❌ Funds locked\n❌ Unpredictable behavior",
        "solution": "Replace unwrap() with Result<T, Error> returns and proper error types. Use ok_or(Error::NotFound) pattern",
        "verification": "cd contracts && grep -r '\\.unwrap()' --exclude-dir=target"
    },
    
    {
        "number": 13,
        "title": "🟠 HIGH: TODO Comments Indicate Incomplete Features (10+ instances)",
        "labels": ["high-priority", "enhancement", "backend", "technical-debt", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "Multiple files",
        "estimate": "12-16 hours",
        "description": "10+ TODO comments for unimplemented features: premium tier detection, alert delivery, metrics calculation, slippage calculation, transaction signing",
        "impact": "❌ Features advertised but not working\n❌ User confusion\n❌ Technical debt",
        "solution": "Prioritize and implement: 1) Premium tier detection (rate_limit.rs:201), 2) Alert delivery (alert_service.rs:86), 3) Metrics calculation (anchor_monitor.rs:49-51)",
        "verification": "grep -r 'TODO' backend/src/ && verify each feature works"
    },
    
    {
        "number": 14,
        "title": "🟠 HIGH: console.log Statements in Production (50+ instances)",
        "labels": ["high-priority", "code-quality", "frontend", "production-readiness", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "Multiple files",
        "estimate": "3-4 hours",
        "description": "50+ console.log/error/warn statements leak information and impact performance. Files: app/alerts/page.tsx (8), components/OnChainVerification.tsx, lib/logger.ts",
        "impact": "❌ Information leakage\n❌ Performance impact\n❌ Unprofessional",
        "solution": "Replace all console statements with logger from lib/logger.ts. Use logger.debug() for development, logger.error() for errors",
        "verification": "grep -r 'console\\.' frontend/src/ --exclude-dir=node_modules"
    },
    
    {
        "number": 15,
        "title": "🟠 HIGH: TypeScript any Type Usage (30+ instances)",
        "labels": ["high-priority", "code-quality", "frontend", "type-safety", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "Multiple files",
        "estimate": "4-6 hours",
        "description": "30+ instances of 'any' type defeating TypeScript's type safety. Examples: useWebSocket.ts:13, app/api/network-graph/route.ts:39,55, components/OnChainVerification.tsx:61,172",
        "impact": "❌ No type safety\n❌ Runtime errors\n❌ Poor IDE support",
        "solution": "Define proper types for each usage. Example: Replace 'any' in useWebSocket with proper message interface, define anchor/corridor types in network-graph",
        "verification": "npm run type-check && grep -r ': any' frontend/src/"
    },
    
    {
        "number": 16,
        "title": "🟠 HIGH: Hardcoded API URLs in Frontend",
        "labels": ["high-priority", "configuration", "frontend", "deployment", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/components/CostCalculator.tsx:44, frontend/src/services/sep10Auth.ts:10",
        "estimate": "1-2 hours",
        "description": "Hardcoded fallback URLs like 'http://127.0.0.1:8080/api' and 'http://localhost:8080' prevent proper deployment configuration",
        "impact": "❌ Production deployment issues\n❌ Cannot configure different environments\n❌ Hardcoded localhost",
        "solution": "Remove hardcoded fallbacks, require NEXT_PUBLIC_API_URL environment variable, throw error if missing. Create config.ts with validated environment variables",
        "verification": "Unset NEXT_PUBLIC_API_URL and verify app fails with clear error"
    },
    
    {
        "number": 17,
        "title": "🟠 HIGH: Missing Error Context in Database Operations",
        "labels": ["high-priority", "enhancement", "backend", "error-handling", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/database.rs",
        "estimate": "4-5 hours",
        "description": "Database errors don't provide context about which operation failed, making debugging difficult",
        "impact": "❌ Poor debugging experience\n❌ Generic error messages\n❌ Hard to diagnose issues",
        "solution": "Add .context() to all database operations with descriptive messages including entity IDs. Example: .context(format!('Failed to fetch anchor with id: {}', id))",
        "verification": "Trigger database errors and verify error messages include operation context"
    },
    
    {
        "number": 18,
        "title": "🟠 HIGH: Missing Error Boundaries in Frontend",
        "labels": ["high-priority", "bug", "frontend", "error-handling", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "Multiple page components",
        "estimate": "2-3 hours",
        "description": "Many components lack error boundaries causing entire app crashes. Missing in: corridors/page.tsx, anchors/page.tsx, analytics/page.tsx, governance/page.tsx",
        "impact": "❌ Entire app crashes on component errors\n❌ Poor user experience\n❌ No error recovery",
        "solution": "Wrap each major page component with ErrorBoundary providing fallback UI. Create reusable ErrorBoundary component with retry functionality",
        "verification": "Trigger errors in each page and verify graceful fallback UI"
    },
    
    {
        "number": 19,
        "title": "🟠 HIGH: No Request Timeout Configuration",
        "labels": ["high-priority", "enhancement", "backend", "performance", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/main.rs",
        "estimate": "1-2 hours",
        "description": "Requests can hang indefinitely without timeout. Partially addressed in PR #816 but needs verification",
        "impact": "❌ Resource exhaustion\n❌ Hanging connections\n❌ DoS vulnerability",
        "solution": "Implement TimeoutLayer with configurable REQUEST_TIMEOUT_SECONDS (default 30s). Verify PR #816 implementation is complete",
        "verification": "Test with slow endpoint and verify timeout after configured duration"
    },
    
    {
        "number": 20,
        "title": "🟠 HIGH: No Graceful Shutdown",
        "labels": ["high-priority", "enhancement", "backend", "reliability", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/main.rs:74",
        "estimate": "2-3 hours",
        "description": "Server doesn't handle SIGTERM/SIGINT gracefully, causing abrupt termination and potential data loss",
        "impact": "❌ Abrupt termination\n❌ In-flight requests dropped\n❌ Potential data corruption",
        "solution": "Implement shutdown_signal() handler for SIGTERM/SIGINT, use axum::serve().with_graceful_shutdown(), allow in-flight requests to complete",
        "verification": "Send SIGTERM and verify server waits for in-flight requests before shutting down"
    },
    
    {
        "number": 21,
        "title": "🟠 HIGH: Missing Loading States in Frontend Pages",
        "labels": ["high-priority", "enhancement", "frontend", "ux", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "Multiple page components",
        "estimate": "2-3 hours",
        "description": "Pages don't show loading indicators during data fetching, poor UX",
        "impact": "❌ Poor user experience\n❌ Users don't know if app is working\n❌ Looks broken",
        "solution": "Add loading state to all data-fetching pages with SkeletonLoader components",
        "verification": "Navigate to each page and verify loading indicator appears"
    },
    
    {
        "number": 22,
        "title": "🟠 HIGH: No Rate Limiting on WebSocket Connections",
        "labels": ["high-priority", "security", "backend", "websocket", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/websocket.rs",
        "estimate": "3-4 hours",
        "description": "WebSocket connections have no rate limiting, DoS vulnerability",
        "impact": "🔒 DoS vulnerability\n🔒 Resource exhaustion\n🔒 Server crashes",
        "solution": "Add connection limit (MAX_CONNECTIONS=1000), implement per-IP rate limiting, return 503 when at capacity",
        "verification": "Create 1000+ connections and verify server rejects excess"
    },
    
    {
        "number": 23,
        "title": "🟠 HIGH: Missing Input Validation on Forms",
        "labels": ["high-priority", "security", "frontend", "validation", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/components/Sep24Flow.tsx, Sep31PaymentFlow.tsx, CostCalculator.tsx",
        "estimate": "3-4 hours",
        "description": "Forms accept invalid input without validation (URLs, amounts, addresses)",
        "impact": "🔒 Invalid data submitted\n🔒 Poor UX\n🔒 Backend errors",
        "solution": "Add validation for URL inputs, numeric inputs, Stellar addresses. Show error messages with aria-invalid",
        "verification": "Submit invalid data and verify validation errors appear"
    },
    
    {
        "number": 24,
        "title": "🟠 HIGH: Hardcoded Magic Numbers (20+ instances)",
        "labels": ["high-priority", "code-quality", "backend", "contracts", "configuration", "P1"],
        "priority": "P1 - High",
        "area": "Backend & Contracts",
        "file": "backend/src/rpc/stellar.rs:21-40, backend/src/auth/sep10.rs, contracts/analytics/src/lib.rs:63-67",
        "estimate": "4-6 hours",
        "description": "20+ hardcoded constants should be environment-configurable: MAX_RETRIES=3, CHALLENGE_EXPIRY_SECONDS=300, LEDGERS_TO_EXTEND=518400",
        "impact": "❌ Cannot tune for production\n❌ Hardcoded timeouts\n❌ Inflexible configuration",
        "solution": "Move all constants to environment variables with sensible defaults. Document in .env.example",
        "verification": "Change env vars and verify behavior changes accordingly"
    },
    
    {
        "number": 25,
        "title": "🟠 HIGH: Database Pool Uses Default Settings",
        "labels": ["high-priority", "enhancement", "backend", "performance", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/main.rs:33",
        "estimate": "2-3 hours",
        "description": "Database connection pool not configurable for production workloads",
        "impact": "❌ Cannot scale\n❌ Connection exhaustion\n❌ Poor performance",
        "solution": "Implement PoolOptions with configurable max_connections, min_connections, acquire_timeout, idle_timeout from environment",
        "verification": "Set DB_POOL_MAX_CONNECTIONS and verify pool respects limit"
    },
    
    {
        "number": 26,
        "title": "🟠 HIGH: Forbidden require() Imports in Frontend (4 instances)",
        "labels": ["high-priority", "code-quality", "frontend", "modernization", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/next.config.ts:6, frontend/scripts/replace-console-statements.js",
        "estimate": "30 minutes",
        "description": "4 instances of require() instead of ES6 imports",
        "impact": "❌ Not modern JavaScript\n❌ Linting errors\n❌ Inconsistent code style",
        "solution": "Replace all require() with import statements. Convert next.config.ts to use import",
        "verification": "npm run lint && verify no require() warnings"
    },
    
    {
        "number": 27,
        "title": "🟠 HIGH: Unused Variables and Imports (20+ instances)",
        "labels": ["high-priority", "code-quality", "frontend", "cleanup", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "Multiple files",
        "estimate": "2-3 hours",
        "description": "20+ unused imports and variables cluttering codebase. Examples: waitFor unused, TrendingUp/Activity/Download icons unused, MainLayout/SkeletonCorridorCard unused",
        "impact": "❌ Code clutter\n❌ Larger bundle size\n❌ Confusing for developers",
        "solution": "Remove all unused imports and variables. Use ESLint autofix where possible",
        "verification": "npm run lint && verify no unused variable warnings"
    },
    
    {
        "number": 28,
        "title": "🟠 HIGH: Missing React Hook Dependencies",
        "labels": ["high-priority", "bug", "frontend", "react", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/app/[locale]/dashboard/page.tsx:85",
        "estimate": "15 minutes",
        "description": "useCallback missing 't' dependency, potential stale closure bugs",
        "impact": "❌ Stale closures\n❌ Bugs with translations\n❌ React warnings",
        "solution": "Add missing dependencies to all hooks. Run ESLint and fix all hook dependency warnings",
        "verification": "npm run lint && verify no hook dependency warnings"
    },
    
    {
        "number": 29,
        "title": "🟠 HIGH: Webhook Dispatcher Spawned Without Error Recovery",
        "labels": ["high-priority", "bug", "backend", "reliability", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/main.rs:136-141",
        "estimate": "2-3 hours",
        "description": "Webhook dispatcher task spawned without supervision, silent failures",
        "impact": "❌ Silent webhook failures\n❌ No error visibility\n❌ No automatic recovery",
        "solution": "Add error handling to spawned task, log errors, implement retry logic, add health check for webhook dispatcher",
        "verification": "Trigger webhook error and verify it's logged and retried"
    },
    
    {
        "number": 30,
        "title": "🟠 HIGH: Missing Input Validation on API Endpoints",
        "labels": ["high-priority", "security", "backend", "validation", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/api/corridors.rs:83, backend/src/api/api_keys.rs, backend/src/api/alerts.rs",
        "estimate": "4-6 hours",
        "description": "API endpoints lack input validation and rate limiting. Pattern matching without validation, API key rotation not rate-limited, alert operations not rate-limited",
        "impact": "🔒 Security vulnerability\n🔒 Invalid data accepted\n🔒 DoS attacks possible",
        "solution": "Add input validation middleware, implement rate limiting on sensitive operations, validate all user inputs",
        "verification": "Send invalid/malicious inputs and verify proper rejection"
    },
    
    {
        "number": 31,
        "title": "🟠 HIGH: TODO - Mock Data Still in Use (Frontend Analytics)",
        "labels": ["high-priority", "enhancement", "frontend", "api-integration", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/lib/analytics.ts:78",
        "estimate": "4-6 hours",
        "description": "Analytics dashboard still uses mock data with TODO comment to replace with actual API",
        "impact": "❌ Fake data shown to users\n❌ Misleading analytics\n❌ Feature incomplete",
        "solution": "Implement actual API call to backend analytics endpoint, remove mock data generation, handle loading/error states",
        "verification": "Verify analytics page shows real data from backend"
    },
    
    {
        "number": 32,
        "title": "🟠 HIGH: TODO - Error Tracking Not Implemented",
        "labels": ["high-priority", "enhancement", "frontend", "observability", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/lib/logger.ts:92",
        "estimate": "3-4 hours",
        "description": "Error tracking service not implemented, errors only stored in sessionStorage",
        "impact": "❌ No error visibility\n❌ Cannot diagnose production issues\n❌ Poor observability",
        "solution": "Integrate error tracking service (Sentry, Rollbar, or custom endpoint), send errors to backend, implement error aggregation",
        "verification": "Trigger error and verify it appears in error tracking service"
    },
    
    {
        "number": 33,
        "title": "🟠 HIGH: Deprecated SEP-10 Module Not Removed",
        "labels": ["high-priority", "cleanup", "backend", "technical-debt", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/lib.rs:1, backend/src/auth.rs:1, backend/src/auth/sep10.rs:75-77",
        "estimate": "1-2 hours",
        "description": "Deprecated SEP-10 module commented out but not removed, causing confusion. Comment says 'not used by default'",
        "impact": "❌ Code confusion\n❌ Technical debt\n❌ Unclear which implementation to use",
        "solution": "Remove deprecated SEP-10 module entirely if sep10_simple is canonical, or document why both exist",
        "verification": "Verify authentication still works after removal"
    },
    
    {
        "number": 34,
        "title": "🟠 HIGH: Test Code in Production Directories",
        "labels": ["high-priority", "cleanup", "backend", "project-structure", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/ml_tests.rs, backend/src/services/snapshot_test.rs, backend/src/services/webhook_event_service_tests.rs",
        "estimate": "1 hour",
        "description": "Test files in src/ directory should be in tests/ directory",
        "impact": "❌ Tests compiled into production binary\n❌ Larger binary size\n❌ Wrong project structure",
        "solution": "Move all test files to tests/ directory, update imports, verify tests still run",
        "verification": "cargo test && verify binary size decreased"
    },
    
    {
        "number": 35,
        "title": "🟠 HIGH: Duplicate Function Implementation (get_event_stats_old)",
        "labels": ["high-priority", "cleanup", "backend", "code-quality", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/services/event_indexer.rs:412",
        "estimate": "30 minutes",
        "description": "get_event_stats_old() exists alongside get_event_stats(), unclear which is current",
        "impact": "❌ Code duplication\n❌ Confusion\n❌ Maintenance burden",
        "solution": "Remove get_event_stats_old() if get_event_stats() is current, or rename/document if both needed",
        "verification": "Verify event stats API still works after removal"
    },
    
    # ========================================================================
    # MEDIUM PRIORITY ISSUES (25) - P2 Priority
    # ========================================================================
    
    {
        "number": 36,
        "title": "🟡 MEDIUM: Contract Profile Configuration Warnings (3 instances)",
        "labels": ["medium-priority", "contracts", "configuration", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "contracts/access-control/Cargo.toml, contracts/stellar_insights/Cargo.toml, contracts/governance/Cargo.toml",
        "estimate": "15 minutes",
        "description": "Warning: profiles for non-root package will be ignored. Profile sections in individual contracts ignored",
        "impact": "⚠️ Confusing warnings\n⚠️ Profiles not applied\n⚠️ Build configuration unclear",
        "solution": "Remove [profile.*] sections from individual contract Cargo.toml files, keep only in workspace root",
        "verification": "cargo build && verify no profile warnings"
    },
    
    {
        "number": 37,
        "title": "🟡 MEDIUM: Unreachable Patterns in Contracts (14 warnings)",
        "labels": ["medium-priority", "contracts", "code-quality", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "contracts/stellar_insights/src/lib.rs",
        "estimate": "1-2 hours",
        "description": "14 unreachable pattern warnings in match statements",
        "impact": "⚠️ Dead code\n⚠️ Logic errors\n⚠️ Confusing code",
        "solution": "Review all match statements, remove unreachable patterns, ensure proper pattern ordering",
        "verification": "cargo clippy && verify no unreachable pattern warnings"
    },
    
    {
        "number": 38,
        "title": "🟡 MEDIUM: Unused Helper Functions (6 instances)",
        "labels": ["medium-priority", "frontend", "cleanup", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/app/[locale]/anchors/page.tsx, corridors/page.tsx, corridors/[pair]/page.tsx",
        "estimate": "1-2 hours",
        "description": "6 helper functions defined but never used: getHealthStatusColor, getHealthStatusIcon, corridorUpdates",
        "impact": "⚠️ Code clutter\n⚠️ Larger bundle\n⚠️ Confusion",
        "solution": "Either use the functions or remove them. If intended for future use, add TODO comment",
        "verification": "npm run lint && verify no unused function warnings"
    },
    
    {
        "number": 39,
        "title": "🟡 MEDIUM: Missing Accessibility Labels (ARIA)",
        "labels": ["medium-priority", "frontend", "accessibility", "a11y", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "Multiple component files",
        "estimate": "4-6 hours",
        "description": "Interactive elements lack proper ARIA labels. Buttons with only icons, divs with onClick, missing roles",
        "impact": "⚠️ Accessibility issues\n⚠️ Screen reader problems\n⚠️ WCAG non-compliance",
        "solution": "Add aria-label to all icon buttons, use proper semantic HTML (button not div), add role attributes where needed",
        "verification": "Run axe-core accessibility tests && verify no ARIA violations"
    },
    
    {
        "number": 40,
        "title": "🟡 MEDIUM: No Metrics Endpoint for Monitoring",
        "labels": ["medium-priority", "enhancement", "backend", "observability", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/main.rs",
        "estimate": "4-5 hours",
        "description": "No Prometheus metrics endpoint for monitoring",
        "impact": "⚠️ No observability\n⚠️ Cannot monitor production\n⚠️ No alerting",
        "solution": "Add /metrics endpoint with Prometheus format, expose key metrics (request count, latency, errors, DB pool, cache hits)",
        "verification": "curl /metrics && verify Prometheus format output"
    },
    
    {
        "number": 41,
        "title": "🟡 MEDIUM: Health Check Too Simple",
        "labels": ["medium-priority", "enhancement", "backend", "observability", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/main.rs",
        "estimate": "3-4 hours",
        "description": "Health check returns 'OK' without checking dependencies (database, cache, RPC)",
        "impact": "⚠️ False healthy status\n⚠️ Load balancer issues\n⚠️ Cannot detect degraded state",
        "solution": "Implement comprehensive health check with dependency checks, return JSON with status and component health",
        "verification": "Stop database and verify health check returns degraded status"
    },
    
    {
        "number": 42,
        "title": "🟡 MEDIUM: No Offline Support / Service Worker",
        "labels": ["medium-priority", "enhancement", "frontend", "pwa", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/next.config.ts",
        "estimate": "6-8 hours",
        "description": "App doesn't work offline or cache resources, no PWA capabilities",
        "impact": "⚠️ No offline support\n⚠️ Poor mobile experience\n⚠️ Not installable",
        "solution": "Add next-pwa plugin, configure service worker, implement offline fallback pages, add manifest.json",
        "verification": "Install as PWA and verify works offline"
    },
    
    {
        "number": 43,
        "title": "🟡 MEDIUM: No Storage TTL Management in Contracts",
        "labels": ["medium-priority", "contracts", "storage", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "Multiple contract files",
        "estimate": "3-4 hours",
        "description": "Persistent storage doesn't extend TTL, data will expire unexpectedly",
        "impact": "⚠️ Data loss\n⚠️ Contract failures\n⚠️ Unexpected behavior",
        "solution": "Add env.storage().persistent().extend_ttl() after all storage writes with appropriate LEDGERS_TO_EXTEND",
        "verification": "Deploy contract and verify storage TTL is extended"
    },
    
    {
        "number": 44,
        "title": "🟡 MEDIUM: No Event Emission for Critical Operations",
        "labels": ["medium-priority", "contracts", "observability", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "Multiple contract files",
        "estimate": "3-4 hours",
        "description": "Critical operations like admin changes don't emit events, no audit trail",
        "impact": "⚠️ No audit trail\n⚠️ Cannot track changes\n⚠️ Poor observability",
        "solution": "Add env.events().publish() for all critical operations (admin changes, config updates, major state changes)",
        "verification": "Perform admin change and verify event is emitted"
    },
    
    {
        "number": 45,
        "title": "🟡 MEDIUM: N+1 Query Patterns (5+ instances)",
        "labels": ["medium-priority", "performance", "backend", "database", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/services/realtime_broadcaster.rs:388-392, verification_rewards.rs:216, price_feed.rs:304-307",
        "estimate": "4-6 hours",
        "description": "Multiple N+1 query patterns causing performance issues. Iterating over results and making individual queries",
        "impact": "⚠️ Performance degradation\n⚠️ High database load\n⚠️ Slow response times",
        "solution": "Batch queries, use JOINs, implement eager loading, use window functions for leaderboard",
        "verification": "Profile queries and verify reduced query count"
    },
    
    {
        "number": 46,
        "title": "🟡 MEDIUM: Inefficient Loops (10+ instances)",
        "labels": ["medium-priority", "performance", "backend", "contracts", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend & Contracts",
        "file": "backend/src/api/export.rs, contracts/analytics/src/lib.rs",
        "estimate": "3-4 hours",
        "description": "Multiple iterations over same data for formatting, repeated iteration patterns in contracts",
        "impact": "⚠️ Performance issues\n⚠️ Wasted CPU cycles\n⚠️ Slow exports",
        "solution": "Combine iterations, use iterators efficiently, cache intermediate results",
        "verification": "Profile code and verify reduced iteration count"
    },
    
    {
        "number": 47,
        "title": "🟡 MEDIUM: Missing Caching Strategy",
        "labels": ["medium-priority", "performance", "backend", "caching", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/services/price_feed.rs, backend/src/api/anchors.rs",
        "estimate": "4-6 hours",
        "description": "Price feed and anchor metrics fetched without caching, causing repeated expensive operations",
        "impact": "⚠️ Performance issues\n⚠️ High RPC usage\n⚠️ Slow responses",
        "solution": "Implement caching layer for price feed (5min TTL), anchor metrics (1min TTL), use existing CacheManager",
        "verification": "Monitor cache hit rate and verify reduced RPC calls"
    },
    
    {
        "number": 48,
        "title": "🟡 MEDIUM: Memory Inefficiency in SQL Building",
        "labels": ["medium-priority", "performance", "backend", "database", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/database.rs:720-731, backend/src/services/event_indexer.rs:512",
        "estimate": "2-3 hours",
        "description": "Dynamic SQL string building with placeholders instead of parameterized queries, string formatting on every query",
        "impact": "⚠️ Memory allocations\n⚠️ Performance overhead\n⚠️ SQL injection risk",
        "solution": "Use parameterized queries, prepare statements once, avoid string formatting in hot paths",
        "verification": "Profile memory usage and verify reduced allocations"
    },
    
    {
        "number": 49,
        "title": "🟡 MEDIUM: Missing Test Coverage",
        "labels": ["medium-priority", "testing", "backend", "quality", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/cache_invalidation.rs, backend/src/backup.rs, backend/src/vault/**",
        "estimate": "6-8 hours",
        "description": "Critical modules have no tests: cache invalidation, backup functionality, vault integration",
        "impact": "⚠️ No test coverage\n⚠️ Regression risk\n⚠️ Hard to refactor",
        "solution": "Add unit tests for cache invalidation logic, integration tests for backup, mock tests for vault",
        "verification": "cargo test && verify coverage increased"
    },
    
    {
        "number": 50,
        "title": "🟡 MEDIUM: Flaky Tests",
        "labels": ["medium-priority", "testing", "backend", "reliability", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/tests/rpc_resilience_test.rs, backend/tests/websocket_integration_test.rs",
        "estimate": "2-3 hours",
        "description": "Tests have timing issues with AtomicUsize and WebSocket connections, occasionally fail",
        "impact": "⚠️ CI failures\n⚠️ Developer frustration\n⚠️ Reduced confidence",
        "solution": "Add proper synchronization, use tokio::time::pause() for time-based tests, increase timeouts where appropriate",
        "verification": "Run tests 100 times and verify no failures"
    },
    
    {
        "number": 51,
        "title": "🟡 MEDIUM: Missing Documentation",
        "labels": ["medium-priority", "documentation", "backend", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/services/event_indexer.rs, backend/src/rpc/circuit_breaker.rs, backend/src/vault/**",
        "estimate": "3-4 hours",
        "description": "EventOrderBy enum, circuit breaker configuration, vault module lack documentation",
        "impact": "⚠️ Hard to understand\n⚠️ Onboarding difficulty\n⚠️ Maintenance issues",
        "solution": "Add rustdoc comments to all public APIs, document configuration options, add usage examples",
        "verification": "cargo doc && verify documentation is complete"
    },
    
    {
        "number": 52,
        "title": "🟡 MEDIUM: Unclear Naming Conventions",
        "labels": ["medium-priority", "code-quality", "backend", "naming", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/models/corridor.rs, backend/src/services/event_indexer.rs",
        "estimate": "2-3 hours",
        "description": "Inconsistent naming: PaymentRecord vs CorridorTransaction, get_event_stats vs get_event_stats_old",
        "impact": "⚠️ Confusion\n⚠️ Inconsistent codebase\n⚠️ Hard to navigate",
        "solution": "Standardize naming conventions, rename inconsistent types/functions, update documentation",
        "verification": "Review codebase and verify consistent naming"
    },
    
    {
        "number": 53,
        "title": "🟡 MEDIUM: Missing Environment Variable Validation",
        "labels": ["medium-priority", "configuration", "backend", "validation", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/main.rs:189-192, backend/src/rpc/config.rs, backend/src/database.rs:233-237",
        "estimate": "2-3 hours",
        "description": "Environment variables parsed without range validation. REQUEST_TIMEOUT_SECONDS, SLOW_QUERY_THRESHOLD_MS not validated",
        "impact": "⚠️ Invalid configuration accepted\n⚠️ Runtime errors\n⚠️ Unexpected behavior",
        "solution": "Add validation for all numeric configs (min/max ranges), fail fast on invalid values with clear error messages",
        "verification": "Set invalid values and verify server fails with clear error"
    },
    
    {
        "number": 54,
        "title": "🟡 MEDIUM: Inconsistent Configuration (ELK Duplication)",
        "labels": ["medium-priority", "configuration", "backend", "cleanup", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/.env.example",
        "estimate": "15 minutes",
        "description": "ELK Stack configuration duplicated 6 times in .env.example (lines 200-350)",
        "impact": "⚠️ Confusing configuration\n⚠️ Maintenance burden\n⚠️ Copy-paste errors",
        "solution": "Remove duplicate ELK configuration sections, keep only one canonical section",
        "verification": "Review .env.example and verify no duplication"
    },
    
    {
        "number": 55,
        "title": "🟡 MEDIUM: Missing Configuration Documentation",
        "labels": ["medium-priority", "documentation", "backend", "configuration", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/rpc/rate_limiter.rs:34-50, backend/src/cache/helpers.rs",
        "estimate": "2-3 hours",
        "description": "Rate limiter and cache configuration loaded from env but not documented",
        "impact": "⚠️ Unknown configuration options\n⚠️ Hard to tune\n⚠️ Poor documentation",
        "solution": "Document all configuration options in .env.example with descriptions and defaults",
        "verification": "Review .env.example and verify all options documented"
    },
    
    {
        "number": 56,
        "title": "🟡 MEDIUM: Tight Coupling in Architecture",
        "labels": ["medium-priority", "architecture", "backend", "refactoring", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/main.rs:59-119, backend/src/services/realtime_broadcaster.rs",
        "estimate": "8-12 hours",
        "description": "All services tightly coupled in main function, realtime broadcaster depends on multiple services without dependency injection",
        "impact": "⚠️ Hard to test\n⚠️ Difficult to refactor\n⚠️ Poor modularity",
        "solution": "Implement dependency injection pattern, create service builder, use trait objects for dependencies",
        "verification": "Write unit tests for services in isolation"
    },
    
    {
        "number": 57,
        "title": "🟡 MEDIUM: God Object - database.rs (1781 lines)",
        "labels": ["medium-priority", "architecture", "backend", "refactoring", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/database.rs",
        "estimate": "12-16 hours",
        "description": "database.rs is 1781 lines handling anchors, assets, corridors, metrics, API keys - too many responsibilities",
        "impact": "⚠️ Hard to maintain\n⚠️ Difficult to navigate\n⚠️ Merge conflicts",
        "solution": "Split into modules: db/anchors.rs, db/assets.rs, db/corridors.rs, db/metrics.rs, db/api_keys.rs",
        "verification": "Verify all tests pass after refactoring"
    },
    
    {
        "number": 58,
        "title": "🟡 MEDIUM: Missing Abstraction in RPC Client",
        "labels": ["medium-priority", "architecture", "backend", "refactoring", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/rpc/stellar.rs",
        "estimate": "4-6 hours",
        "description": "RPC client directly handles mock data and real requests without abstraction layer",
        "impact": "⚠️ Hard to test\n⚠️ Mock logic mixed with real\n⚠️ Poor separation of concerns",
        "solution": "Create trait for RPC operations, implement MockRpcClient and RealRpcClient, use trait objects",
        "verification": "Write tests using MockRpcClient"
    },
    
    {
        "number": 59,
        "title": "🟡 MEDIUM: Missing TypeScript Strict Mode",
        "labels": ["medium-priority", "frontend", "type-safety", "configuration", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/tsconfig.json",
        "estimate": "6-8 hours",
        "description": "TypeScript strict mode not enabled, allowing unsafe code patterns",
        "impact": "⚠️ Type safety issues\n⚠️ Runtime errors\n⚠️ Poor code quality",
        "solution": "Enable strict mode in tsconfig.json, fix all resulting type errors (will reveal many issues)",
        "verification": "npm run type-check && verify strict mode enabled"
    },
    
    {
        "number": 60,
        "title": "🟡 MEDIUM: Placeholder SEP10 Public Key Not Validated",
        "labels": ["medium-priority", "security", "backend", "configuration", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/.env.example:19",
        "estimate": "1 hour",
        "description": "SEP10_SERVER_PUBLIC_KEY placeholder (GXXXX...) not validated on startup",
        "impact": "⚠️ Invalid configuration accepted\n⚠️ Authentication failures\n⚠️ Confusing errors",
        "solution": "Add startup validation to ensure SEP10_SERVER_PUBLIC_KEY is valid Stellar public key format",
        "verification": "Start with invalid key and verify clear error message"
    },
    
    # ========================================================================
    # LOW PRIORITY ISSUES (10) - P3 Priority / Technical Debt
    # ========================================================================
    
    {
        "number": 61,
        "title": "🟢 LOW: Commented-Out Code Cleanup Needed",
        "labels": ["low-priority", "cleanup", "backend", "technical-debt", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/src/lib.rs:1, backend/src/auth.rs:1, backend/src/services/realtime_broadcaster.rs:503",
        "estimate": "30 minutes",
        "description": "Multiple instances of commented-out code: // pub mod sep10;, commented test code",
        "impact": "⚠️ Code clutter\n⚠️ Confusion\n⚠️ Version control noise",
        "solution": "Remove all commented-out code (it's in git history if needed)",
        "verification": "grep -r '^\\s*//' backend/src/ && verify no commented code"
    },
    
    {
        "number": 62,
        "title": "🟢 LOW: Unused Dependencies Verification Needed",
        "labels": ["low-priority", "dependencies", "cleanup", "P3"],
        "priority": "P3 - Low",
        "area": "Backend & Frontend",
        "file": "backend/Cargo.toml, frontend/package.json",
        "estimate": "1-2 hours",
        "description": "lazy_static usage not verified in backend, @axe-core/cli not used in frontend scripts",
        "impact": "⚠️ Larger dependencies\n⚠️ Slower builds\n⚠️ Security surface",
        "solution": "Run cargo-udeps and npm-check-unused, remove unused dependencies",
        "verification": "cargo build && npm install && verify no unused warnings"
    },
    
    {
        "number": 63,
        "title": "🟢 LOW: No Version Pinning Strategy",
        "labels": ["low-priority", "dependencies", "configuration", "P3"],
        "priority": "P3 - Low",
        "area": "Backend & Frontend",
        "file": "backend/Cargo.toml, frontend/package.json",
        "estimate": "2-3 hours",
        "description": "No clear version pinning strategy. Frontend uses ^ allowing breaking changes",
        "impact": "⚠️ Unexpected breaking changes\n⚠️ Build reproducibility issues\n⚠️ CI failures",
        "solution": "Pin exact versions for production dependencies, document versioning strategy",
        "verification": "Review dependency versions and verify pinning strategy"
    },
    
    {
        "number": 64,
        "title": "🟢 LOW: Soroban SDK Version Check Needed",
        "labels": ["low-priority", "dependencies", "contracts", "P3"],
        "priority": "P3 - Low",
        "area": "Contracts",
        "file": "contracts/Cargo.toml",
        "estimate": "30 minutes",
        "description": "Soroban SDK version 21.0.0, verify if latest stable version",
        "impact": "⚠️ Missing features\n⚠️ Security updates\n⚠️ Bug fixes",
        "solution": "Check latest Soroban SDK version, update if newer stable available, test contracts",
        "verification": "cargo update && cargo test"
    },
    
    {
        "number": 65,
        "title": "🟢 LOW: DOMPurify Version Security Check",
        "labels": ["low-priority", "security", "frontend", "dependencies", "P3"],
        "priority": "P3 - Low",
        "area": "Frontend",
        "file": "frontend/package.json",
        "estimate": "15 minutes",
        "description": "DOMPurify version 3.3.1, verify no known CVEs",
        "impact": "⚠️ Potential XSS vulnerabilities\n⚠️ Security risk",
        "solution": "Check npm audit and CVE databases, update if vulnerabilities found",
        "verification": "npm audit && verify no DOMPurify vulnerabilities"
    },
    
    {
        "number": 66,
        "title": "🟢 LOW: OpenTelemetry Version Check",
        "labels": ["low-priority", "dependencies", "backend", "observability", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/Cargo.toml",
        "estimate": "30 minutes",
        "description": "OpenTelemetry 0.20, check for security updates and new features",
        "impact": "⚠️ Missing features\n⚠️ Potential security issues",
        "solution": "Check latest OpenTelemetry version, update if significant improvements, test tracing",
        "verification": "cargo update && verify tracing still works"
    },
    
    {
        "number": 67,
        "title": "🟢 LOW: Circular Dependencies Risk",
        "labels": ["low-priority", "architecture", "backend", "code-quality", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/src/database.rs, backend/src/services/analytics.rs",
        "estimate": "2-3 hours",
        "description": "database.rs imports from service modules, services import from database - circular dependency risk",
        "impact": "⚠️ Compilation issues\n⚠️ Hard to refactor\n⚠️ Poor architecture",
        "solution": "Refactor to remove circular dependencies, use dependency injection, create clear module hierarchy",
        "verification": "cargo build && verify no circular dependency warnings"
    },
    
    {
        "number": 68,
        "title": "🟢 LOW: Missing Transaction Support in Database",
        "labels": ["low-priority", "enhancement", "backend", "database", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/src/database.rs",
        "estimate": "4-6 hours",
        "description": "Multi-step database operations not atomic, no transaction support",
        "impact": "⚠️ Data consistency issues\n⚠️ Race conditions\n⚠️ Partial updates",
        "solution": "Add transaction support using sqlx::Transaction, wrap multi-step operations in transactions",
        "verification": "Test multi-step operations and verify atomicity"
    },
    
    {
        "number": 69,
        "title": "🟢 LOW: No Code Splitting Strategy",
        "labels": ["low-priority", "performance", "frontend", "optimization", "P3"],
        "priority": "P3 - Low",
        "area": "Frontend",
        "file": "frontend/next.config.ts",
        "estimate": "3-4 hours",
        "description": "No code splitting strategy, large bundle sizes",
        "impact": "⚠️ Slow initial load\n⚠️ Large bundles\n⚠️ Poor performance",
        "solution": "Implement dynamic imports for heavy components, use Next.js dynamic() for code splitting, analyze bundle with webpack-bundle-analyzer",
        "verification": "npm run build && analyze bundle sizes"
    },
    
    {
        "number": 70,
        "title": "🟢 LOW: Distributed Tracing Context Propagation",
        "labels": ["low-priority", "enhancement", "backend", "observability", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/src/observability/tracing.rs",
        "estimate": "2-3 hours",
        "description": "Distributed tracing partially implemented (PR #728) but context propagation needs verification",
        "impact": "⚠️ Incomplete tracing\n⚠️ Lost trace context\n⚠️ Poor observability",
        "solution": "Verify trace context propagates across service boundaries, add trace IDs to logs, test with Jaeger",
        "verification": "Make request and verify complete trace in Jaeger UI"
    }
]
