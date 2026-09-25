# Distributed Tracing Context Propagation — Verification Guide

## Overview

The Stellar Insights backend implements W3C TraceContext propagation for distributed tracing across service boundaries using OpenTelemetry and the `tracing` crate.

## Architecture

### Trace Context Flow

1. **Inbound**: `trace_propagation_middleware` extracts W3C `traceparent` and `tracestate` headers from incoming HTTP requests
2. **Processing**: `TraceIdLayer` stamps `trace_id` and `span_id` onto every span, so all log events carry trace IDs
3. **Outbound**: `inject_trace_context()` injects current trace context into outbound `reqwest` requests

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `OTEL_ENABLED` | `true` | Enable/disable OpenTelemetry |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318/v1/traces` | OTLP endpoint |
| `LOG_FORMAT` | `json` | Log format |
| `LOG_DIR` | (none) | Rotating log directory |

## Verifying with Jaeger

### 1. Start Jaeger
```bash
docker run -d --name jaeger -p 16686:16686 -p 4318:4318 jaegertracing/all-in-one:latest
```

### 2. Start Backend
```bash
OTEL_ENABLED=true OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318/v1/traces cargo run
```

### 3. Verify
- Open http://localhost:16686
- Select `payraider-backend` service
- Each request creates a trace with unique `trace_id`
- Downstream calls share the same `trace_id`
- Span durations are recorded

## Tests

Comprehensive propagation tests in `backend/tests/observability/tracing_propagation_tests.rs` cover:
- Inbound traceparent extraction
- Malformed header handling
- Outbound trace context injection
- End-to-end multi-hop propagation
- Tracestate propagation
- Multi-hop trace ID consistency
