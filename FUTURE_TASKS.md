# 🔧 Future Tasks & Code Improvements

**Last Updated:** February 26, 2026  
**Status:** Cleanup Phase 1 Complete

---

## 📊 Current State

### ✅ Completed (Phase 1)
- Deleted 166+ duplicate markdown files (50,000+ LOC removed)
- Removed embedded "Sponsored Reserves Monitor" project (236KB)
- Removed dead code modules (apm, gdpr, elk_health)
- Removed duplicate contract directories (5 task-named dirs)
- Removed test/verification scripts and temp files
- Cleaned up commented code in lib.rs
- Reduced git history size (236M → 191M, saved 45M)

### 📦 Current Codebase Size
```
Total:     191M (includes .git at 183M)
Backend:   2.4M
Frontend:  3.0M
Contracts: 1.3M
Actual Code: ~7M
```

---

## 🚨 Critical Issues to Fix

### 1. **CACHED vs NON-CACHED API DUPLICATION** ⚠️ HIGH PRIORITY
**Problem:** Every major API endpoint exists twice with duplicated logic.

**Files:**
- `backend/src/api/corridors.rs` (510 LOC) + `corridors_cached.rs` (1,247 LOC)
- `backend/src/api/anchors.rs` (109 LOC) + `anchors_cached.rs` (274 LOC)
- `backend/src/api/metrics.rs` (855 LOC) + `metrics_cached.rs` (2,400 LOC)

**Impact:** ~2,000+ lines of pure duplication, bug fixes must be applied twice

**Solution:**
1. Implement caching as middleware/decorator pattern
2. Delete all `*_cached.rs` files
3. Use single implementation with cache layer

**Estimated Time:** 2-3 days

---

### 2. **MONOLITHIC ROOT-LEVEL FILES** ⚠️ HIGH PRIORITY
**Problem:** 43 Rust files at `backend/src/` root instead of organized modules.

**Largest Files:**
- `database.rs` (1,480 LOC) - should be split into modules
- `analytics.rs` (462 LOC)
- `rate_limit.rs` (460 LOC)
- `websocket.rs` (456 LOC)
- `models.rs` (433 LOC)
- `error.rs` (404 LOC)
- `cache.rs` (381 LOC)
- `handlers.rs` (367 LOC)

**Solution:**
1. Create feature-based module structure:
   ```
   backend/src/
   ├── features/
   │   ├── corridors/
   │   │   ├── mod.rs
   │   │   ├── handlers.rs
   │   │   ├── models.rs
   │   │   └── service.rs
   │   ├── anchors/
   │   ├── analytics/
   │   └── payments/
   ├── infrastructure/
   │   ├── database/
   │   ├── cache/
   │   └── websocket/
   └── shared/
       ├── error.rs
       └── validation.rs
   ```
2. Split `database.rs` into separate modules (queries, migrations, connection)
3. Move models into their respective feature modules

**Estimated Time:** 3-4 days

---

### 3. **MULTIPLE OVERLAPPING SYSTEMS** ⚠️ MEDIUM PRIORITY

#### A. Notification Systems (4 implementations)
**Files:**
- `backend/src/telegram/` (5 files)
- `backend/src/email/` (3 files)
- `backend/src/webhooks/` (2 files)
- `backend/src/broadcast.rs`

**Solution:** Create unified notification service with multiple channels
```rust
// Unified interface
trait NotificationChannel {
    async fn send(&self, message: Message) -> Result<()>;
}

// Implementations
struct TelegramChannel;
struct EmailChannel;
struct WebhookChannel;
```

**Estimated Time:** 2 days

#### B. Cache Implementations (3+ implementations)
**Files:**
- `backend/src/cache.rs` (381 LOC)
- `backend/src/http_cache.rs` (231 LOC)
- `backend/src/cache_middleware.rs`
- `backend/src/cache_invalidation.rs`

**Solution:** Consolidate to single cache layer with clear strategy
- Keep Redis cache as primary
- Remove redundant implementations
- Centralize invalidation logic

**Estimated Time:** 2 days

#### C. Authentication Methods (4 implementations)
**Files:**
- `backend/src/auth/sep10.rs`
- `backend/src/auth/sep10_simple.rs`
- `backend/src/auth/sep10_middleware.rs`
- `backend/src/auth/oauth.rs`

