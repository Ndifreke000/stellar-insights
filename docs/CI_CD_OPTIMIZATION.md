# CI/CD Pipeline Optimization Guide

This document outlines the optimizations applied to the Stellar Insights CI/CD pipelines for faster builds and deployments.

## Overview

The Stellar Insights CI/CD system consists of multiple GitHub Actions workflows that run on every push and pull request. These pipelines have been optimized for:

- **Dependency caching** to reduce install/build time
- **Job parallelization** to run independent checks concurrently
- **Build artifact caching** to avoid rebuilding unchanged code
- **Terraform validation** as a fast-failing early check
- **Performance monitoring** to track CI metrics

## Optimization Strategies Applied

### 1. Dependency Caching

**What**: Package manager caches are stored between workflow runs and restored on cache hits.

**Impact**: Reduces npm/pnpm/cargo install time from 2-3 minutes to 10-30 seconds on cache hits.

**Workflows affected**:
- `frontend-build.yml` - pnpm cache via `actions/setup-node`
- `coverage.yml` - cargo cache for Rust backend
- `deploy.yml` - Docker cache via GitHub Actions cache backend

**Example**:
```yaml
- uses: actions/setup-node@v4
  with:
    node-version: 22
    cache: pnpm  # Automatically caches ~/.pnpm-store
    cache-dependency-path: frontend/pnpm-lock.yaml
```

Cache invalidates automatically when:
- Lockfile (pnpm-lock.yaml, Cargo.lock) changes
- Node version or runner OS changes

### 2. Build Artifact Caching

**What**: Compiled artifacts and build outputs are cached between runs, keyed on input file hashes.

**Impact**: Next.js build cache reduces rebuild time from 60+ seconds to 5-10 seconds; cargo cache saves 30-60 seconds.

**Workflows affected**:
- `frontend-build.yml` - Next.js cache (.next/cache directory)
- `coverage.yml` - Rust build artifacts (backend/target)
- `deploy.yml` - Docker layers via Docker buildx

**Example**:
```yaml
- name: Restore Next.js build cache
  uses: actions/cache@v4
  with:
    path: frontend/.next/cache
    key: nextjs-${{ runner.os }}-${{ hashFiles('frontend/pnpm-lock.yaml') }}-${{ hashFiles('frontend/src/**') }}
    restore-keys: |
      nextjs-${{ runner.os }}-${{ hashFiles('frontend/pnpm-lock.yaml') }}-
      nextjs-${{ runner.os }}-
```

### 3. Docker Build Caching

**What**: Docker layer cache is stored in GitHub Actions cache and reused across builds.

**Impact**: Unchanged layers are skipped; rebuilds go from 5 minutes to 30-60 seconds.

**Workflow**: `deploy.yml`

**Config**:
```yaml
- name: Build and push image
  uses: docker/build-push-action@v6
  with:
    context: ./backend
    file: ./backend/Dockerfile
    push: true
    cache-from: type=gha  # Pull cache from GitHub Actions
    cache-to: type=gha,mode=max  # Store full cache for future runs
```

### 4. Parallel Job Execution

**What**: Independent jobs run concurrently instead of sequentially.

**Impact**: Reduces total pipeline time from sum of all jobs to the longest job duration.

**Workflows affected**:
- `coverage.yml` - backend-coverage and frontend-coverage run in parallel
- `security.yml` - security scans run in parallel
- `terraform-validate.yml` - validation, formatting, security checks run in parallel

**Example** (coverage.yml):
```yaml
jobs:
  backend-coverage:
    name: Backend Coverage
    runs-on: ubuntu-latest
    # Runs independently
  
  frontend-coverage:
    name: Frontend Coverage
    runs-on: ubuntu-latest
    # Runs in parallel with backend-coverage
```

### 5. Terraform Validation as Early Check

**What**: A fast-failing `terraform-validate.yml` workflow checks Terraform syntax before deployment workflows run.

**Impact**: Catches infrastructure-as-code errors in < 30 seconds, before long deployment jobs start.

