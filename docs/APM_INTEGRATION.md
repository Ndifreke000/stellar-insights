# Application Performance Monitoring (APM) Integration

Stellar Insights integrates OpenTelemetry for distributed tracing and APM. This enables deep visibility into application performance, database queries, and inter-service communication.

## Overview

The APM system collects:

- **Request traces**: HTTP request → response spans with latency
- **Database spans**: SQL query execution with timing
- **Error events**: Exceptions and error-level logs
- **Service dependencies**: Calls to downstream services
- **Correlation IDs**: Trace requests across services via `traceparent` (W3C standard)

Traces are exported via OTLP (OpenTelemetry Protocol) to your chosen backend.

## Architecture

```
┌─────────────────────────────────┐
│  Backend Application            │
│  (Rust + tracing + OpenTelemetry)
└────────────┬────────────────────┘
             │ OTLP/HTTP (gRPC or JSON)
             │ :4318 (default)
             ▼
   ┌──────────────────────┐
   │  APM Collector       │
   │  • Jaeger (local)    │
   │  • New Relic (prod)  │
   │  • Datadog (prod)    │
   └──────────────────────┘
             │
             ▼
   ┌──────────────────────┐
   │  APM UI / Analytics  │
   │  • Trace views       │
   │  • Service map       │
   │  • Error tracking    │
   └──────────────────────┘
```

## Quick Start

### Enable APM (Default)

APM is **enabled by default** via OpenTelemetry integration. Set these environment variables:

```bash
# Enable APM tracing (default: true)
export OTEL_ENABLED=true

# Service name for traces
export OTEL_SERVICE_NAME=stellar-insights-backend

# OTLP endpoint (default: localhost:4318/v1/traces for Jaeger)
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318/v1/traces
```

### Disable APM

To disable telemetry (e.g., in tests):

```bash
export OTEL_ENABLED=false
```

When disabled, the app uses a no-op tracer with zero performance impact.

## Choosing an APM Backend

### Option 1: Jaeger (Local Development) — Default

**Why Jaeger?**
- Free, open-source
- Works with OTLP natively
- Good for local development
- Sufficient for staging environments

**Setup (Docker):**

```bash
# Start Jaeger all-in-one
docker run -d \
  --name jaeger \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 4317:4317 \
  -p 4318:4318 \
  -p 16686:16686 \
  jaegertracing/all-in-one

# Access UI at http://localhost:16686
```

**Environment variables:**

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318/v1/traces
export OTEL_SERVICE_NAME=stellar-insights-backend
```

### Option 2: New Relic (Production)

**Why New Relic?**
- Hosted APM with full-featured UI
- Superior alerting and anomaly detection
- Integrated error tracking
- Production-grade SLAs

**Setup:**

1. Create New Relic account (nrelic.com)
2. Get your License Key from Settings
3. Set environment variables:

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp.nr-data.net:4318
export OTEL_EXPORTER_OTLP_HEADERS="api-key=YOUR_LICENSE_KEY"
export OTEL_SERVICE_NAME=stellar-insights-backend
export OTEL_ENABLED=true
```

4. Verify traces appear in New Relic UI:
   - Go to APM → Services → stellar-insights-backend

### Option 3: Datadog (Production)

**Why Datadog?**
- Integrated logs + metrics + traces
- Advanced analytics
- Great for multi-service environments
- Excellent for security compliance

**Setup:**

1. Create Datadog account (datadoghq.com)
2. Get your API key from Settings
3. Set environment variables:

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export DD_AGENT_HOST=datadog-agent
export DD_TRACE_AGENT_PORT=8126
export OTEL_SERVICE_NAME=stellar-insights-backend
export DD_TAGS="env:production,version:1.0.0"
```

4. Ensure Datadog agent is running:

```bash
docker run -d \
  --name datadog-agent \
  --hostname datadog-agent \
  -e DD_API_KEY=YOUR_API_KEY \
  -e DD_SITE=datadoghq.com \
  -p 8126:8126 \
  datadog/agent