**Solution:** 
1. Document which is canonical (likely `sep10.rs`)
2. Remove or clearly mark others as deprecated
3. Consolidate into single auth module

**Estimated Time:** 1-2 days

#### D. Handler Files (7 files)
**Files:**
- `backend/src/handlers.rs`
- `backend/src/alert_handlers.rs`
- `backend/src/snapshot_handlers.rs`
- `backend/src/rpc_handlers.rs`
- `backend/src/ml_handlers.rs`
- `backend/src/api/replay_handlers.rs`

**Solution:** Move handlers into their respective feature modules

**Estimated Time:** 1 day

---

### 4. **WEAK TEST COVERAGE** ⚠️ HIGH PRIORITY
**Current State:**
- Backend: <10% coverage (only 6 test files, mostly stubs)
- Frontend: <15% coverage (13 test files, mostly a11y tests)

**Missing Tests:**
- Corridor calculation logic
- Cache invalidation
- API endpoints (integration tests)
- Payment processing
- GraphQL resolvers
- Frontend components (functional tests)
- State management

**Solution:**
1. Add unit tests for core business logic
2. Add integration tests for API endpoints
3. Add E2E tests for critical user flows
4. Set up CI to enforce minimum coverage (70%+)

**Estimated Time:** 1-2 weeks

---

### 5. **DUAL API IMPLEMENTATIONS** ⚠️ MEDIUM PRIORITY
**Problem:** Both REST and GraphQL APIs fully implemented

**Files:**
- REST: `backend/src/api/` (30+ files)
- GraphQL: `backend/src/graphql/` (10+ files)

**Impact:** Double maintenance burden, unclear which to use

**Solution:**
1. Document use cases for each:
   - REST: Public API, simple queries, caching
   - GraphQL: Complex queries, relationships, real-time
2. OR: Pick one as primary, deprecate the other
3. OR: Keep both but share business logic layer

**Estimated Time:** 1 day (decision) + 3-5 days (implementation)

---

### 6. **MIDDLEWARE SPRAWL** ⚠️ LOW PRIORITY
**Problem:** 15+ middleware files with unclear separation

**Files:**
- `cache_middleware.rs`
- `auth_middleware.rs`
- `ip_whitelist_middleware.rs`
- `api_analytics_middleware.rs`
- `api_v1_middleware.rs`
- `api_v2_middleware.rs`
- `request_signing_middleware.rs`
- And more...

**Solution:**
1. Audit which middleware are actually used
2. Consolidate overlapping functionality
3. Create clear middleware chain documentation

**Estimated Time:** 2 days

---

### 7. **DEPENDENCY CLEANUP** ⚠️ LOW PRIORITY

#### Backend (Cargo.toml)
**Remove/Replace:**
- `md5` → Use `sha2` (md5 is cryptographically broken)
- `lazy_static` → Use `OnceLock` (modern Rust)
- `dotenv` + `dotenvy` → Pick one
- `tokio` features = "full" → Use specific features only
- `ndarray` → Remove if unused (ML library)

**Consolidate Observability:**
- 6 tracing crates → Reduce to 3-4
- 3 OpenTelemetry crates → Evaluate necessity

#### Frontend (package.json)
**Evaluate:**
- `d3-force-3d` → Remove if 3D graphs not essential (large bundle)
- `jspdf` + `jspdf-autotable` → Consider server-side PDF generation
- `framer-motion` → Evaluate if animation complexity justified

**Estimated Time:** 1 day

---

### 8. **API VERSIONING CONFUSION** ⚠️ LOW PRIORITY
**Problem:** Multiple API version files but no clear strategy

**Files:**
- `api_v1_middleware.rs`
- `api_v2_middleware.rs`
- `api/v1/` directory (mostly empty)

**Solution:**
1. Document current API version
2. Remove unused version files
3. Implement proper versioning strategy if needed

**Estimated Time:** 1 day

---

### 9. **CONTRACT ORGANIZATION** ⚠️ LOW PRIORITY
**Current State:** 9 contract directories with unclear purpose

**Directories:**
- `access-control/`
- `analytics/`
- `example-contract/`
- `governance/`
- `secure-contract/`
- `snapshot-contract/`
- `stellar_insights/`

**Solution:**
1. Document purpose of each contract
2. Remove example/template contracts if not needed
3. Consolidate related contracts
4. Add README explaining contract architecture

**Estimated Time:** 1 day

---

