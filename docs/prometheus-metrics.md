# Prometheus Metrics

This document describes the Prometheus metrics exposed by the Stellar Insights backend.

## Endpoint

The metrics are available at `/metrics` in Prometheus text format.

## Available Metrics

### HTTP Metrics
- `http_requests_total` - Total number of HTTP requests processed
- `http_request_duration_seconds` - HTTP request duration in seconds (histogram, p50/p95/p99 buckets)
- `http_request_duration_by_endpoint_seconds` - HTTP request duration per `method`×`endpoint` label pair (histogram)
- `http_errors_total` - Total number of HTTP errors by `status_code`, `method`, and `path`
- `http_in_flight_requests` - Number of in-flight HTTP requests (gauge)
- `http_request_slo_violations_total` - Requests exceeding the 500 ms SLO target, by `endpoint` and `slo_target_ms`
- `http_responses_compressed_total` - Responses sent with a `Content-Encoding` header (compression active)

### Database Metrics
- `db_query_duration_seconds` - Database query duration in seconds (histogram)
- `db_query_duration_by_operation_seconds` - Query duration per `operation`×`status` label pair (histogram)
- `db_pool_size` - Total database pool connections (gauge)
- `db_pool_idle` - Idle database pool connections (gauge)
- `db_pool_active` - Active database pool connections (gauge)
- `db_pool_connections_active` - Active connections (canonical gauge, kept in sync with `db_pool_active`)
- `db_pool_connections_idle` - Idle connections (canonical gauge, kept in sync with `db_pool_idle`)
- `db_pool_utilization` - Pool utilisation as an integer percentage (0–100, gauge)
- `db_pool_wait_time_seconds` - Time spent waiting for a pool connection (histogram)
- `db_pool_errors_total` - Pool errors by `kind` (`exhausted`, `near_exhaustion`, etc.)
- `db_errors_total` - Database errors by `error_type` and `query_type`
- `db_slow_queries_total` - Queries exceeding the slow-query threshold, by `operation`

### Cache Metrics
- `cache_operations_total` - Total number of cache operations
- `cache_hits_total` - Total number of cache hits
- `cache_misses_total` - Total number of cache misses

### RPC Metrics
- `rpc_calls_total` - Total number of RPC calls made
- `rpc_call_duration_seconds` - RPC call duration in seconds (histogram)
- `rpc_errors_total` - Total number of RPC errors by `method` and `error_type`

### Application / Background Metrics
- `errors_total` - Total number of errors encountered (all sources)
- `background_jobs_total` - Total number of background jobs executed
- `active_connections` - Number of active WebSocket connections (gauge)
- `corridors_tracked` - Number of tracked corridors (gauge)

### Backup Metrics
- `backup_verifications_total` - Backup verification attempts by `result` (`success` / failure reason)
- `backup_size_bytes` - Size of the most recent backup in bytes (gauge)

### Stellar-Specific Metrics
- `stellar_ledger_lag_seconds` - Ledger ingestion lag in seconds (gauge)
- `stellar_transaction_success_rate` - Rolling transaction success rate histogram (buckets: 0, 0.25, 0.5, 0.75, 1.0)
- `stellar_anchor_health` - Per-`anchor` health indicator: 1 = healthy, 0 = unhealthy (gauge)
- `stellar_corridor_reliability` - Per-`corridor` reliability histogram (rolling 24 h)
- `price_feed_stale_assets` - Per-`asset` oracle staleness indicator: 1 = stale, 0 = fresh (gauge)

## Usage in Prometheus

A ready-to-use `prometheus.yml` is committed at `monitoring/prometheus.yml`.
Point Prometheus at it directly:

```bash
prometheus --config.file=monitoring/prometheus.yml
```

Or add the following scrape job to an existing configuration:

```yaml
scrape_configs:
  - job_name: 'stellar-insights'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

## Important Metrics to Monitor

### High Priority Alerts
- `http_errors_total` rate > 0.01 - High HTTP error rate
- `db_pool_active` / `db_pool_size` > 0.9 - Database pool exhaustion
- `cache_misses_total` / `cache_operations_total` > 0.5 - High cache miss rate

### Medium Priority Alerts
- `http_request_duration_seconds` 95th percentile > 2s - Slow HTTP responses
- `db_query_duration_seconds` 95th percentile > 1s - Slow database queries
- `rpc_errors_total` rate > 0.005 - RPC errors

### Low Priority Alerts
- `active_connections` > 1000 - High WebSocket connection count
- `background_jobs_total` rate < 0.1 - Background jobs not running

## Grafana Dashboards

Three importable dashboard JSON files are committed in `docs/grafana/`:

| File | Purpose |
|---|---|
| `docs/grafana/observability-dashboard.json` | General backend observability (request rate, latency, cache, DB, errors) |
| `docs/grafana/testnet-dashboard.json` | Testnet-focused: TPS, API latency percentiles (p50/p95/p99), pool utilisation, WebSocket connections, RPC error rate, cache hit/miss ratio |
| `docs/grafana/mainnet-dashboard.json` | Same panels as testnet with mainnet-specific `job` label |

### Importing a dashboard

1. Open Grafana → **Dashboards → Import**.
2. Upload or paste the contents of any JSON file above.
3. Select your Prometheus data source.
4. Confirm the `job` variable matches your Prometheus scrape job name (default: `stellar-insights`).

All panels target metrics that are registered by `init_metrics()` and emitted by the HTTP
middleware, so they will populate against a live scrape target immediately — no "No data"
panels expected as long as the backend is receiving traffic.

## Implementation Details

The metrics are implemented using the `prometheus` Rust crate and are automatically collected through middleware and instrumentation throughout the application.

Key components:
- `observability/metrics.rs` - Core metrics definitions and collection
- `api/metrics.rs` - HTTP endpoint for metrics exposure
- HTTP middleware automatically tracks request latency and errors
- Database pool metrics are updated every 30 seconds
- Cache metrics are updated on each cache operation
