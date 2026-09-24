# API Request Logging

Every HTTP request gets one structured access-log event. Every log line emitted while handling the
request carries the same `request_id` and `correlation_id`.

Code: `backend/src/request_id.rs` (IDs) and `backend/src/observability/logging.rs` (access log).

## Request and correlation IDs

| Header | Meaning |
|--------|---------|
| `X-Request-ID` | One per HTTP request. Reused from the caller if valid, otherwise a new UUID. |
| `X-Correlation-ID` | Shared by every request in one flow across services. Reused from the caller if valid, otherwise the request ID. |

- Both are returned on every response and exposed through CORS, so browser code can read them. The
  frontend attaches the value to `ApiError.requestId` and to query error logs.
- Caller-supplied IDs are accepted only if they are ≤128 characters of `[A-Za-z0-9._:-]`. Anything else
  is replaced, which prevents log injection.
- Handling runs inside an `http_request` span with both IDs, so logs from handlers, DB and RPC calls
  include them automatically. OpenTelemetry trace context (`traceparent`) is propagated separately for
  distributed tracing.

## Access log

Target `api_access`, one event per request, emitted when the response is ready:

```json
{
  "timestamp": "2026-09-24T10:15:02.113Z",
  "level": "INFO",
  "target": "api_access",
  "message": "GET /api/corridors 200",
  "request_id": "5f0c…",
  "correlation_id": "5f0c…",
  "http_method": "GET",
  "http_path": "/api/corridors",
  "http_query": "limit=10&cursor=djE6bzoxMA",
  "http_status": 200,
  "response_time_ms": 42,
  "request_bytes": 0,
  "response_bytes": 5120,
  "client_ip": "203.0.*.*",
  "user_agent": "Mozilla/5.0 …",
  "instance_id": "stellar-insights-backend-7d9f-abcde"
}
```

- Level: 5xx → ERROR, 4xx → WARN, otherwise INFO.
- Field names match `elk/logstash/pipeline/logstash.conf`. Events are tagged `api_access` and `http_request` there.
- JSON logs are flattened (`flatten_event`), so fields sit at the top level where Filebeat and Logstash expect them.

## Sensitive data

- `Authorization`, `Cookie` and `Set-Cookie` headers are never logged.
- Query-string, form and JSON body values are replaced with `[REDACTED]` when the key contains any of:
  `password`, `passwd`, `secret`, `token`, `apikey`/`api_key`, `authorization`, `private`, `seed`,
  `mnemonic`, `signature`, `jwt`, `cookie`, `credential`, `session`. JSON is redacted recursively.
  This covers, for example, the WebSocket `?token=`.
- Client IPs are partially redacted (`redact_ip`).
- Unparseable JSON bodies are not echoed.

## Configuration

| Variable | Default | Meaning |
|----------|---------|---------|
| `API_LOG_SAMPLE_RATE` | `1.0` | Fraction of fast 2xx/3xx requests logged. **4xx/5xx and slow requests are always logged.** |
| `API_LOG_SLOW_MS` | `1000` | Requests at least this slow are always logged. |
| `API_LOG_SKIP_PATHS` | `/health,/metrics,/ready,/live` | Exact paths never logged (probes, Prometheus scrapes). |
| `API_LOG_BODIES` | `false` | Also log redacted request and response bodies (target `api_access_body`). Turn on temporarily when reproducing an issue. |
| `API_LOG_MAX_BODY_BYTES` | `2048` | Truncation length for logged bodies. |
| `RUST_LOG` | `payraider_backend=info,tower_http=info` | Standard filter. Keep `api_access=info` in production for the audit trail (mainnet: `warn,api_access=info`). |
| `LOG_FORMAT` | `json` | `json` for ELK, anything else for human-readable output. |
| `LOG_DIR` | unset | Also write daily-rotated `stellar-insights.*.log` files to this directory. |

Body capture only happens for JSON, text or form content with a known length of at most 64 KiB. Streaming
responses (SSE, downloads) and WebSocket upgrades are never buffered.

Mainnet samples 20% of fast successful requests (`api-log-sample-rate` in
`k8s/overlays/mainnet/kustomization.yaml`). Other environments log everything.

## Finding a request

```bash
# Locally (LOG_DIR=logs)
curl -si http://localhost:8080/api/corridors | grep -i x-request-id
grep '"request_id":"<id>"' logs/stellar-insights.*.log

# Kubernetes
kubectl -n stellar-insights logs -l component=backend --prefix | grep '<id>'
```

In Kibana, filter `request_id:"<id>"` for one request, or `correlation_id:"<id>"` for a whole flow.
Use `tags:api_access and http_status >= 500` for failing requests.