```

## Instrumentation Details

### Request/Response Spans

Every HTTP request is traced:

```
┌─ Request Received ──────────────────────┐
│ Span Name: GET /api/v1/anchors           │
│ Attributes:                             │
│  - http.method: GET                     │
│  - http.url: /api/v1/anchors            │
│  - http.status_code: 200                │
│  - http.duration_ms: 45                 │
│  - request.id: 550e8400-e29b...         │
│  - user_id: (if authenticated)          │
└─────────────────────────────────────────┘
      │
      ├─ Child Span: Database Query 1
      │  └─ SELECT * FROM anchors LIMIT 10
      │
      ├─ Child Span: Cache Lookup
      │  └─ GET anchors:all
      │
      └─ Child Span: Response JSON Encoding
         └─ 2ms encoding time
```

### Database Query Spans

Database queries (via sqlx) include:

```json
{
  "span_name": "db.query",
  "duration_ms": 12,
  "db.statement": "SELECT * FROM anchors WHERE status = $1",
  "db.operation": "SELECT",
  "db.rows_affected": 5,
  "otel.status_code": "OK"
}
```

### Error Tracking

Errors automatically create error spans:

```json
{
  "span_name": "request",
  "error": true,
  "exception.type": "DatabaseError",
  "exception.message": "connection timeout",
  "http.status_code": 500,
  "otel.status_code": "ERROR",
  "otel.status_description": "connection timeout"
}
```

## Correlation: Traces ↔ Logs

Every trace is linked to logs via `request_id` and `traceparent` (W3C):

**Request header (automatically generated):**
```
traceparent: 00-550e8400e29b41d4a716446655440000-b9c7c989f97918e1-01
```

**Log entry (from Elasticsearch):**
```json
{
  "@timestamp": "2026-08-25T14:30:45.123Z",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "traceparent": "00-550e8400e29b41d4a716446655440000-...",
  "level": "INFO",
  "message": "→ request",
  "path": "/api/v1/anchors"
}
```

**Workflow:**
1. Request arrives → `request_id` generated
2. Trace span created with same `request_id`
3. Log entry includes `request_id`
4. In APM UI: Click trace → Jump to logs for same request ID
5. In Kibana: Filter `request_id: X` → See all logs for that trace

## Sensitive Data Handling

### Automatic Redaction

The APM module redacts sensitive span attributes:

```rust
// In src/observability/apm.rs
pub mod redaction {
    pub fn redact_value(value: &str) -> String {
        // Redacts authorization, passwords, tokens, secrets
        // Returns "REDACTED" if sensitive keyword found
    }
}
```

**Redacted keywords:**
- authorization, bearer
- password
- api_key, api-key, apikey
- token
- secret
- credential
- auth

### Best Practices

1. **Never capture user PII in span attributes**:
   ```rust
   // ❌ DON'T
   span.set_attribute("user_email", email);
   
   // ✓ DO
   span.set_attribute("user_id", user_id);
   ```

2. **Redact at the source**:
   ```rust
   // ✓ DO
   let safe_value = apm::redaction::redact_value(&sensitive);
   span.set_attribute("field", safe_value);
   ```

3. **Exclude sensitive headers**:
   ```rust
   // Request/response logging already excludes:
   // - Authorization
   // - Cookie
   // - Set-Cookie
   ```

4. **Limit body capture**:
   ```bash
   # Only capture request/response bodies if explicitly enabled:
   export API_LOG_BODIES=true
   
   # Bodies are truncated to 512 bytes and redacted
   ```

## Monitoring and Alerts

### Key Metrics to Track

| Metric | Healthy | Warning | Critical |
| --- | --- | --- | --- |
| P95 latency | < 500ms | 500-1000ms | > 1000ms |
| Error rate | < 1% | 1-5% | > 5% |
| Apdex score | > 0.95 | 0.85-0.95 | < 0.85 |
| Trace volume | > 100/s | 10-100/s | < 10/s |

### Setting Up Alerts

**New Relic:**
```bash
# Alert: High error rate
# Navigate to Alerts & AI → Alert Conditions → New Alert
# Condition: Error Rate > 5% for 5 minutes
# Notification: Slack, PagerDuty, Email
```

**Datadog:**
```yaml
# In terraform or alerting config
alert:
  name: "Stellar Insights High Error Rate"
  query: |
    avg:trace.web.request.errors{service:stellar-insights-backend} > 0.05
  thresholds:
    critical: 0.05
    warning: 0.02
  notify:
    - "@pagerduty"