### 10. **FRONTEND COMPONENT SIZE** ⚠️ MEDIUM PRIORITY
**Problem:** Large components (>500 LOC) doing too much

**Largest Components:**
- `EnhancedNotificationCenter.tsx` (1,123 LOC)
- `api.ts` (762 LOC)
- `anchors/page.tsx` (756 LOC)
- `NotificationCenter.tsx` (661 LOC)
- `developer/keys/page.tsx` (660 LOC)

**Solution:** Break down into smaller, reusable components (<300 LOC rule)

**Estimated Time:** 3-4 days

---

## 🎯 Recommended Implementation Order

### Week 1: Foundation
1. **Add comprehensive tests** (enables safe refactoring)
2. **Fix cached endpoint duplication** (biggest code smell)
3. **Document API strategy** (REST vs GraphQL decision)

### Week 2: Structure
4. **Reorganize monolithic files** (feature-based modules)
5. **Consolidate notification systems**
6. **Consolidate cache implementations**

### Week 3: Cleanup
7. **Consolidate auth methods**
8. **Clean up dependencies**
9. **Fix API versioning**
10. **Audit middleware**

### Week 4: Polish
11. **Break down large frontend components**
12. **Organize contracts**
13. **Add missing documentation**
14. **Performance optimization**

---

## 📈 Success Metrics

**Code Quality:**
- [ ] Test coverage >70%
- [ ] No files >500 LOC
- [ ] No duplicate logic
- [ ] Clear module boundaries

**Performance:**
- [ ] API response time <100ms (p95)
- [ ] Frontend bundle size <500KB
- [ ] Database queries optimized

**Maintainability:**
- [ ] Clear documentation for all features
- [ ] Consistent code style
- [ ] CI/CD pipeline with quality gates
- [ ] Onboarding guide for new developers

---

## 🔍 Technical Debt Tracking

### High Priority (Do First)
- [ ] Cached endpoint duplication
- [ ] Monolithic file organization
- [ ] Test coverage

### Medium Priority (Do Next)
- [ ] Multiple overlapping systems
- [ ] Dual API implementations
- [ ] Large frontend components

### Low Priority (Nice to Have)
- [ ] Middleware sprawl
- [ ] Dependency cleanup
- [ ] API versioning
- [ ] Contract organization

---

## 💡 Architecture Improvements

### Consider for Future:
1. **Microservices:** Split analytics engine from API server
2. **Event Sourcing:** For audit trail and replay capability
3. **CQRS:** Separate read/write models for better performance
4. **Feature Flags:** For gradual rollout and A/B testing
5. **API Gateway:** For rate limiting, auth, routing
6. **Message Queue:** For async processing (RabbitMQ/Kafka)
7. **Read Replicas:** For database scaling
8. **CDN:** For static asset delivery

---

## 📚 Documentation Needed

### Missing Docs:
- [ ] Architecture decision records (ADRs)
- [ ] API authentication guide
- [ ] Deployment guide
- [ ] Monitoring & alerting setup
- [ ] Database schema documentation
- [ ] Contribution guidelines
- [ ] Security best practices
- [ ] Performance tuning guide

---

## 🐛 Known Issues

### Backend:
- [ ] Commented out `gdpr` module - remove or implement
- [ ] Multiple auth implementations - clarify which is canonical
- [ ] Cache invalidation strategy unclear
- [ ] Error handling inconsistent across modules
- [ ] Logging levels not standardized

### Frontend:
- [ ] Large bundle size (investigate code splitting)
- [ ] Missing error boundaries
- [ ] Accessibility testing incomplete
- [ ] State management could be simplified
- [ ] API client error handling needs improvement

### Contracts:
- [ ] No integration tests
- [ ] Unclear deployment strategy
- [ ] Missing upgrade mechanism
- [ ] No gas optimization audit

---

## 🚀 Quick Wins (Can Do Today)

1. **Remove unused dependencies** (30 min)
2. **Add .editorconfig for consistent formatting** (15 min)
3. **Set up pre-commit hooks** (30 min)
4. **Add CONTRIBUTING.md** (1 hour)
5. **Document environment variables** (30 min)
6. **Add health check endpoints** (1 hour)
7. **Set up error tracking (Sentry)** (1 hour)
8. **Add request ID middleware** (30 min)

---

**Next Review:** March 5, 2026  
**Owner:** Development Team
