# Development Branch: feature/refactor-dependency-injection-and-documentation

This branch contains the implemented changes for the following issues:

## Issues Addressed

1. **P2-2455**: Dependency Injection Pattern for RealtimeBroadcaster
2. **P2-2367**: Duplicate ELK Configuration Cleanup
3. **P2-2369**: Rate Limiter and Cache Configuration Documentation
4. **P2-2368**: IAM Trust Policy Fix for Backup Workflow

## Branch Details

- **Branch Name**: `feature/refactor-dependency-injection-and-documentation`
- **Remote**: `origin`
- **Repository**: `Hydrax117/stellar-insights`

## Changes Summary

### P2-2455: Dependency Injection Pattern

**Files Changed**:
- `backend/src/main.rs` - Updated to use ServiceContainer for all service construction
- `backend/src/services/service_container.rs` - New service container implementation

**Key Changes**:
- Added `RealtimeBroadcaster` to `ServiceContainer` with trait-based dependencies
- Implemented `RealtimeBroadcaster::new()` accepting `Arc<dyn DataPort>` and `Arc<WebhookEventService>`
- Updated `main.rs` to extract services from container instead of inline instantiation
- Added RealtimeBroadcaster background task with graceful shutdown handling

### P2-2367: Configuration Cleanup

**Files Changed**:
- `backend/.env.example`

**Key Changes**:
- Removed duplicate commented-out backup configuration section
- Consolidated backup settings into single canonical section

### P2-2369: Configuration Documentation

**Files Changed**:
- `backend/.env.example`

**Key Changes**:
- Added comprehensive Redis configuration documentation
- Documented rate limiter settings with descriptions and defaults:
  - `RPC_RATE_LIMIT_REQUESTS_PER_MINUTE` (default: 90)
  - `RPC_RATE_LIMIT_BURST_SIZE` (default: 10)
  - `RPC_RATE_LIMIT_QUEUE_SIZE` (default: 100)
- Documented cache TTL settings with descriptions and defaults:
  - `CACHE_CORRIDOR_METRICS_TTL` (default: 300s = 5 min)
  - `CACHE_ANCHOR_DATA_TTL` (default: 600s = 10 min)
  - `CACHE_DASHBOARD_STATS_TTL` (default: 60s = 1 min)

### P2-2368: IAM Trust Policy Fix

**Files Changed**:
- `terraform/global/backups.tf`

**Key Changes**:
- Changed OIDC subject from `repo:Ndifreke000/payraider:ref:refs/heads/main` to `repo:Ndifreke000/stellar-insights:*`
- Switched from `StringEquals` to `StringLike` for workflow_dispatch support

## Pull Request Markdown

A PR documentation file has been created at:
- `.github/pr_2455_2367_2369_2368.md`

This file contains detailed information about all changes, testing notes, and related issues.

## How to Apply These Changes

1. Checkout the branch:
   ```bash
   git fetch origin
   git checkout feature/refactor-dependency-injection-and-documentation
   ```

2. Build the backend:
   ```bash
   cd backend
   cargo check
   cargo build
   ```

3. Apply Terraform changes (if applicable):
   ```bash
   cd terraform/global
   terraform plan
   terraform apply
   ```

## Testing

All changes have been tested for:
- Rust compilation (`cargo check`)
- ServiceContainer service instantiation
- Terraform configuration syntax

## Related Documentation

- `.github/pr_2455_2367_2369_2368.md` - Full PR documentation
- `docs/backup-system.md` - Backup system documentation (referenced in P2-2368)
