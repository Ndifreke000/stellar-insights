"""
55 NEW GitHub Issues for Stellar Insights
Based on comprehensive codebase analysis
Organized by priority: Critical (5), High (20), Medium (20), Low (10)
"""

NEW_ISSUES = [
    # ========================================================================
    # CRITICAL ISSUES (5) - P0 Priority
    # ========================================================================
    
    {
        "number": 71,
        "title": "🔴 CRITICAL: No Input Sanitization on API Endpoints",
        "labels": ["critical", "security", "backend", "validation", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/api/corridors.rs, backend/src/api/anchors.rs, backend/src/api/webhooks.rs",
        "estimate": "8-12 hours",
        "description": """API endpoints accept raw user input without sanitization or validation:
- Corridor pair parameters not validated for SQL injection
- Webhook URLs not validated (SSRF vulnerability)
- Asset codes/issuers not validated for XSS
- JSON payloads not size-limited (DoS risk)
- No regex validation on string inputs""",
        "impact": """🔒 SQL injection vulnerability
🔒 SSRF attacks via webhook URLs
🔒 XSS via asset codes
🔒 DoS via large payloads
🔒 Data corruption from invalid inputs""",
        "solution": """1. Create input validation middleware using validator crate
2. Add regex patterns for asset codes (^[A-Z0-9]{1,12}$)
3. Validate Stellar addresses (G[A-Z2-7]{55})
4. Whitelist webhook URL schemes (https only)
5. Add max payload size limit (10MB)
6. Sanitize all string inputs before database queries""",
        "verification": """# Test SQL injection
curl -X POST http://localhost:8080/api/corridors -d '{"pair":"USDC'; DROP TABLE anchors;--"}'

# Test SSRF
curl -X POST http://localhost:8080/api/webhooks -d '{"url":"http://169.254.169.254/metadata"}'

# Verify proper rejection with 400 Bad Request"""
    },
    
    {
        "number": 72,
        "title": "🔴 CRITICAL: Database Connection Pool Exhaustion Risk",
        "labels": ["critical", "bug", "backend", "database", "performance", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/database.rs, backend/src/main.rs:33",
        "estimate": "4-6 hours",
        "description": """Database connection pool can be exhausted causing cascading failures:
- Default pool size (10) too small for production
- No connection timeout handling
- Long-running queries hold connections
- No connection leak detection
- Background jobs compete with API requests for connections""",
        "impact": """❌ Service outages under load
❌ Cascading failures
❌ Requests timeout waiting for connections
❌ Cannot scale horizontally
❌ Poor production reliability""",
        "solution": """1. Increase default pool size to 50 (DB_POOL_MAX_CONNECTIONS=50)
2. Implement connection timeout (30s)
3. Add connection leak detection and logging
4. Create separate pool for background jobs
5. Add metrics for pool utilization
6. Implement circuit breaker for database operations""",
        "verification": """# Load test with 100 concurrent requests
ab -n 1000 -c 100 http://localhost:8080/api/corridors

# Monitor pool metrics
curl http://localhost:8080/metrics | grep db_pool

# Verify no connection exhaustion errors"""
    },
    
    {
        "number": 73,
        "title": "🔴 CRITICAL: Redis Connection Failures Not Handled",
        "labels": ["critical", "bug", "backend", "cache", "reliability", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/cache/manager.rs, backend/src/main.rs:34",
        "estimate": "6-8 hours",
        "description": """Redis connection failures cause server crashes:
- No fallback when Redis unavailable
- Cache operations use unwrap() causing panics
- No circuit breaker for Redis operations
- Connection pool not configured
- No retry logic for transient failures""",
        "impact": """❌ Server crashes when Redis down
❌ No graceful degradation
❌ Cannot deploy without Redis
❌ Poor production reliability
❌ Cascading failures""",
        "solution": """1. Implement fallback to in-memory cache when Redis unavailable
2. Replace all unwrap() with proper error handling
3. Add circuit breaker for Redis operations
4. Configure connection pool with retry logic
5. Add health check for Redis connectivity
6. Log cache misses and fallback usage""",
        "verification": """# Stop Redis
docker stop redis

# Verify server continues running
curl http://localhost:8080/health

# Verify fallback to in-memory cache
curl http://localhost:8080/api/corridors

# Check logs for fallback messages"""
    },
    
    {
        "number": 74,
        "title": "🔴 CRITICAL: No Rate Limiting on Expensive Operations",
        "labels": ["critical", "security", "backend", "rate-limiting", "P0"],
        "priority": "P0 - Critical",
        "area": "Backend",
        "file": "backend/src/api/export.rs, backend/src/api/analytics.rs, backend/src/api/rpc.rs",
        "estimate": "6-8 hours",
        "description": """Expensive operations lack rate limiting allowing DoS:
- CSV/PDF export endpoints unlimited
- Analytics aggregation queries unlimited
- RPC proxy endpoints unlimited
- Contract event indexing unlimited
- No per-user quotas""",
        "impact": """🔒 DoS vulnerability
🔒 Resource exhaustion
🔒 Database overload
🔒 Server crashes
🔒 Cost explosion (cloud resources)""",
        "solution": """1. Add rate limiting to export endpoints (5 requests/minute)
2. Rate limit analytics queries (20 requests/minute)
3. Rate limit RPC proxy (100 requests/minute)
4. Implement per-user quotas with API keys
5. Add queue for expensive operations
6. Return 429 Too Many Requests with Retry-After header""",
        "verification": """# Test export rate limiting
for i in {1..10}; do curl http://localhost:8080/api/export/csv; done

# Verify 429 after 5 requests
# Verify Retry-After header present"""
    },
    
    {
        "number": 75,
        "title": "🔴 CRITICAL: Smart Contract Upgrade Mechanism Missing",
        "labels": ["critical", "enhancement", "contracts", "upgradability", "P0"],
        "priority": "P0 - Critical",
        "area": "Contracts",
        "file": "contracts/analytics/src/lib.rs, contracts/governance/src/lib.rs",
        "estimate": "16-24 hours",
        "description": """Smart contracts lack upgrade mechanism:
- No contract versioning
- No migration path for data
- Cannot fix bugs without redeployment
- No governance-controlled upgrades
- Risk of locked funds""",
        "impact": """❌ Cannot fix critical bugs
❌ Data migration impossible
❌ Funds could be locked
❌ No upgrade path
❌ Redeployment loses state""",
        "solution": """1. Implement proxy pattern for upgradability
2. Add contract version storage
3. Create migration functions for data
4. Integrate with governance for upgrade approval
5. Add timelock for upgrades (48 hours)
6. Implement emergency upgrade mechanism
7. Add upgrade event emission""",
        "verification": """# Deploy v1 contract
stellar contract deploy --wasm analytics_v1.wasm

# Store data in v1
stellar contract invoke --id $CONTRACT_ID -- store_snapshot

# Deploy v2 and migrate
stellar contract invoke --id $CONTRACT_ID -- upgrade --new-wasm-hash $V2_HASH

# Verify data migrated correctly"""
    },
    
    # ========================================================================
    # HIGH PRIORITY ISSUES (20) - P1 Priority
    # ========================================================================
    
    {
        "number": 76,
        "title": "🟠 HIGH: No Monitoring Alerts Configuration",
        "labels": ["high-priority", "enhancement", "backend", "observability", "monitoring", "P1"],
        "priority": "P1 - High",
        "area": "Backend & Infrastructure",
        "file": "backend/src/observability/, k8s/monitoring/",
        "estimate": "8-12 hours",
        "description": """No alerting configured for critical metrics:
- No alerts for high error rates
- No alerts for slow queries
- No alerts for connection pool exhaustion
- No alerts for cache failures
- No PagerDuty/Slack integration""",
        "impact": """❌ Cannot detect production issues
❌ No proactive monitoring
❌ Incidents discovered by users
❌ Long MTTR (Mean Time To Recovery)
❌ Poor operational visibility""",
        "solution": """1. Configure Prometheus alerting rules
2. Set up AlertManager with routing
3. Integrate with Slack/PagerDuty
4. Create runbooks for each alert
5. Define SLOs and alert thresholds
6. Add alert for error rate > 5%
7. Add alert for p99 latency > 1s
8. Add alert for DB pool utilization > 80%""",
        "verification": """# Trigger high error rate
# Verify Slack notification received
# Verify alert shows in AlertManager UI
# Verify runbook link in alert"""
    },
    
    {
        "number": 77,
        "title": "🟠 HIGH: Frontend Bundle Size Not Optimized",
        "labels": ["high-priority", "performance", "frontend", "optimization", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/next.config.ts, frontend/package.json",
        "estimate": "6-8 hours",
        "description": """Frontend bundle size not analyzed or optimized:
- No bundle analyzer configured
- Heavy dependencies (recharts, d3-force-3d) not code-split
- No tree-shaking verification
- Images not optimized
- No lazy loading for routes""",
        "impact": """❌ Slow initial page load
❌ Poor mobile experience
❌ High bandwidth usage
❌ Poor Lighthouse scores
❌ SEO impact""",
        "solution": """1. Add @next/bundle-analyzer
2. Implement dynamic imports for heavy components
3. Lazy load chart libraries
4. Optimize images with next/image
5. Enable tree-shaking for unused exports
6. Split vendor bundles
7. Target bundle size < 200KB initial load""",
        "verification": """npm run build
npm run analyze

# Verify main bundle < 200KB
# Verify code splitting working
# Verify Lighthouse score > 90"""
    },
    
    {
        "number": 78,
        "title": "🟠 HIGH: No Database Migration Rollback Strategy",
        "labels": ["high-priority", "database", "backend", "deployment", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/migrations/, backend/src/database.rs",
        "estimate": "4-6 hours",
        "description": """Database migrations lack rollback capability:
- No down migrations defined
- Cannot rollback failed deployments
- No migration testing strategy
- No backup before migration
- Risk of data loss""",
        "impact": """❌ Cannot rollback deployments
❌ Risk of data loss
❌ Downtime during failed migrations
❌ Manual recovery required
❌ Production incidents""",
        "solution": """1. Create down migrations for all existing migrations
2. Add pre-migration backup step
3. Test migrations on staging data
4. Add migration dry-run mode
5. Document rollback procedures
6. Add migration health checks""",
        "verification": """# Run migration
sqlx migrate run

# Rollback migration
sqlx migrate revert

# Verify data integrity
# Verify application still works"""
    },
    
    {
        "number": 79,
        "title": "🟠 HIGH: API Response Times Not Tracked",
        "labels": ["high-priority", "observability", "backend", "performance", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/observability/metrics.rs, backend/src/main.rs",
        "estimate": "4-6 hours",
        "description": """No metrics for API response times:
- No histogram metrics for latency
- Cannot identify slow endpoints
- No p50/p95/p99 tracking
- No SLO monitoring
- Cannot detect performance regressions""",
        "impact": """❌ Cannot identify performance issues
❌ No SLO tracking
❌ Cannot optimize slow endpoints
❌ Poor observability
❌ User complaints without data""",
        "solution": """1. Add histogram metrics for all endpoints
2. Track p50, p95, p99 latencies
3. Add endpoint-specific metrics
4. Create Grafana dashboard for latencies
5. Set SLO targets (p95 < 500ms)
6. Add alerts for SLO violations""",
        "verification": """curl http://localhost:8080/metrics | grep http_request_duration

# Verify histogram buckets present
# Verify per-endpoint metrics
# Check Grafana dashboard"""
    },
    
    {
        "number": 80,
        "title": "🟠 HIGH: No Secrets Management Solution",
        "labels": ["high-priority", "security", "backend", "infrastructure", "P1"],
        "priority": "P1 - High",
        "area": "Backend & Infrastructure",
        "file": "backend/.env.example, k8s/secrets/",
        "estimate": "8-12 hours",
        "description": """Secrets stored in environment variables without proper management:
- No HashiCorp Vault integration
- No AWS Secrets Manager
- Secrets in plain text .env files
- No secret rotation
- No audit trail for secret access""",
        "impact": """🔒 Security vulnerability
🔒 Secrets in version control risk
🔒 No secret rotation
🔒 No audit trail
🔒 Compliance issues""",
        "solution": """1. Integrate HashiCorp Vault or AWS Secrets Manager
2. Remove secrets from .env files
3. Implement secret rotation (90 days)
4. Add audit logging for secret access
5. Use Kubernetes secrets in production
6. Encrypt secrets at rest""",
        "verification": """# Verify secrets loaded from Vault
curl http://localhost:8080/health

# Check audit logs
vault audit list

# Verify no secrets in logs"""
    },
    
    {
        "number": 81,
        "title": "🟠 HIGH: Frontend Error Tracking Not Integrated",
        "labels": ["high-priority", "observability", "frontend", "error-tracking", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/lib/logger.ts, frontend/next.config.ts",
        "estimate": "4-6 hours",
        "description": """Sentry configured but not fully integrated:
- Errors only logged to sessionStorage
- No source maps uploaded
- No release tracking
- No user context attached
- No breadcrumbs for debugging""",
        "impact": """❌ Cannot diagnose production errors
❌ No stack traces
❌ Cannot reproduce issues
❌ Poor error visibility
❌ Long debugging time""",
        "solution": """1. Complete Sentry integration
2. Upload source maps on build
3. Add release tracking
4. Attach user context to errors
5. Add breadcrumbs for user actions
6. Configure error sampling (100% for now)
7. Set up error alerts""",
        "verification": """# Trigger error in production
throw new Error('Test error')

# Verify error in Sentry dashboard
# Verify source maps working
# Verify user context present"""
    },
    
    {
        "number": 82,
        "title": "🟠 HIGH: No API Versioning Strategy Documented",
        "labels": ["high-priority", "api", "backend", "documentation", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/api/mod.rs, docs/RPC.md",
        "estimate": "6-8 hours",
        "description": """API versioning exists (v1, v2) but strategy not documented:
- No deprecation policy
- No migration guide for v1 to v2
- No version negotiation
- No sunset dates
- Breaking changes not tracked""",
        "impact": """❌ Breaking changes break clients
❌ No migration path
❌ Poor API governance
❌ Client confusion
❌ Support burden""",
        "solution": """1. Document API versioning strategy
2. Define deprecation policy (6 months notice)
3. Create migration guide v1 → v2
4. Add version negotiation via Accept header
5. Track breaking changes in CHANGELOG
6. Add deprecation warnings to v1 endpoints
7. Set sunset date for v1""",
        "verification": """# Check API documentation
cat docs/API_VERSIONING.md

# Test version negotiation
curl -H "Accept: application/vnd.stellar-insights.v2+json" http://localhost:8080/api/corridors

# Verify deprecation warnings in v1"""
    },
    
    {
        "number": 83,
        "title": "🟠 HIGH: No Load Testing or Performance Baselines",
        "labels": ["high-priority", "testing", "performance", "backend", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/load-tests/, backend/benches/",
        "estimate": "12-16 hours",
        "description": """No load testing or performance baselines established:
- No load test scenarios
- No performance benchmarks
- No capacity planning data
- Cannot detect performance regressions
- No stress testing""",
        "impact": """❌ Unknown system capacity
❌ Cannot detect regressions
❌ No capacity planning
❌ Production surprises
❌ Cannot optimize effectively""",
        "solution": """1. Create load test scenarios with k6/Gatling
2. Establish performance baselines
3. Test scenarios: 100, 500, 1000 concurrent users
4. Measure throughput (requests/sec)
5. Identify bottlenecks
6. Document capacity limits
7. Add load tests to CI/CD
8. Create performance dashboard""",
        "verification": """# Run load test
k6 run load-tests/corridors.js

# Verify metrics collected
# Check performance dashboard
# Verify no errors under load"""
    },
    
    {
        "number": 84,
        "title": "🟠 HIGH: WebSocket Connection Limits Not Enforced",
        "labels": ["high-priority", "security", "backend", "websocket", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/websocket.rs, backend/src/services/realtime_broadcaster.rs",
        "estimate": "4-6 hours",
        "description": """WebSocket connections unlimited causing resource exhaustion:
- No max connections limit
- No per-IP connection limit
- No connection timeout
- No heartbeat/ping-pong
- Memory leak risk""",
        "impact": """❌ Resource exhaustion
❌ DoS vulnerability
❌ Memory leaks
❌ Server crashes
❌ Cannot scale""",
        "solution": """1. Add max connections limit (10,000)
2. Limit connections per IP (10)
3. Implement connection timeout (5 minutes idle)
4. Add ping/pong heartbeat (30s interval)
5. Close stale connections
6. Add metrics for active connections
7. Return 503 when at capacity""",
        "verification": """# Create 100 connections
for i in {1..100}; do wscat -c ws://localhost:8080/ws & done

# Verify connection limit enforced
# Verify metrics show active connections
# Verify stale connections closed"""
    },
    
    {
        "number": 85,
        "title": "🟠 HIGH: No Disaster Recovery Plan",
        "labels": ["high-priority", "infrastructure", "backend", "disaster-recovery", "P1"],
        "priority": "P1 - High",
        "area": "Infrastructure",
        "file": "docs/, terraform/, k8s/",
        "estimate": "16-24 hours",
        "description": """No disaster recovery plan or procedures:
- No backup testing
- No restore procedures
- No RTO/RPO defined
- No failover testing
- No incident response plan""",
        "impact": """❌ Data loss risk
❌ Extended downtime
❌ No recovery procedures
❌ Compliance issues
❌ Business continuity risk""",
        "solution": """1. Define RTO (Recovery Time Objective): 4 hours
2. Define RPO (Recovery Point Objective): 1 hour
3. Document backup procedures
4. Document restore procedures
5. Test backup restoration monthly
6. Create incident response runbook
7. Set up multi-region failover
8. Test disaster recovery quarterly""",
        "verification": """# Test backup restoration
./scripts/restore-backup.sh latest

# Verify data integrity
# Verify application works
# Document time taken
# Update runbook"""
    },
    
    {
        "number": 86,
        "title": "🟠 HIGH: No API Documentation Auto-Generation",
        "labels": ["high-priority", "documentation", "backend", "api", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/api/, docs/RPC.md",
        "estimate": "8-12 hours",
        "description": """API documentation manually maintained, out of sync with code:
- utoipa configured but not complete
- Swagger UI not exposed
- Examples outdated
- Request/response schemas not documented
- No interactive API explorer""",
        "impact": """❌ Documentation drift
❌ Poor developer experience
❌ Integration difficulties
❌ Support burden
❌ Adoption friction""",
        "solution": """1. Complete utoipa annotations for all endpoints
2. Expose Swagger UI at /api/docs
3. Add request/response examples
4. Generate OpenAPI 3.0 spec
5. Add authentication examples
6. Create Postman collection
7. Auto-generate docs on build""",
        "verification": """# Access Swagger UI
open http://localhost:8080/api/docs

# Test API calls from Swagger UI
# Verify all endpoints documented
# Export OpenAPI spec"""
    },
    
    {
        "number": 87,
        "title": "🟠 HIGH: Contract Gas Optimization Needed",
        "labels": ["high-priority", "contracts", "optimization", "gas", "P1"],
        "priority": "P1 - High",
        "area": "Contracts",
        "file": "contracts/analytics/src/lib.rs, contracts/governance/src/lib.rs",
        "estimate": "12-16 hours",
        "description": """Smart contracts not optimized for gas usage:
- Repeated storage reads in loops
- Inefficient data structures
- No gas benchmarks
- Storage not minimized
- No gas profiling""",
        "impact": """❌ High transaction costs
❌ Poor user experience
❌ Adoption barrier
❌ Competitive disadvantage
❌ Wasted resources""",
        "solution": """1. Profile gas usage with soroban-cli
2. Cache storage reads in memory
3. Use efficient data structures (Map vs Vec)
4. Minimize storage writes
5. Batch operations where possible
6. Add gas benchmarks to CI
7. Target < 100k gas per transaction""",
        "verification": """# Profile gas usage
soroban contract invoke --id $CONTRACT_ID -- store_snapshot --profile

# Compare before/after optimization
# Verify gas reduction > 30%
# Add gas benchmarks to CI"""
    },
    
    {
        "number": 88,
        "title": "🟠 HIGH: No Frontend Performance Monitoring",
        "labels": ["high-priority", "observability", "frontend", "performance", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/src/lib/monitoring.ts, frontend/next.config.ts",
        "estimate": "6-8 hours",
        "description": """No real user monitoring (RUM) for frontend performance:
- No Core Web Vitals tracking
- No page load time metrics
- No API call latency tracking
- No error rate monitoring
- Cannot identify slow pages""",
        "impact": """❌ Cannot identify performance issues
❌ No user experience metrics
❌ Cannot optimize effectively
❌ Poor SEO
❌ User complaints without data""",
        "solution": """1. Integrate Web Vitals library
2. Track LCP, FID, CLS metrics
3. Send metrics to backend /api/metrics
4. Create performance dashboard
5. Set performance budgets
6. Add alerts for poor metrics
7. Track API call latencies""",
        "verification": """# Check Web Vitals
import { getCLS, getFID, getLCP } from 'web-vitals'

# Verify metrics sent to backend
curl http://localhost:8080/api/metrics/frontend

# Check performance dashboard"""
    },
    
    {
        "number": 89,
        "title": "🟠 HIGH: No Database Query Performance Monitoring",
        "labels": ["high-priority", "observability", "backend", "database", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/database.rs, backend/src/observability/",
        "estimate": "6-8 hours",
        "description": """Database queries not monitored for performance:
- Slow query logging incomplete
- No query execution plan analysis
- No index usage tracking
- Cannot identify missing indexes
- No query performance dashboard""",
        "impact": """❌ Cannot identify slow queries
❌ Cannot optimize database
❌ Performance degradation over time
❌ No capacity planning
❌ Poor observability""",
        "solution": """1. Enable SQLx query logging with execution time
2. Log queries > 100ms with EXPLAIN output
3. Track query execution plans
4. Add metrics for query duration
5. Create slow query dashboard
6. Identify missing indexes
7. Add query performance tests""",
        "verification": """# Check slow query logs
tail -f logs/slow_queries.log

# Verify EXPLAIN output present
# Check query performance dashboard
# Verify metrics collected"""
    },
    
    {
        "number": 90,
        "title": "🟠 HIGH: No Content Security Policy (CSP)",
        "labels": ["high-priority", "security", "frontend", "headers", "P1"],
        "priority": "P1 - High",
        "area": "Frontend",
        "file": "frontend/next.config.ts, frontend/middleware.ts",
        "estimate": "4-6 hours",
        "description": """No Content Security Policy headers configured:
- No CSP headers
- XSS vulnerability
- No script-src restrictions
- No frame-ancestors protection
- No upgrade-insecure-requests""",
        "impact": """🔒 XSS vulnerability
🔒 Clickjacking risk
🔒 Data injection attacks
🔒 No defense in depth
🔒 Compliance issues""",
        "solution": """1. Add CSP headers in next.config.ts
2. Restrict script-src to 'self' and trusted CDNs
3. Add frame-ancestors 'none'
4. Enable upgrade-insecure-requests
5. Add report-uri for violations
6. Test CSP with csp-evaluator
7. Monitor CSP violations""",
        "verification": """# Check CSP headers
curl -I http://localhost:3000

# Verify CSP header present
# Test with csp-evaluator
# Verify no console errors"""
    },
    
    {
        "number": 91,
        "title": "🟠 HIGH: No Dependency Vulnerability Scanning",
        "labels": ["high-priority", "security", "dependencies", "backend", "frontend", "P1"],
        "priority": "P1 - High",
        "area": "Backend & Frontend",
        "file": ".github/workflows/, backend/Cargo.toml, frontend/package.json",
        "estimate": "4-6 hours",
        "description": """No automated dependency vulnerability scanning:
- No cargo-audit in CI
- No npm audit in CI
- No Dependabot configured
- No SBOM generation
- No vulnerability alerts""",
        "impact": """🔒 Security vulnerabilities undetected
🔒 Supply chain attacks
🔒 Compliance issues
🔒 No vulnerability tracking
🔒 Delayed patching""",
        "solution": """1. Add cargo-audit to CI pipeline
2. Add npm audit to CI pipeline
3. Configure Dependabot for auto-updates
4. Generate SBOM with cargo-sbom
5. Set up vulnerability alerts
6. Create security policy
7. Fail CI on high/critical vulnerabilities""",
        "verification": """# Run vulnerability scan
cargo audit
npm audit

# Check Dependabot PRs
# Verify CI fails on vulnerabilities
# Check SBOM generated"""
    },
    
    {
        "number": 92,
        "title": "🟠 HIGH: No API Authentication Rate Limiting",
        "labels": ["high-priority", "security", "backend", "authentication", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/auth/sep10.rs, backend/src/api/auth.rs",
        "estimate": "4-6 hours",
        "description": """Authentication endpoints not rate limited:
- SEP-10 challenge endpoint unlimited
- Token endpoint unlimited
- Brute force attacks possible
- No account lockout
- No suspicious activity detection""",
        "impact": """🔒 Brute force vulnerability
🔒 Credential stuffing attacks
🔒 Account takeover risk
🔒 Resource exhaustion
🔒 Security incident risk""",
        "solution": """1. Rate limit challenge endpoint (10/minute per IP)
2. Rate limit token endpoint (5/minute per account)
3. Implement account lockout (5 failed attempts)
4. Add CAPTCHA after 3 failures
5. Log suspicious authentication attempts
6. Add alerts for brute force patterns
7. Implement exponential backoff""",
        "verification": """# Test rate limiting
for i in {1..20}; do curl http://localhost:8080/auth/challenge; done

# Verify 429 after 10 requests
# Test account lockout
# Verify alerts triggered"""
    },
    
    {
        "number": 93,
        "title": "🟠 HIGH: No Database Backup Verification",
        "labels": ["high-priority", "infrastructure", "backend", "backup", "P1"],
        "priority": "P1 - High",
        "area": "Backend & Infrastructure",
        "file": "backend/src/backup.rs, scripts/backup.sh",
        "estimate": "6-8 hours",
        "description": """Database backups not verified or tested:
- Backups created but never restored
- No backup integrity checks
- No automated restore testing
- Unknown if backups are usable
- No backup monitoring""",
        "impact": """❌ Backups may be corrupted
❌ Cannot recover from disaster
❌ False sense of security
❌ Data loss risk
❌ Compliance issues""",
        "solution": """1. Implement automated backup verification
2. Test restore weekly in staging
3. Add backup integrity checks (checksums)
4. Monitor backup success/failure
5. Alert on backup failures
6. Document restore procedures
7. Test point-in-time recovery""",
        "verification": """# Run backup verification
./scripts/verify-backup.sh latest

# Test restore to staging
./scripts/restore-to-staging.sh latest

# Verify data integrity
# Check monitoring alerts"""
    },
    
    {
        "number": 94,
        "title": "🟠 HIGH: No Horizontal Scaling Strategy",
        "labels": ["high-priority", "infrastructure", "backend", "scalability", "P1"],
        "priority": "P1 - High",
        "area": "Backend & Infrastructure",
        "file": "backend/src/main.rs, k8s/deployment.yaml",
        "estimate": "12-16 hours",
        "description": """Application not designed for horizontal scaling:
- In-memory state (WebSocket connections)
- No session affinity
- No distributed locking
- Background jobs not coordinated
- Cannot scale beyond single instance""",
        "impact": """❌ Cannot scale horizontally
❌ Single point of failure
❌ Limited capacity
❌ Cannot handle traffic spikes
❌ Poor availability""",
        "solution": """1. Move WebSocket state to Redis
2. Implement session affinity in load balancer
3. Use distributed locks (Redis) for background jobs
4. Coordinate jobs with leader election
5. Make all state external (Redis/DB)
6. Test with multiple instances
7. Document scaling procedures""",
        "verification": """# Deploy 3 instances
kubectl scale deployment stellar-insights --replicas=3

# Test WebSocket across instances
# Verify background jobs coordinated
# Load test with multiple instances"""
    },
    
    {
        "number": 95,
        "title": "🟠 HIGH: No API Request/Response Logging",
        "labels": ["high-priority", "observability", "backend", "logging", "P1"],
        "priority": "P1 - High",
        "area": "Backend",
        "file": "backend/src/observability/logging.rs, backend/src/main.rs",
        "estimate": "4-6 hours",
        "description": """API requests/responses not logged for debugging:
- No request ID tracking
- No request/response body logging
- Cannot reproduce issues
- No audit trail
- Poor debugging experience""",
        "impact": """❌ Cannot debug production issues
❌ No audit trail
❌ Cannot reproduce bugs
❌ Poor observability
❌ Compliance issues""",
        "solution": """1. Add request ID middleware (X-Request-ID)
2. Log all requests with method, path, status
3. Log request/response bodies (sanitized)
4. Add correlation IDs for distributed tracing
5. Implement structured logging
6. Add log sampling for high-volume endpoints
7. Integrate with ELK stack""",
        "verification": """# Make API request
curl -v http://localhost:8080/api/corridors

# Check logs for request ID
tail -f logs/api.log | grep request_id

# Verify structured logging
# Check ELK dashboard"""
    },
    
    # ========================================================================
    # MEDIUM PRIORITY ISSUES (20) - P2 Priority
    # ========================================================================
    
    {
        "number": 96,
        "title": "🟡 MEDIUM: No API Pagination Consistency",
        "labels": ["medium-priority", "api", "backend", "consistency", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/api/corridors.rs, backend/src/api/anchors.rs, backend/src/api/payments.rs",
        "estimate": "6-8 hours",
        "description": """Pagination implementation inconsistent across endpoints:
- Some use limit/offset
- Some use cursor-based
- No standard pagination format
- No total count returned
- No next/prev links""",
        "impact": """⚠️ Poor API consistency
⚠️ Client confusion
⚠️ Integration difficulties
⚠️ Poor developer experience
⚠️ Documentation complexity""",
        "solution": """1. Standardize on cursor-based pagination
2. Return consistent pagination metadata
3. Include total count (optional)
4. Add next/prev cursor links
5. Document pagination in OpenAPI
6. Add pagination helpers
7. Update all endpoints""",
        "verification": """# Test pagination
curl http://localhost:8080/api/corridors?limit=10&cursor=abc123

# Verify consistent format
# Check next/prev links
# Verify documentation"""
    },
    
    {
        "number": 97,
        "title": "🟡 MEDIUM: No Frontend State Management Strategy",
        "labels": ["medium-priority", "frontend", "architecture", "state-management", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/app/, frontend/src/contexts/",
        "estimate": "12-16 hours",
        "description": """No consistent state management approach:
- Mix of useState, useContext, props drilling
- No global state management
- Duplicate API calls
- No cache invalidation strategy
- State synchronization issues""",
        "impact": """⚠️ Code complexity
⚠️ Performance issues
⚠️ Bugs from stale state
⚠️ Poor maintainability
⚠️ Difficult to debug""",
        "solution": """1. Evaluate state management solutions (Zustand, Jotai, React Query)
2. Implement React Query for server state
3. Use Zustand for client state
4. Define state management patterns
5. Refactor components to use new patterns
6. Add state devtools
7. Document state management strategy""",
        "verification": """# Check state management
# Verify no duplicate API calls
# Test state synchronization
# Check devtools integration"""
    },
    
    {
        "number": 98,
        "title": "🟡 MEDIUM: No Contract Event Indexing Backfill",
        "labels": ["medium-priority", "backend", "contracts", "indexing", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/services/event_indexer.rs, backend/src/jobs/contract_event_listener.rs",
        "estimate": "8-12 hours",
        "description": """Contract event indexer only processes new events:
- No historical event backfill
- Cannot recover from missed events
- No gap detection
- No reorg handling
- Data completeness issues""",
        "impact": """⚠️ Incomplete event data
⚠️ Cannot recover from downtime
⚠️ Data gaps
⚠️ Analytics inaccurate
⚠️ User confusion""",
        "solution": """1. Implement backfill mechanism
2. Add gap detection in event sequence
3. Handle blockchain reorgs
4. Add progress tracking for backfill
5. Implement rate limiting for backfill
6. Add backfill status endpoint
7. Document backfill procedures""",
        "verification": """# Start backfill
curl -X POST http://localhost:8080/admin/backfill -d '{"from_ledger": 1000, "to_ledger": 2000}'

# Check progress
curl http://localhost:8080/admin/backfill/status

# Verify events indexed"""
    },
    
    {
        "number": 99,
        "title": "🟡 MEDIUM: No API Response Caching Headers",
        "labels": ["medium-priority", "performance", "backend", "caching", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/api/, backend/src/main.rs",
        "estimate": "4-6 hours",
        "description": """API responses lack caching headers:
- No Cache-Control headers
- No ETag support
- No Last-Modified headers
- No conditional requests (304)
- Unnecessary data transfer""",
        "impact": """⚠️ Wasted bandwidth
⚠️ Slower responses
⚠️ Higher server load
⚠️ Poor mobile experience
⚠️ Increased costs""",
        "solution": """1. Add Cache-Control headers per endpoint
2. Implement ETag generation
3. Support If-None-Match requests
4. Add Last-Modified headers
5. Return 304 Not Modified when appropriate
6. Configure cache TTLs per endpoint
7. Document caching strategy""",
        "verification": """# Test caching
curl -I http://localhost:8080/api/corridors

# Verify Cache-Control header
# Test ETag with If-None-Match
# Verify 304 response"""
    },
    
    {
        "number": 100,
        "title": "🟡 MEDIUM: No Frontend Internationalization Testing",
        "labels": ["medium-priority", "frontend", "i18n", "testing", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/app/[locale]/, frontend/messages/",
        "estimate": "6-8 hours",
        "description": """Internationalization implemented but not tested:
- No tests for translations
- No missing translation detection
- No RTL layout testing
- No locale switching tests
- Translation keys may be missing""",
        "impact": """⚠️ Broken translations
⚠️ Missing translations
⚠️ RTL layout issues
⚠️ Poor international UX
⚠️ User complaints""",
        "solution": """1. Add tests for all translation keys
2. Detect missing translations
3. Test RTL layouts (Arabic, Hebrew)
4. Test locale switching
5. Add translation coverage report
6. Validate translation files
7. Add i18n to CI pipeline""",
        "verification": """# Run i18n tests
npm run test:i18n

# Check translation coverage
npm run i18n:coverage

# Test RTL layout
# Verify no missing keys"""
    },
    
    {
        "number": 101,
        "title": "🟡 MEDIUM: No Database Connection Pooling Metrics",
        "labels": ["medium-priority", "observability", "backend", "database", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/database.rs, backend/src/observability/metrics.rs",
        "estimate": "4-6 hours",
        "description": """Database connection pool not monitored:
- No metrics for pool size
- No metrics for wait time
- No metrics for connection errors
- Cannot detect pool exhaustion
- No capacity planning data""",
        "impact": """⚠️ Cannot detect pool issues
⚠️ No capacity planning
⚠️ Performance degradation invisible
⚠️ Poor observability
⚠️ Debugging difficulties""",
        "solution": """1. Add metrics for pool size (active, idle)
2. Track connection wait time
3. Track connection errors
4. Add pool utilization percentage
5. Create pool monitoring dashboard
6. Add alerts for high utilization
7. Export metrics to Prometheus""",
        "verification": """curl http://localhost:8080/metrics | grep db_pool

# Verify metrics present:
# - db_pool_connections_active
# - db_pool_connections_idle
# - db_pool_wait_time_seconds
# - db_pool_errors_total"""
    },
    
    {
        "number": 102,
        "title": "🟡 MEDIUM: No Frontend Component Library Documentation",
        "labels": ["medium-priority", "documentation", "frontend", "components", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/components/",
        "estimate": "8-12 hours",
        "description": """Reusable components lack documentation:
- No Storybook or component docs
- No usage examples
- No prop documentation
- Hard to discover components
- Duplicate components created""",
        "impact": """⚠️ Component duplication
⚠️ Inconsistent UI
⚠️ Poor developer experience
⚠️ Onboarding difficulties
⚠️ Maintenance burden""",
        "solution": """1. Set up Storybook
2. Document all reusable components
3. Add usage examples
4. Document props and variants
5. Add accessibility notes
6. Create component guidelines
7. Deploy Storybook to staging""",
        "verification": """# Start Storybook
npm run storybook

# Verify all components documented
# Check usage examples
# Verify accessibility notes"""
    },
    
    {
        "number": 103,
        "title": "🟡 MEDIUM: No Contract Audit Trail",
        "labels": ["medium-priority", "contracts", "security", "audit", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "contracts/analytics/src/lib.rs, contracts/governance/src/lib.rs",
        "estimate": "6-8 hours",
        "description": """Smart contracts lack comprehensive audit trail:
- Admin actions not logged
- Parameter changes not tracked
- No event emission for critical operations
- Cannot reconstruct history
- Compliance issues""",
        "impact": """⚠️ No audit trail
⚠️ Cannot track changes
⚠️ Compliance issues
⚠️ Security concerns
⚠️ Debugging difficulties""",
        "solution": """1. Emit events for all admin actions
2. Log parameter changes with old/new values
3. Add timestamp to all events
4. Include caller address in events
5. Create event indexer for audit log
6. Add audit log query API
7. Document audit trail""",
        "verification": """# Perform admin action
stellar contract invoke --id $CONTRACT_ID -- update_admin

# Query events
stellar contract events --id $CONTRACT_ID

# Verify event emitted with details"""
    },
    
    {
        "number": 104,
        "title": "🟡 MEDIUM: No API Request Validation Middleware",
        "labels": ["medium-priority", "backend", "validation", "middleware", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/api/, backend/src/middleware/",
        "estimate": "8-12 hours",
        "description": """Request validation scattered across handlers:
- No centralized validation
- Inconsistent error messages
- Duplicate validation logic
- Hard to maintain
- Missing validations""",
        "impact": """⚠️ Inconsistent validation
⚠️ Code duplication
⚠️ Maintenance burden
⚠️ Missing validations
⚠️ Poor error messages""",
        "solution": """1. Create validation middleware
2. Use validator crate for rules
3. Define validation schemas
4. Centralize error messages
5. Add custom validators
6. Document validation rules
7. Add validation tests""",
        "verification": """# Test validation
curl -X POST http://localhost:8080/api/corridors -d '{"invalid": "data"}'

# Verify consistent error format
# Check error messages
# Verify all fields validated"""
    },
    
    {
        "number": 105,
        "title": "🟡 MEDIUM: No Frontend Form Validation Library",
        "labels": ["medium-priority", "frontend", "validation", "forms", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/components/Sep24Flow.tsx, Sep31PaymentFlow.tsx, CostCalculator.tsx",
        "estimate": "8-12 hours",
        "description": """Forms use manual validation without library:
- Inconsistent validation logic
- No validation schemas
- Poor error handling
- Duplicate validation code
- Hard to maintain""",
        "impact": """⚠️ Inconsistent validation
⚠️ Code duplication
⚠️ Poor UX
⚠️ Maintenance burden
⚠️ Missing validations""",
        "solution": """1. Integrate React Hook Form
2. Add Zod for schema validation
3. Create reusable validation schemas
4. Standardize error messages
5. Add field-level validation
6. Improve error display
7. Add validation tests""",
        "verification": """# Test form validation
# Submit invalid data
# Verify error messages
# Check field-level validation
# Verify accessibility"""
    },
    
    {
        "number": 106,
        "title": "🟡 MEDIUM: No Background Job Monitoring",
        "labels": ["medium-priority", "observability", "backend", "jobs", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/jobs/, backend/src/observability/",
        "estimate": "6-8 hours",
        "description": """Background jobs not monitored:
- No job execution metrics
- No failure tracking
- No duration tracking
- Cannot detect stuck jobs
- No job status dashboard""",
        "impact": """⚠️ Cannot detect job failures
⚠️ No visibility into job health
⚠️ Stuck jobs undetected
⚠️ Poor observability
⚠️ Debugging difficulties""",
        "solution": """1. Add metrics for job execution
2. Track job duration
3. Track job failures
4. Add job status endpoint
5. Create job monitoring dashboard
6. Add alerts for job failures
7. Log job execution details""",
        "verification": """curl http://localhost:8080/metrics | grep job_

# Verify metrics:
# - job_executions_total
# - job_duration_seconds
# - job_failures_total
# - job_last_success_timestamp"""
    },
    
    {
        "number": 107,
        "title": "🟡 MEDIUM: No API Response Compression Configuration",
        "labels": ["medium-priority", "performance", "backend", "optimization", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/main.rs, backend/.env.example",
        "estimate": "3-4 hours",
        "description": """Response compression configured but not optimized:
- Compression level not tunable
- No compression for small responses
- No content-type filtering
- No compression metrics
- Unknown compression ratio""",
        "impact": """⚠️ Wasted bandwidth
⚠️ Slower responses for small payloads
⚠️ CPU overhead
⚠️ Cannot optimize
⚠️ No visibility""",
        "solution": """1. Make compression level configurable
2. Skip compression for responses < 1KB
3. Filter by content-type (JSON, HTML only)
4. Add compression metrics
5. Track compression ratio
6. Benchmark compression overhead
7. Document compression strategy""",
        "verification": """# Test compression
curl -H "Accept-Encoding: gzip" http://localhost:8080/api/corridors

# Verify Content-Encoding header
# Check compression ratio
# Verify small responses not compressed"""
    },
    
    {
        "number": 108,
        "title": "🟡 MEDIUM: No Frontend Error Boundary Testing",
        "labels": ["medium-priority", "testing", "frontend", "error-handling", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/components/ErrorBoundary.tsx, frontend/src/app/",
        "estimate": "4-6 hours",
        "description": """Error boundaries exist but not tested:
- No tests for error scenarios
- No tests for fallback UI
- No tests for error recovery
- Unknown if error boundaries work
- No error boundary coverage""",
        "impact": """⚠️ Error boundaries may not work
⚠️ No confidence in error handling
⚠️ Poor UX on errors
⚠️ Regression risk
⚠️ Production surprises""",
        "solution": """1. Add tests for error boundaries
2. Test error scenarios
3. Test fallback UI rendering
4. Test error recovery
5. Test error reporting
6. Add error boundary coverage
7. Document error handling""",
        "verification": """# Run error boundary tests
npm run test -- ErrorBoundary

# Verify all scenarios covered
# Check coverage report
# Test in browser"""
    },
    
    {
        "number": 109,
        "title": "🟡 MEDIUM: No Contract Storage Cost Optimization",
        "labels": ["medium-priority", "contracts", "optimization", "storage", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "contracts/analytics/src/lib.rs, contracts/governance/src/lib.rs",
        "estimate": "8-12 hours",
        "description": """Contract storage not optimized for cost:
- Storing full data instead of hashes
- No data compression
- No storage cleanup
- Expired data not removed
- High storage costs""",
        "impact": """⚠️ High storage costs
⚠️ Wasted resources
⚠️ Poor scalability
⚠️ User cost burden
⚠️ Competitive disadvantage""",
        "solution": """1. Store hashes instead of full data where possible
2. Implement data compression
3. Add storage cleanup mechanism
4. Remove expired data automatically
5. Optimize data structures
6. Add storage cost metrics
7. Document storage strategy""",
        "verification": """# Measure storage usage
stellar contract invoke --id $CONTRACT_ID -- get_storage_stats

# Compare before/after optimization
# Verify cost reduction > 50%"""
    },
    
    {
        "number": 110,
        "title": "🟡 MEDIUM: No API Deprecation Warnings",
        "labels": ["medium-priority", "api", "backend", "deprecation", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/src/api/, backend/src/middleware/",
        "estimate": "4-6 hours",
        "description": """No mechanism for API deprecation warnings:
- Cannot warn clients about deprecations
- No sunset dates communicated
- Breaking changes surprise clients
- Poor API governance
- Migration difficulties""",
        "impact": """⚠️ Breaking changes surprise clients
⚠️ Poor API governance
⚠️ Migration difficulties
⚠️ Support burden
⚠️ Client frustration""",
        "solution": """1. Add Deprecation header to deprecated endpoints
2. Add Sunset header with date
3. Add deprecation warnings to responses
4. Document deprecation policy
5. Create migration guides
6. Track deprecated endpoint usage
7. Alert when deprecated endpoints used""",
        "verification": """# Call deprecated endpoint
curl -I http://localhost:8080/api/v1/old-endpoint

# Verify Deprecation header
# Verify Sunset header
# Check warning in response"""
    },
    
    {
        "number": 111,
        "title": "🟡 MEDIUM: No Frontend Accessibility Testing Automation",
        "labels": ["medium-priority", "testing", "frontend", "accessibility", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/src/, .github/workflows/",
        "estimate": "6-8 hours",
        "description": """Accessibility testing not automated:
- axe-core configured but not in CI
- No automated WCAG testing
- No keyboard navigation tests
- No screen reader tests
- Accessibility regressions possible""",
        "impact": """⚠️ Accessibility regressions
⚠️ WCAG non-compliance
⚠️ Poor disabled user experience
⚠️ Legal risk
⚠️ No confidence in accessibility""",
        "solution": """1. Add axe-core tests to CI
2. Test all pages for WCAG AA
3. Add keyboard navigation tests
4. Test with screen reader (pa11y)
5. Add accessibility coverage report
6. Fail CI on violations
7. Document accessibility standards""",
        "verification": """# Run accessibility tests
npm run test:a11y

# Verify CI fails on violations
# Check coverage report
# Test with screen reader"""
    },
    
    {
        "number": 112,
        "title": "🟡 MEDIUM: No Database Index Usage Analysis",
        "labels": ["medium-priority", "database", "backend", "performance", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend",
        "file": "backend/migrations/, backend/src/database.rs",
        "estimate": "6-8 hours",
        "description": """Database indexes not analyzed for effectiveness:
- Unknown if indexes are used
- No index usage statistics
- May have unused indexes
- May be missing critical indexes
- No index optimization""",
        "impact": """⚠️ Slow queries
⚠️ Wasted storage on unused indexes
⚠️ Missing critical indexes
⚠️ Poor query performance
⚠️ Cannot optimize""",
        "solution": """1. Enable SQLite query planner analysis
2. Log EXPLAIN QUERY PLAN for slow queries
3. Identify unused indexes
4. Identify missing indexes
5. Add index usage metrics
6. Create index optimization guide
7. Review indexes quarterly""",
        "verification": """# Analyze index usage
sqlite3 stellar_insights.db "EXPLAIN QUERY PLAN SELECT * FROM corridors WHERE source_asset = 'USDC'"

# Check index usage
# Identify unused indexes
# Add missing indexes"""
    },
    
    {
        "number": 113,
        "title": "🟡 MEDIUM: No Frontend Build Optimization",
        "labels": ["medium-priority", "performance", "frontend", "build", "P2"],
        "priority": "P2 - Medium",
        "area": "Frontend",
        "file": "frontend/next.config.ts, frontend/package.json",
        "estimate": "6-8 hours",
        "description": """Frontend build not optimized:
- No build time tracking
- No build cache optimization
- No parallel builds
- Slow CI builds
- No build performance metrics""",
        "impact": """⚠️ Slow CI builds
⚠️ Slow deployments
⚠️ Developer frustration
⚠️ Wasted CI time
⚠️ Higher costs""",
        "solution": """1. Enable Next.js build cache
2. Use SWC instead of Babel
3. Enable parallel builds
4. Track build time metrics
5. Optimize dependencies
6. Use build cache in CI
7. Target build time < 2 minutes""",
        "verification": """# Measure build time
time npm run build

# Verify build cache used
# Check build time metrics
# Verify parallel builds"""
    },
    
    {
        "number": 114,
        "title": "🟡 MEDIUM: No Contract Fuzzing Tests",
        "labels": ["medium-priority", "testing", "contracts", "security", "P2"],
        "priority": "P2 - Medium",
        "area": "Contracts",
        "file": "contracts/analytics/src/tests.rs, contracts/governance/src/tests.rs",
        "estimate": "12-16 hours",
        "description": """Smart contracts lack fuzzing tests:
- No property-based testing
- No random input testing
- Edge cases not covered
- Potential vulnerabilities
- No fuzzing in CI""",
        "impact": """⚠️ Undiscovered edge cases
⚠️ Potential vulnerabilities
⚠️ Poor test coverage
⚠️ Security risks
⚠️ Production bugs""",
        "solution": """1. Implement fuzzing with proptest
2. Add property-based tests
3. Test with random inputs
4. Test boundary conditions
5. Add fuzzing to CI
6. Document fuzzing strategy
7. Target 1000+ fuzzing iterations""",
        "verification": """# Run fuzzing tests
cargo test --release -- --ignored fuzz

# Verify 1000+ iterations
# Check for panics
# Review edge cases found"""
    },
    
    {
        "number": 115,
        "title": "🟡 MEDIUM: No API Client SDK",
        "labels": ["medium-priority", "api", "sdk", "developer-experience", "P2"],
        "priority": "P2 - Medium",
        "area": "Backend & Documentation",
        "file": "docs/, sdk/",
        "estimate": "16-24 hours",
        "description": """No official API client SDK:
- Clients must implement HTTP manually
- No type safety for clients
- Poor developer experience
- Integration difficulties
- Higher support burden""",
        "impact": """⚠️ Poor developer experience
⚠️ Integration difficulties
⚠️ Higher support burden
⚠️ Adoption friction
⚠️ More bugs in client code""",
        "solution": """1. Generate TypeScript SDK from OpenAPI
2. Generate Python SDK
3. Add authentication helpers
4. Add retry logic
5. Add rate limiting handling
6. Publish to npm/PyPI
7. Document SDK usage""",
        "verification": """# Install SDK
npm install @stellar-insights/sdk

# Use SDK
import { StellarInsights } from '@stellar-insights/sdk'
const client = new StellarInsights({ apiKey: 'xxx' })
const corridors = await client.corridors.list()"""
    },
    
    # ========================================================================
    # LOW PRIORITY ISSUES (10) - P3 Priority
    # ========================================================================
    
    {
        "number": 116,
        "title": "🟢 LOW: No Code Coverage Tracking",
        "labels": ["low-priority", "testing", "backend", "frontend", "quality", "P3"],
        "priority": "P3 - Low",
        "area": "Backend & Frontend",
        "file": ".github/workflows/, backend/Cargo.toml, frontend/package.json",
        "estimate": "4-6 hours",
        "description": """No code coverage tracking or reporting:
- Unknown test coverage percentage
- Cannot identify untested code
- No coverage trends
- No coverage requirements
- No coverage in CI""",
        "impact": """⚠️ Unknown test coverage
⚠️ Cannot identify gaps
⚠️ No quality metrics
⚠️ Regression risk
⚠️ Poor visibility""",
        "solution": """1. Add tarpaulin for Rust coverage
2. Add c8 for TypeScript coverage
3. Generate coverage reports
4. Upload to Codecov
5. Add coverage badges
6. Set minimum coverage (80%)
7. Fail CI if coverage drops""",
        "verification": """# Generate coverage
cargo tarpaulin --out Html
npm run test:coverage

# Check coverage report
# Verify Codecov integration
# Check coverage badge"""
    },
    
    {
        "number": 117,
        "title": "🟢 LOW: No Changelog Automation",
        "labels": ["low-priority", "documentation", "automation", "P3"],
        "priority": "P3 - Low",
        "area": "Project",
        "file": "CHANGELOG.md, .github/workflows/",
        "estimate": "3-4 hours",
        "description": """Changelog manually maintained:
- No automated changelog generation
- Inconsistent format
- Missing entries
- Hard to maintain
- No release notes automation""",
        "impact": """⚠️ Inconsistent changelog
⚠️ Missing entries
⚠️ Maintenance burden
⚠️ Poor release notes
⚠️ User confusion""",
        "solution": """1. Use conventional commits
2. Add commitlint to CI
3. Generate changelog with git-cliff
4. Automate on release
5. Include breaking changes
6. Link to issues/PRs
7. Document commit conventions""",
        "verification": """# Generate changelog
git-cliff --output CHANGELOG.md

# Verify format
# Check all commits included
# Verify links work"""
    },
    
    {
        "number": 118,
        "title": "🟢 LOW: No Contributor Guidelines",
        "labels": ["low-priority", "documentation", "community", "P3"],
        "priority": "P3 - Low",
        "area": "Project",
        "file": "CONTRIBUTING.md, docs/",
        "estimate": "4-6 hours",
        "description": """No contributor guidelines:
- No CONTRIBUTING.md
- No code of conduct
- No PR template
- No issue templates
- Hard for new contributors""",
        "impact": """⚠️ Poor contributor experience
⚠️ Inconsistent contributions
⚠️ More review cycles
⚠️ Onboarding difficulties
⚠️ Community friction""",
        "solution": """1. Create CONTRIBUTING.md
2. Add CODE_OF_CONDUCT.md
3. Create PR template
4. Create issue templates
5. Document development setup
6. Add commit conventions
7. Document review process""",
        "verification": """# Check files exist
ls CONTRIBUTING.md CODE_OF_CONDUCT.md .github/PULL_REQUEST_TEMPLATE.md

# Verify templates work
# Test new contributor flow"""
    },
    
    {
        "number": 119,
        "title": "🟢 LOW: No Performance Budgets Defined",
        "labels": ["low-priority", "performance", "frontend", "standards", "P3"],
        "priority": "P3 - Low",
        "area": "Frontend",
        "file": "frontend/next.config.ts, .github/workflows/",
        "estimate": "3-4 hours",
        "description": """No performance budgets defined:
- No bundle size limits
- No load time targets
- No Core Web Vitals targets
- Cannot prevent regressions
- No performance standards""",
        "impact": """⚠️ Performance regressions
⚠️ No standards
⚠️ Cannot prevent bloat
⚠️ Poor user experience
⚠️ No accountability""",
        "solution": """1. Define bundle size budget (200KB)
2. Define load time budget (3s)
3. Define Core Web Vitals targets
4. Add budget checks to CI
5. Fail CI on budget violations
6. Document performance standards
7. Monitor budgets in production""",
        "verification": """# Check bundle size
npm run build
# Verify size < 200KB

# Check Lighthouse score
lighthouse http://localhost:3000
# Verify score > 90"""
    },
    
    {
        "number": 120,
        "title": "🟢 LOW: No Database Query Explain Plan Logging",
        "labels": ["low-priority", "database", "backend", "observability", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/src/database.rs, backend/src/observability/",
        "estimate": "4-6 hours",
        "description": """Slow queries logged but no explain plans:
- Cannot diagnose slow queries
- No query optimization data
- Missing index detection difficult
- Poor debugging experience
- Manual analysis required""",
        "impact": """⚠️ Hard to optimize queries
⚠️ Cannot identify issues
⚠️ Manual analysis required
⚠️ Poor observability
⚠️ Debugging difficulties""",
        "solution": """1. Log EXPLAIN QUERY PLAN for slow queries
2. Parse and format explain output
3. Identify missing indexes
4. Add explain plan to logs
5. Create query optimization guide
6. Add explain plan to metrics
7. Document query optimization""",
        "verification": """# Trigger slow query
# Check logs for EXPLAIN output
tail -f logs/slow_queries.log

# Verify explain plan present
# Verify actionable information"""
    },
    
    {
        "number": 121,
        "title": "🟢 LOW: No Frontend Performance Profiling",
        "labels": ["low-priority", "performance", "frontend", "profiling", "P3"],
        "priority": "P3 - Low",
        "area": "Frontend",
        "file": "frontend/src/, frontend/next.config.ts",
        "estimate": "4-6 hours",
        "description": """No frontend performance profiling setup:
- No React DevTools profiler
- No performance marks
- No user timing API
- Cannot identify bottlenecks
- No profiling in development""",
        "impact": """⚠️ Cannot identify bottlenecks
⚠️ Hard to optimize
⚠️ Poor developer experience
⚠️ Performance issues invisible
⚠️ Guesswork optimization""",
        "solution": """1. Add React DevTools profiler
2. Add performance marks
3. Use User Timing API
4. Add profiling helpers
5. Document profiling process
6. Create profiling guide
7. Add profiling to development""",
        "verification": """# Profile component
import { Profiler } from 'react'

# Check performance marks
performance.getEntriesByType('mark')

# Use React DevTools profiler"""
    },
    
    {
        "number": 122,
        "title": "🟢 LOW: No Contract Gas Benchmarking",
        "labels": ["low-priority", "contracts", "performance", "benchmarking", "P3"],
        "priority": "P3 - Low",
        "area": "Contracts",
        "file": "contracts/benches/, contracts/analytics/, contracts/governance/",
        "estimate": "6-8 hours",
        "description": """No gas benchmarking for contracts:
- Unknown gas costs
- Cannot track gas regressions
- No optimization targets
- No gas comparison
- Poor cost visibility""",
        "impact": """⚠️ Unknown gas costs
⚠️ Cannot detect regressions
⚠️ Cannot optimize effectively
⚠️ User cost surprises
⚠️ No cost planning""",
        "solution": """1. Create gas benchmarks
2. Benchmark all contract functions
3. Track gas costs over time
4. Add gas benchmarks to CI
5. Fail CI on gas regressions
6. Document gas costs
7. Create gas optimization guide""",
        "verification": """# Run gas benchmarks
cargo bench --package analytics

# Check gas costs
# Compare with previous version
# Verify no regressions"""
    },
    
    {
        "number": 123,
        "title": "🟢 LOW: No API Response Time SLOs",
        "labels": ["low-priority", "observability", "backend", "slo", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "docs/, backend/src/observability/",
        "estimate": "3-4 hours",
        "description": """No Service Level Objectives defined:
- No response time targets
- No availability targets
- No error rate targets
- Cannot measure service quality
- No SLO monitoring""",
        "impact": """⚠️ No service quality metrics
⚠️ Cannot measure reliability
⚠️ No accountability
⚠️ User expectations unclear
⚠️ No improvement targets""",
        "solution": """1. Define SLOs for key endpoints
2. Set p95 latency targets (< 500ms)
3. Set availability target (99.9%)
4. Set error rate target (< 1%)
5. Add SLO monitoring
6. Create SLO dashboard
7. Document SLOs""",
        "verification": """# Check SLO dashboard
# Verify SLO tracking
# Check SLO compliance
# Review SLO documentation"""
    },
    
    {
        "number": 124,
        "title": "🟢 LOW: No Frontend Component Testing",
        "labels": ["low-priority", "testing", "frontend", "components", "P3"],
        "priority": "P3 - Low",
        "area": "Frontend",
        "file": "frontend/src/components/, frontend/src/app/",
        "estimate": "16-24 hours",
        "description": """No component unit tests:
- Only integration tests exist
- Components not tested in isolation
- Hard to test edge cases
- Poor test coverage
- Regression risk""",
        "impact": """⚠️ Poor test coverage
⚠️ Cannot test edge cases
⚠️ Regression risk
⚠️ Hard to refactor
⚠️ Low confidence""",
        "solution": """1. Add React Testing Library tests
2. Test all reusable components
3. Test component props
4. Test user interactions
5. Test edge cases
6. Add component test coverage
7. Document testing patterns""",
        "verification": """# Run component tests
npm run test -- components/

# Check coverage
npm run test:coverage

# Verify all components tested"""
    },
    
    {
        "number": 125,
        "title": "🟢 LOW: No Database Seeding for Development",
        "labels": ["low-priority", "development", "backend", "database", "P3"],
        "priority": "P3 - Low",
        "area": "Backend",
        "file": "backend/seed_data.sh, backend/src/database.rs",
        "estimate": "4-6 hours",
        "description": """seed_data.sh exists but incomplete:
- Limited seed data
- No realistic data generation
- Hard to test features
- Poor development experience
- Manual data creation needed""",
        "impact": """⚠️ Poor development experience
⚠️ Hard to test features
⚠️ Manual data creation
⚠️ Inconsistent test data
⚠️ Onboarding difficulties""",
        "solution": """1. Expand seed data script
2. Generate realistic data
3. Add data for all entities
4. Add data relationships
5. Make seeding idempotent
6. Document seeding process
7. Add seed data to CI""",
        "verification": """# Run seed script
./backend/seed_data.sh

# Verify data created
sqlite3 stellar_insights.db "SELECT COUNT(*) FROM corridors"

# Test application with seed data"""
    }
]
