# API Service Level Objectives

Canonical targets for API reliability and latency. These are the thresholds
that the alert rules in [`monitoring/prometheus-alert-rules.yaml`](../monitoring/prometheus-alert-rules.yaml)
are built to catch violations of; see
[DEPLOYMENT_SLOS_AND_ALERTING.md](./DEPLOYMENT_SLOS_AND_ALERTING.md) for how
to deploy those rules. This document is the single place to look up *what*
the targets are, independent of how they're enforced.

## Overall targets

| Objective | Target |
|---|---|
| Availability (successful requests / total requests) | 99.9% per 30-day window |
| Global error rate | < 1% (warning), < 5% (critical incident) |
| Health check (`/health`) latency | p99 < 500ms |

## Per-endpoint targets

| Endpoint | Error rate | Latency |
|---|---|---|
| `/anchors` | < 0.5% (warning), < 2% (critical) | p95 < 5s, p99 < 10s (critical) |
| `/corridors` | < 0.5% (warning), < 2% (critical) | p95 < 5s, p99 < 10s (critical) |

## Dependency targets

| Dependency | Objective |
|---|---|
| Stellar RPC calls | error rate < 1% (warning), < 5% (critical) |
| Circuit breaker trips | < 0.1/s sustained |
| Database connection pool | < 90% utilization |
| Database query latency | p95 < 1s |
| Cache hit rate | ≥ 50% |
| Background jobs | failure rate < 5%; queue depth < 1000 |

## Measuring compliance

All of the above are backed by Prometheus metrics already exported by the
backend (`http_errors_total`, request-duration histograms, `job_*` metrics
from [`job_metrics.rs`](../backend/src/observability/job_metrics.rs), etc.).
Grafana dashboards for these live under [`docs/grafana`](./grafana)
(`observability-dashboard.json` covers testnet/mainnet in one view) — import
one to visualize current compliance against the targets in this document.

When adding a new endpoint or background job, add its targets to the tables
above and a matching alert rule in `prometheus-alert-rules.yaml` in the same
change, so the two never drift apart.
