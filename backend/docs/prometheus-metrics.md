# Prometheus Metrics

This document describes the Prometheus metrics exposed by the Stellar Insights backend.

## Endpoint

The metrics are available at `/metrics` in Prometheus text format.

## Available Metrics

### HTTP Metrics
- `http_requests_total` - Total number of HTTP requests processed
- `http_request_duration_seconds` - HTTP request duration in seconds (histogram)
- `http_errors_total` - Total number of HTTP errors by status code, method, and path
- `http_in_flight_requests` - Number of in-flight HTTP requests

### Database Metrics
- `db_query_duration_seconds` - Database query duration in seconds (histogram)
- `db_pool_size` - Total database pool connections
- `db_pool_idle` - Idle database pool connections
- `db_pool_active` - Active database pool connections
- `db_errors_total` - Total number of database errors by type and query type

### Cache Metrics
- `cache_operations_total` - Total number of cache operations
- `cache_hits_total` - Total number of cache hits
- `cache_misses_total` - Total number of cache misses

### RPC Metrics
- `rpc_calls_total` - Total number of RPC calls made
- `rpc_call_duration_seconds` - RPC call duration in seconds (histogram)
- `rpc_errors_total` - Total number of RPC errors by method and error type

### Application Metrics
- `errors_total` - Total number of errors encountered
- `background_jobs_total` - Total number of background jobs executed
- `active_connections` - Number of active websocket connections
- `corridors_tracked` - Number of tracked corridors

## Usage in Prometheus

Add the following to your `prometheus.yml` configuration:

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

## Grafana Dashboard

A sample Grafana dashboard configuration can be found in `monitoring/grafana-dashboard.json`.

## Implementation Details

The metrics are implemented using the `prometheus` Rust crate and are automatically collected through middleware and instrumentation throughout the application.

Key components:
- `observability/metrics.rs` - Core metrics definitions and collection
- `api/metrics.rs` - HTTP endpoint for metrics exposure
- HTTP middleware automatically tracks request latency and errors
- Database pool metrics are updated every 30 seconds
- Cache metrics are updated on each cache operation
