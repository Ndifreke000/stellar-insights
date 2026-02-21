# Health & Readiness Endpoints

The backend exposes two probe endpoints for Kubernetes (or any orchestrator) to
verify service health without requiring application-level knowledge.

---

## `GET /health` — Liveness probe

Confirms the HTTP process is alive and able to accept connections.  No
dependency checks are performed; this endpoint should almost always return `200`.

### Response `200 OK`

```json
{
  "status": "healthy",
  "timestamp": "2026-02-02T15:00:00Z",
  "version": "0.1.0"
}
```

---

## `GET /ready` — Readiness probe

Confirms all downstream dependencies are reachable before the pod is added to
the load-balancer pool.

| Dependency | Check performed                            |
|------------|--------------------------------------------|
| `database` | `SELECT 1` against the Postgres pool       |
| `redis`    | `PING` command via async connection        |
| `rpc`      | `net_version` JSON-RPC call to the node    |

### Response `200 OK` — all checks passed

```json
{
  "status": "healthy",
  "timestamp": "2026-02-02T15:00:00Z",
  "version": "0.1.0",
  "checks": {
    "database": { "status": "up", "latency_ms": 2 },
    "redis":    { "status": "up", "latency_ms": 1 },
    "rpc":      { "status": "up", "latency_ms": 150 }
  }
}
```

### Response `503 Service Unavailable` — one or more checks failed

```json
{
  "status": "unhealthy",
  "timestamp": "2026-02-02T15:00:00Z",
  "version": "0.1.0",
  "checks": {
    "database": { "status": "up",   "latency_ms": 2 },
    "redis":    { "status": "down", "latency_ms": 5003, "error": "Connection refused" },
    "rpc":      { "status": "up",   "latency_ms": 120 }
  }
}
```

### HTTP status codes

| Code | Meaning                                |
|------|----------------------------------------|
| 200  | All checks passed                      |
| 503  | One or more dependency checks failed   |
| 500  | The health check handler itself failed |

---

## Kubernetes probe configuration

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 3000
  initialDelaySeconds: 5
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /ready
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 15
  failureThreshold: 3
```

---

## Running tests

```bash
# Start dependencies first (or point env vars at existing instances)
export DATABASE_URL=postgres://postgres:password@localhost/testdb
export REDIS_URL=redis://127.0.0.1/
export RPC_URL=http://localhost:8545

cargo test -p backend
```