```

**Jaeger (Local):**
- No built-in alerting (use Prometheus instead)
- Monitor Jaeger service health manually or via external tool

## Production Deployment

### Environment Configuration

```bash
# For production New Relic:
export OTEL_ENABLED=true
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp.nr-data.net:4318
export OTEL_EXPORTER_OTLP_HEADERS="api-key=${NEW_RELIC_LICENSE_KEY}"
export OTEL_SERVICE_NAME=stellar-insights-backend
export OTEL_RESOURCE_ATTRIBUTES="environment=production,pod.name=$(hostname)"
```

### High-Volume Trace Sampling

To reduce costs with high request volumes, implement sampling:

```rust
// In observability/tracing.rs
// Sample 10% of traces in production
let sampler = opentelemetry_sdk::trace::Sampler::AlwaysOff; // Default
// Change to:
let sampler = opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(0.1);
```

### Storage and Retention

**New Relic retention:**
- Standard: 30 days
- Can extend to 1 year (paid)

**Datadog retention:**
- 15 days (default)
- Adjustable in settings

**Jaeger retention:**
- Configure in collector (default: 24 hours)

## Troubleshooting

### Q: Traces not appearing?

**A:** Check these in order:
1. Verify `OTEL_ENABLED=true` in environment
2. Test OTLP endpoint connectivity: `curl -I http://localhost:4318/v1/traces`
3. Check application logs for OTEL errors: `grep -i "otel\|tracer" app.log`
4. Verify service name: `echo $OTEL_SERVICE_NAME`

### Q: High latency from APM instrumentation?

**A:**
1. Reduce trace sampling (see "Production Deployment" above)
2. Disable telemetry if not needed: `OTEL_ENABLED=false`
3. Check OTLP endpoint latency (network issue?)
4. Enable batch exporting (already done by default)

### Q: Memory usage increased after enabling APM?

**A:**
1. APM spans are batched (default: 512 spans per batch)
2. Batch export rate: every 5 seconds
3. If memory still high, reduce sample rate
4. Check if traces are being exported (no network backlog)

### Q: APM provider not receiving traces?

**A:**
1. **New Relic**: Verify API key in `OTEL_EXPORTER_OTLP_HEADERS`
2. **Datadog**: Verify agent is running and port 8126 is accessible
3. **Jaeger**: Verify collector is running (`docker ps | grep jaeger`)
4. All: Enable debug logging: `RUST_LOG=opentelemetry=debug`

## See Also

- [Logs Correlation via ELK](./ELK_LOG_AGGREGATION.md)
- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [OTLP Specification](https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md)
- [New Relic OTLP Ingestion](https://docs.newrelic.com/docs/more-integrations/open-source-telemetry-integrations/opentelemetry/opentelemetry-intro/)
- [Datadog OpenTelemetry](https://docs.datadoghq.com/opentelemetry/)

## Design Decisions

**APM Provider Selection:**
- **Default**: Jaeger (open-source, OTLP-native, local development)
- **Production**: New Relic or Datadog (team choice)
- **Rationale**: Jaeger for low cost/complexity locally, enterprise APM for production scale

**Fail-Soft Behavior:**
- APM errors never crash the application
- Missing OTLP endpoint → logs warning, continues with no-op tracer
- Bad APM configuration → logs warning, continues normally
- **Rationale**: APM is observability, not availability; must not impact user-facing reliability

**Correlation via Request ID:**
- W3C `traceparent` header for distributed traces
- Custom `request_id` field in logs and spans for cross-system correlation
- **Rationale**: Compatibility with standard tools + custom correlation for logs

**Sensitive Data Redaction:**
- Keyword-based redaction in APM module
- Header-based redaction in logging middleware
- Truncation of request/response bodies
- **Rationale**: Defense-in-depth; multiple layers reduce risk of PII leakage
