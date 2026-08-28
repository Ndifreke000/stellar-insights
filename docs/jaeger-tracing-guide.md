# Jaeger Tracing End-to-End Verification Guide

Closes #1861 — confirms that traces produced by the backend actually reach the
Jaeger collector and appear in the UI, and that trace context propagates across
outbound Stellar RPC / Horizon calls.

## Architecture overview

```
Browser / curl
  │  traceparent header (W3C TraceContext)
  ▼
Axum middleware stack
  ├─ TraceLayer (tower-http)  ← creates root span per request
  ├─ trace_propagation_middleware  ← extracts remote context, stamps trace_id/span_id
  └─ your handler
       └─ inject_trace_context(client.get(horizon_url))  ← propagates context outbound
            │  traceparent header forwarded to Horizon / Stellar RPC
            ▼
         Stellar RPC / Horizon (external; traces end here unless they support OTLP)

OpenTelemetry SDK
  └─ OTLP HTTP exporter → http://localhost:4318/v1/traces → Jaeger collector
```

## Step 1 — Start Jaeger

```bash
docker compose -f docker-compose.jaeger.yml up -d
# Wait for healthy:
docker compose -f docker-compose.jaeger.yml ps
```

Jaeger UI is available at http://localhost:16686.

## Step 2 — Start the backend with OTEL enabled

```bash
cd backend
cp .env.example .env
# Enable tracing and point at the local Jaeger instance:
echo "OTEL_ENABLED=true" >> .env
echo "OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318/v1/traces" >> .env
echo "RPC_MOCK_MODE=true" >> .env   # avoids needing live Stellar RPC

cargo run
```

The startup log will include:

```
INFO OpenTelemetry tracing enabled
```

## Step 3 — Make a few requests

```bash
curl http://localhost:8080/health
curl http://localhost:8080/api/corridors?limit=5
curl http://localhost:8080/api/anchors?limit=5
```

## Step 4 — Verify traces in the Jaeger UI

1. Open http://localhost:16686
2. In the **Service** dropdown select `stellar-insights-backend`
3. Click **Find Traces**
4. You should see one trace per HTTP request.  Each trace has at minimum:
   - A root span named after the HTTP route (e.g. `GET /api/corridors`)
   - Child spans for database queries (SQLx creates child spans automatically
     when the `tracing` feature is enabled)

If no traces appear:
- Confirm `OTEL_ENABLED=true` is set in `.env` (default in `.env.example` is `false`)
- Confirm `OTEL_EXPORTER_OTLP_ENDPOINT` points to port **4318** (HTTP), not 4317 (gRPC)
- Check backend logs for `OpenTelemetry tracing enabled` on startup
- The OTLP exporter uses a **batch processor** — there is up to ~5 s delay before
  spans are flushed; wait a moment and refresh

## Step 5 — Verify trace context propagation to outbound calls

The `inject_trace_context` helper in `backend/src/observability/tracing.rs`
injects `traceparent` / `tracestate` headers into every outbound `reqwest`
request, so Stellar RPC / Horizon calls carry the same trace ID as the
originating HTTP request.

To confirm propagation is wired up:

```bash
# Enable debug logging so outbound headers are visible
RUST_LOG=backend=debug,reqwest=debug cargo run
```

Then make a request that triggers an RPC call:

```bash
curl http://localhost:8080/api/rpc/ledger/latest
```

In the backend logs you should see outbound request lines that include a
`traceparent` header such as:

```
DEBUG reqwest: ... traceparent: "00-<trace-id>-<span-id>-01"
```

In the Jaeger UI the root span for `GET /api/rpc/ledger/latest` will have a
child span representing the outbound HTTP call.  The `traceparent` header
forwarded to Horizon means that if Horizon were an OTLP-instrumented service,
its spans would appear as children of the same trace.

## Configuration reference

| Env var | Default | Description |
|---|---|---|
| `OTEL_ENABLED` | `false` | Set `true` to activate OTLP export |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318/v1/traces` | OTLP HTTP endpoint |

Both variables are documented in `backend/.env.example` under
**Observability (OpenTelemetry / Jaeger)**.

## Shutting down

```bash
docker compose -f docker-compose.jaeger.yml down
```

Jaeger uses in-memory storage by default; all traces are lost on restart.
For persistent storage mount a Badger or Cassandra volume — see the
[Jaeger deployment docs](https://www.jaegertracing.io/docs/deployment/).
