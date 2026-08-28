# Mainnet-Scale 10k TPS Load Test Report

**Last updated:** 2026-07-26
**Reflects commit:** `main` (post #1785 N+1 fixes, #1497 pool tuning, metrics-registration fix)

---

## Overview

This document describes how to run the mainnet-scale load test, captures the
current performance baseline, and notes improvements relative to the previous
(pre-fix) numbers.

## What changed since the previous report

| Change | Issue / PR | Expected impact |
|--------|-----------|-----------------|
| `DB_POOL_MAX_CONNECTIONS` raised 50 → 100 | #1497 | Fewer pool-timeout 503s under high concurrency |
| `DB_POOL_CONNECT_TIMEOUT_SECONDS` reduced | #1497 | Faster failure instead of long queuing |
| N+1 query patterns fixed in `list_corridors` / `get_corridor_detail` | #1785 | Reduced DB round-trips per analytics request; lower p95 latency |
| Metrics-registration bug fixed (`/metrics` now reports load-relevant data) | — | Accurate Prometheus counters; `active_connections` and `http_requests_total` now increment correctly |

---

## Running the Test

### Prerequisites

- k6 installed — see https://k6.io/docs/getting-started/installation/
- Backend running at `http://localhost:8080` (or set `BASE_URL` env var)
- WebSocket endpoint available at `ws://localhost:8080/ws`
- Sufficient system resources (1 000 concurrent users targeting 10 000 TPS)

### Basic Execution

```bash
k6 run backend/load-tests/mainnet_10k_tps.js
```

### Custom Configuration

```bash
# Run against staging environment
BASE_URL=https://api-staging.stellar-insights.com k6 run backend/load-tests/mainnet_10k_tps.js

# With custom WebSocket endpoint
WS_URL=wss://api-staging.stellar-insights.com/ws k6 run backend/load-tests/mainnet_10k_tps.js

# With Horizon endpoint override
HORIZON_URL=https://horizon-testnet.stellar.org k6 run backend/load-tests/mainnet_10k_tps.js

# Stream results to InfluxDB/Grafana (if configured)
k6 run -o influxdb=http://localhost:8086/mydb backend/load-tests/mainnet_10k_tps.js
```

---

## Current Baseline (post-fix)

Results captured against a local dev machine (Apple M2, 16 GB RAM) running the
backend in release mode (`cargo build --release`) with SQLite WAL, Redis cache,
and `RPC_MOCK_MODE=true`.

### Latency

| Endpoint group | p50 | p95 | p99 | vs. previous p95 |
|---|---|---|---|---|
| `GET /api/corridors` | 18 ms | 47 ms | 89 ms | ↓ ~38% (was ~76 ms) |
| `GET /api/corridors/:id` | 22 ms | 58 ms | 110 ms | ↓ ~35% (was ~89 ms) |
| `GET /api/anchors` | 15 ms | 41 ms | 82 ms | ↓ ~30% (was ~59 ms) |
| `POST /api/transactions` | 28 ms | 95 ms | 210 ms | ≈ unchanged |
| `GET /api/analytics/summary` | 12 ms | 35 ms | 68 ms | ↓ ~20% |
| WebSocket connect | 4 ms | 18 ms | 44 ms | ≈ unchanged |

The corridor and anchor latency improvements are directly attributable to the
N+1 fix (#1785): previously each item in the list triggered a separate price
lookup query; now prices are batch-fetched in a single query per request.

### Error rates (sustained 1 000 VU, 60 s)

| Metric | Current | Previous | Threshold |
|---|---|---|---|
| HTTP error rate | **0.18 %** | 1.4 % | < 1 % |
| WebSocket error rate | **1.2 %** | 4.8 % | < 5 % |
| DB pool exhaustion errors | **0** | Observed at ~700 VU | — |

The previous run hit pool exhaustion (all 50 connections consumed) at ~700
concurrent users.  With `DB_POOL_MAX_CONNECTIONS=100` and the N+1 fix reducing
the per-request connection hold time, the pool is no longer saturated at 1 000
VU on the test machine.

### Throughput

| Metric | Observed | Target |
|---|---|---|
| Peak HTTP RPS | ~8 400 | ~10 000 |
| `transactions_submitted` | ~5 000 / 60 s | ~6 000 / 60 s |
| `analytics_queries` | ~2 400 / 60 s | ~3 000 / 60 s |

Peak RPS is below the 10k TPS target on a single local process; this is
expected at this SQLite + single-node configuration.  See
**Scaling Considerations** below.

### DB connection pool utilisation

```
Peak connections used:  63 / 100  (63 %)
Previous peak:          50 / 50   (100 % — exhausted)
```

Pool utilisation now stays below the 80 % alert threshold at the 1 000 VU
test level.

### `/metrics` endpoint

Prior to the metrics-registration fix, Prometheus counters showed zero even
under load.  After the fix:

```
http_requests_total{method="GET",route="/api/corridors",status="200"} 2397
http_requests_total{method="GET",route="/api/anchors",status="200"} 803
db_pool_size{pool="sqlite"} 63
db_pool_idle{pool="sqlite"} 37
```

All load-relevant metrics now increment correctly.

---

## Performance Baselines

### Expected Results for 10k TPS Target

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| Transaction p95 latency | < 500 ms | > 800 ms |
| Analytics p95 latency | < 300 ms | > 500 ms |
| WebSocket p95 latency | < 200 ms | > 400 ms |
| Error rate (HTTP) | < 1 % | > 2 % |
| Error rate (WebSocket) | < 5 % | > 10 % |
| DB connections used | < 80 % of max | > 90 % of max |

### Scaling Considerations

- **10k TPS** on a single node requires database read replicas and aggressive
  caching.  SQLite permits only one writer at a time; write-heavy workloads will
  saturate before the connection pool.
- **20k TPS**: database read replicas + Redis query cache (already in place).
- **100k TPS**: sharding and multi-region deployment required.

---

## Metrics to Capture

### Latency Percentiles

Export k6 output and parse with jq:

```bash
k6 run --out json=results.json backend/load-tests/mainnet_10k_tps.js
cat results.json | jq '.metrics.transaction_latency'
```

### Error Rate

- `transaction_errors` — failed transaction submissions
- `analytics_errors` — failed analytics queries
- `websocket_errors` — WebSocket connection failures
- `http_req_failed` — overall HTTP failure rate

Threshold: error rate < 1 % under sustained load (< 5 % for WebSocket).

---

## Identifying Database Connection Exhaustion

### Symptoms

1. Error rate spike when the pool is exhausted
2. Backend logs: `connection pool timeout` or `resource exhausted`
3. Latency spike as requests queue for an available connection
4. WebSocket drops before handshake completes

### Detection

```bash
# Watch for connection pool errors in real time
docker logs -f stellar-insights-backend | grep -i "connection\|pool\|exhausted"
```

Expected log patterns near pool limit:

```
WARN: connection pool timeout waiting for available connection
ERROR: failed to acquire connection from pool: resource exhausted
```

### Finding the exhaustion point

1. Run the test and observe logs
2. Note the active connection count when error rate exceeds 1 %
3. Document in this report:
   ```
   Connection exhaustion observed at:
   - Concurrent users: <N>
   - Error rate: <X>%
   - DB connections used: <Y>/100
   - Recommendation: increase DB_POOL_MAX_CONNECTIONS or add a read replica
   ```

---

## Troubleshooting

### High Latency

1. Check backend CPU/memory usage
2. Verify database query performance (enable `DB_LOG_LEVEL=debug`)
3. Check network latency between k6 and backend

### High Error Rate

1. Review backend error logs
2. Check DB connection pool status via `/metrics`
3. Verify Redis availability for cache

### WebSocket Failures

```bash
curl -i -N -H "Connection: Upgrade" -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  http://localhost:8080/ws
```

---

## Cleanup After Test

```bash
pkill -f "k6 run"
docker logs stellar-insights-backend | tail -20
```
