# Changelog

All notable changes to the PayRaider project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- **Asset Impersonation Detection (#2397)**:
  - New `AssetImpersonationDetector` with canonical issuer allowlist and lookalike domain detection.
  - Homoglyph detection (visual character substitution) and edit distance similarity comparison.
  - New risk levels: `low`, `medium`, `high`, `critical` in API responses.
  - Admin override workflow with audit log (`asset_verification_audit_log` table).
  - New database tables: `canonical_issuers`, `asset_impersonation_reports`.
  - New API endpoint: `/api/assets/verify/{code}/{issuer}` returns impersonation reports.
  - Impersonation risk exposure in `trust_indicators` with `has_impersonation_risk` field.
- **Webhooks Per-Endpoint FIFO Queue (#2396)**:
  - New `WebhookQueueManager` in `backend/src/webhooks/queue.rs` with per-endpoint FIFO ordering.
  - Parallel delivery across endpoints while maintaining strict ordering within each endpoint.
  - Configurable parallelism via `WEBHOOK_PARALLELISM` environment variable (default: 4).
  - Per-endpoint queues ensure state events arrive in order.
- **Auto-Disable on Sustained Failure (#2396)**:
  - Webhooks auto-disable after configurable consecutive failures (default: 5).
  - New database columns: `consecutive_failures`, `max_consecutive_failures`, `disabled_reason`, `disabled_at`, `enabled_at`.
  - Auto-disable API endpoint at `/api/v1/webhooks/{id}/enable` to re-enable webhooks.
  - Logging of auto-disable events for monitoring.
- **Versioned Payloads (#2396)**:
  - All webhook envelopes include `event_version` field (current: "v1").
  - Backwards-compatible versioning strategy for future extensions.
  - `WebhookEventEnvelope::EVENT_VERSION` constant for type-safe version handling.
- **Rotatable Secrets (#2396)**:
  - Support for secret rotation with overlap period (primary + secondary secrets).
  - New signature methods: `WebhookSignature::sign_with_rotation()`, `verify_with_rotation()`.
  - Rotation period configurable per webhook (default: 30 days).
  - Secret rotation API endpoint at `/api/v1/webhooks/{id}/rotate-secrets`.
  - New database columns: `secret_rotation_enabled`, `secondary_secret`, `secret_rotation_period_days`, `next_rotation_date`.
- **Delivery Logs API (#2396)**:
  - New `delivery_logs` table with complete audit trail of all delivery attempts.
  - New API endpoint: `GET /api/v1/webhooks/{id}/delivery-logs` with pagination.
  - New database columns: `event_version`, `signature`, `last_error`, `created_at`.
- **Webhooks Documentation (#2396)**:
  - Comprehensive webhook documentation in `docs/WEBHOOKS.md`.
  - Event payload format, signature verification, error handling, and security best practices.
  - Migration guide for existing webhooks.

---


## [Unreleased]

### Added
- **API Response Time & Latency Observability (#2392)**:
  - Histogram metrics `http_request_duration_seconds` and `http_request_duration_by_endpoint_seconds` with comprehensive latency buckets covering p50, p95, and p99 ranges.
  - Automated SLO violation tracking (`http_request_slo_violations_total`) for requests exceeding 500ms target.
  - Dedicated Grafana dashboard `docs/grafana/api-latency-dashboard.json` for response times, p50/p95/p99 quantiles, and slowest endpoint rankings.
  - Prometheus alerting rules for `ApiLatencyP95Exceeded`, `ApiLatencyP99Exceeded`, and `ApiSloViolationRateHigh`.
- **Enterprise Secrets Management Solution (#2393)**:
  - External Secrets Operator configurations in `k8s/secrets/` supporting HashiCorp Vault (`vault-secrets-store.yaml`) and AWS Secrets Manager (`aws-secrets-manager.yaml`).
  - Production Kubernetes secret template with strict RBAC least-privilege reading rules (`kubernetes-secrets.yaml`).
  - Automated 90-day secret rotation policy implementation in `scripts/rotate-vault-secrets.sh` covering JWT secrets, encryption keys, and API keys.
  - Vault and AWS audit logging documentation and verification guidelines.
- **Frontend Error Tracking & Sentry Integration (#2394)**:
  - Wrapped Next.js build configuration with `withSentryConfig` in `frontend/next.config.ts` for automated source map uploads, release tracking, and performance monitoring.
  - Enhanced `frontend/src/lib/logger.ts` with user context management (`setUserContext`, `clearUserContext`), breadcrumbs buffering (`addBreadcrumb`, `getBreadcrumbs`), and `captureException`.
  - Configured 100% error sample rate and release tracking across client and server Sentry configs.
- **API Versioning Strategy & Deprecation Policy (#2395)**:
  - Documented API versioning strategy, 6-month deprecation policy, and v1 → v2 migration guide in `docs/API_VERSIONING.md`.
  - Content negotiation support via `Accept: application/vnd.payraider.v2+json` and `v1+json`.
  - RFC 7231 & RFC 8594 standard deprecation headers (`Deprecation`, `Sunset`, `Link`, `Warning`) emitted on v1 endpoints with target sunset date of 2026-12-31.
- **Cost-Based Rate Limiting for Export Endpoints (#2669)**:
  - Cost calculation model in `backend/src/models/cost.rs` with configurable cost units.
  - Cost rate limiter middleware in `backend/src/middleware/cost_rate_limiter.rs` tracking budgets per tier.
  - Daily budget allocation: 100 units (anonymous), 1,000 units (authenticated), 10,000 units (premium).
  - Cost estimate headers in export responses: `X-Cost-Estimate`, `X-Remaining-Budget`, `X-Total-Budget`, `X-Tier`, `X-Budget-Refresh-At`.
  - 429 Too Many Requests with `Retry-After: 86400` when budget exhausted.
  - Example costs: single corridor 30-day export = 40 units, all corridors = ~10,000 units, analytics dashboard = 5 units.
## [Unreleased]

### Added
- **Non-blocking Retention System**:
  - Batched deletion with configurable batch size and sleep intervals to avoid SQLite lock contention
  - Incremental VACUUM support to reclaim space without extended write locks
  - Automatic rollup to corridor metrics before deletion, preserving analytics beyond raw data retention
  - Environment variables for fine-grained configuration: `JOB_RETENTION_ENABLED`, `RETENTION_BATCH_SIZE`, `RETENTION_SLEEP_MS`, `VACUUM_ENABLED`, etc.
  - Configurable retention per table with `TableRetention` settings
  - Monitoring metrics for retention duration, rows deleted, batches processed, and space reclaimed