**Workflow**: `terraform-validate.yml` (created in issue #2144)

**Checks**:
- `terraform validate` on all configurations (< 10 seconds)
- `terraform fmt -check` (< 5 seconds)
- Security scan for hardcoded secrets (< 10 seconds)
- Documentation completeness (< 5 seconds)

**Runs on**:
- Every push to branches with terraform/ changes
- Every pull request with terraform/ changes

### 6. Conditional Job Execution

**What**: Jobs only run when necessary based on file changes or environment conditions.

**Impact**: Skips unnecessary builds; e.g., don't build frontend when only backend files changed.

**Workflows**:
- `deploy.yml` - only runs when backend/ files change
- `frontend-build.yml` - only runs when frontend/ files change
- `terraform-validate.yml` - only runs when terraform/ files change

**Example**:
```yaml
on:
  push:
    paths:
      - 'backend/**'  # Only trigger on backend changes
      - '.github/workflows/deploy.yml'  # Or workflow changes
```

### 7. Performance Monitoring

**What**: Build times and metrics are tracked and reported in CI logs and step summaries.

**Impact**: Identifies performance regressions early; enables data-driven optimization decisions.

**Metrics tracked**:
- Frontend build time (target: < 2 minutes)
- Backend test time (including coverage calculation)
- Docker build time per layer
- Terraform validation time

**Example** (frontend-build.yml):
```yaml
- name: Build with timing
  id: build
  run: |
    START=$(date +%s%3N)
    pnpm build
    END=$(date +%s%3N)
    MS=$((END - START))
    echo "BUILD_MS=${MS}" >> "$GITHUB_ENV"

- name: Publish build metrics
  run: |
    cat >> "$GITHUB_STEP_SUMMARY" <<EOF
    ## Frontend Build Metrics
    | Metric | Value |
    | **Build time** | \`${{ env.BUILD_TIME }}\` |
    EOF
```

## Workflow Dependency Map

```
pull_request or push to main
  ├── terraform-validate.yml (runs if terraform/ changed)
  │   ├── terraform validate (10s)
  │   ├── terraform fmt check (5s)
  │   ├── security scan (10s)
  │   └── documentation check (5s)
  │
  ├── frontend-build.yml (runs if frontend/ changed, in parallel)
  │   ├── Setup Node + cache (10s)
  │   ├── pnpm install (5s on cache hit, 60s first run)
  │   └── pnpm build (30-120s depending on changes)
  │
  ├── coverage.yml (runs if tests needed, in parallel)
  │   ├── backend-coverage (Rust)
  │   │   ├── Setup Rust + cache (20s)
  │   │   ├── cargo build (30s on cache hit)
  │   │   └── cargo llvm-cov (120s)
  │   │
  │   └── frontend-coverage (Vitest)
  │       ├── Setup Node + cache (10s)
  │       ├── npm install (5s on cache hit)
  │       └── vitest (60s)
  │
  ├── security.yml (in parallel)
  │   ├── CodeQL analysis
  │   ├── Credential scanning
  │   └── Secret scanning
  │
  └── deploy.yml (runs on main, if backend/ changed)
      ├── Configure AWS (15s)
      ├── Docker build + push (60-120s)
      ├── Update ECS task definition (10s)
      ├── Create CodeDeploy deployment (20s)
      └── Wait for deployment (5-15m depending on health checks)
```

**Total CI time estimate**:
- **Pull request**: 5-8 minutes (parallel jobs)
- **Main branch push**: 10-15 minutes (includes deploy)
- **Cache misses**: Can double these times (first install is slower)

## Cache Hit Rates

Current estimated cache hit rates by workflow:

| Workflow | Cache Type | Hit Rate | Benefit |
|---|---|---|---|
| frontend-build | pnpm + Next.js | 80% | 2-3 min → 30-60 sec |
| backend coverage | cargo | 75% | 3-5 min → 30-60 sec |
| deploy | Docker layers | 60% | 5 min → 1-2 min |
| terraform-validate | none (fast) | N/A | 30-40 sec total |

**How to improve hit rates**:
- Avoid frequent dependency updates (batch them)
- Keep lock files consistent across environments
- Don't commit yarn.lock/package.lock alongside pnpm-lock.yaml
- Pin specific versions in Dockerfile RUN steps

## Performance Targets & SLOs

### Frontend Build

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| pnpm install | < 30s (cache hit) | 5-10s | ✅ |
| Next.js build | < 2m | 30-90s | ✅ |
| Total job time | < 3m | 1-2m (cache hit) | ✅ |

### Backend Coverage

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| cargo build | < 1m (cache hit) | 30-60s | ✅ |
| llvm-cov tests | < 3m | 1.5-2.5m | ✅ |
| Total job time | < 5m | 2-3m (cache hit) | ✅ |

### Terraform Validation

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| validate | < 30s | 10-15s | ✅ |
| fmt check | < 10s | 5-10s | ✅ |
| security scan | < 20s | 10-15s | ✅ |
| Total job time | < 2m | 30-60s | ✅ |

### Deployment (Blue-Green)

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Docker build | < 2m (cache hit) | 60-120s | ✅ |
| Register task def | < 30s | 10-20s | ✅ |
| CodeDeploy wait | < 15m | 5-10m | ✅ |
| Total time | < 20m | 10-15m (cache hit) | ✅ |

## Recent Optimizations

### Commit: perf: optimize CI/CD pipeline (#2145)

**Changes**:
1. Fixed `frontend-build.yml` malformed job definition and duplicate steps
2. Consolidated build timing logic into single step
3. Added pnpm + Next.js caching (was missing from frontend-build)
4. Improved build metrics reporting in step summary
5. Added warning for builds exceeding 2-minute target
6. Wired Terraform validation into CI via `terraform-validate.yml`
7. Documented all optimization strategies and cache hit rates

**Expected improvements**:
- Frontend builds: 15-30% faster on cache hits (pnpm + Next.js cache)
- Overall pipeline: 10-15% faster (parallel Terraform validation)
- Build reliability: Improved (fixed YAML syntax errors in frontend-build)

## Maintenance & Monitoring

### Weekly Checks

```bash
# Monitor cache hit rates
gh run list --workflow=frontend-build.yml --limit=10 --json conclusion,name,duration

# Check for performance regressions
gh run view $(gh run list --workflow=frontend-build.yml --limit=1 --json databaseId | jq -r '.[]')
```

### Monthly Optimization Review

1. **Review build times** in GitHub Actions
   - Frontend: target 30-90s on cache hit
   - Backend: target 2-3m on cache hit
   - Deploy: target 10-15m total

2. **Audit cache hit rates**
   ```bash
   gh api repos/Ndifreke000/stellar-insights/actions/caches --jq '.caches | length'
   ```

3. **Profile slow steps**
   - Look for steps taking > 30 seconds
   - Check for redundant installs or builds
   - Consider splitting large jobs

4. **Update lock files** (if safe)
   - Periodic dependency updates can invalidate caches
   - Plan large updates during low-traffic periods

### Cache Eviction

GitHub Actions caches automatically evict after 7 days of non-use. To manually clear:

```bash
# List all caches
gh api repos/Ndifreke000/stellar-insights/actions/caches --jq '.caches[] | .id'

# Delete specific cache
gh api repos/Ndifreke000/stellar-insights/actions/caches/{cache-id} -X DELETE
```

## Future Optimization Opportunities

1. **Test splitting & sharding**
   - Run different test suites in parallel jobs
   - Reduces backend coverage time from 2-3m to 1-1.5m

2. **Incremental builds**
   - Skip unchanged packages in monorepo builds
   - Use build graph to only run affected tests

3. **Layer-aware Docker caching**
   - Reorder Dockerfile to put frequently-changing code at end
   - Saves 20-30% on Docker build time

4. **Scheduled cache warmup**
   - Run builds on schedule (e.g., hourly) to keep caches warm
   - Ensures cache hits for morning CI runs

5. **Performance budget enforcement**
   - Fail builds if any job exceeds time budget
   - Automated alerts for regressions

6. **Cost optimization**
   - GitHub Actions provides 2000 free build minutes/month for open source
   - Optimizations reduce cost for private repos (currently $0.008/minute for runners)

## Related Documentation

- [GitHub Actions Caching](https://docs.github.com/en/actions/using-workflows/caching-dependencies-to-speed-up-workflows)
- [Docker Build Caching](https://docs.docker.com/build/cache/)
- [Terraform Best Practices](terraform/README.md)
- [Disaster Recovery Plan](disaster-recovery.md)
- [Backup System](backup-system.md)

## Troubleshooting

### Cache not being used

**Symptom**: Job takes full build time despite cache existing

**Cause**: Cache key mismatch (e.g., lockfile hash changed)

**Solution**:
```bash
# Check if cache was saved
gh run view <run-id> --json jobs | jq '.jobs[] | select(.name | contains("Save")) | .conclusion'

# Manually clear cache and retry
gh api repos/Ndifreke000/stellar-insights/actions/caches --jq '.caches[] | .id' | xargs -I {} gh api repos/Ndifreke000/stellar-insights/actions/caches/{} -X DELETE
```

### Build performance regressed

**Symptom**: Frontend build went from 60s to 120s

**Cause**: Dependency or code change affecting build time

**Solution**:
1. Check `frontend/pnpm-lock.yaml` diff in PR
2. Review code changes in `frontend/src/`
3. Run local build: `cd frontend && pnpm build --timing`
4. Profile with: `pnpm build --profile` (generates .pnpm-profile)

### Terraform validation failing unexpectedly

**Symptom**: `terraform validate` passes locally but fails in CI

**Cause**: Backend configuration mismatch (CI uses `-backend=false`)

**Solution**:
```bash
# CI runs: terraform init -backend=false && terraform validate
# Match this locally:
terraform init -backend=false
terraform validate
```

## Contributing

When optimizing CI:
1. Measure baseline (current job time)
2. Make change (add cache, parallelize, etc.)
3. Measure improvement (new job time)
4. Document in this file
5. Update GitHub Actions workflows
6. Open PR with before/after times

Example:
```markdown
### Optimization: Add Docker layer caching

**Baseline**: Docker build 5m 30s
**After**: Docker build 1m 20s
**Improvement**: 76% faster

**Change**:
- Added `cache-from: type=gha` and `cache-to: type=gha,mode=max` to build-push-action
- Caches all Docker layers across runs

**Trade-off**: 2-3 min upload time on first push (one-time cost)
```
