# Observability Setup

This backend now includes baseline production observability:

- Prometheus metrics endpoint at `/metrics`
- Structured JSON logging (default)
- Request ID propagation (`X-Request-ID`)
- OpenTelemetry tracing via OTLP
- Metrics for HTTP, RPC, cache, DB, websocket connections, and background jobs

## Metrics Endpoint

Run backend and scrape:

```bash
curl -s http://127.0.0.1:8080/metrics
```

Key metric families:

- `http_requests_total`
- `http_request_duration_seconds`
- `rpc_calls_total`
- `rpc_call_duration_seconds`
- `cache_operations_total`
- `db_query_duration_seconds`
- `background_jobs_total`
- `active_connections`
- `corridors_tracked`
- `errors_total`

## Logging

Default format is JSON.

- `LOG_FORMAT=json` (default)
- `LOG_FORMAT=pretty` for local human-readable logs

## OpenTelemetry Tracing

Enable OTLP export:

```bash
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
```

`service.name` is set to `stellar-insights-backend`.

## Request ID

Each request gets an `X-Request-ID` header:

- Reuses upstream `X-Request-ID` if provided
- Otherwise generates a UUID

The same request ID is included in response headers for cross-service debugging.

## Grafana

Example dashboard JSON is provided at:

- `docs/grafana/observability-dashboard.json`
