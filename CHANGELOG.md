# Changelog

All notable changes to the PayRaider project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

---

## [1.0.0] - 2024-01-01

### Added
- Initial v1 REST API for anchors, corridors, payments, and Stellar RPC data.
