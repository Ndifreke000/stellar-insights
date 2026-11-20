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


