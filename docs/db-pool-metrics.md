# Database Connection Pool Metrics

Tracking issue: #2414

## Summary

The SQLx pool is monitored through Prometheus gauges, a histogram and a counter, all
registered in [`backend/src/observability/metrics.rs`](../backend/src/observability/metrics.rs).
They are scraped from `GET /metrics`.

```bash
curl -s http://localhost:8080/metrics | grep db_pool
```

## Metrics

| Metric                          | Type      | Labels | Meaning                                  |
|---------------------------------|-----------|--------|------------------------------------------|
| `db_pool_connections_active`    | gauge     | –      | Connections currently checked out        |
| `db_pool_connections_idle`      | gauge     | –      | Idle connections in the pool             |
| `db_pool_utilization`           | gauge     | –      | `active / total`, from 0 to 1            |
| `db_pool_wait_time_seconds`     | histogram | –      | Time spent waiting to acquire a connection |
| `db_pool_errors_total`          | counter   | `kind` | Pool errors, by kind                     |
| `db_pool_size`                  | gauge     | –      | Total connections (legacy)               |
| `db_pool_idle` / `db_pool_active` | gauge   | –      | Legacy aliases, updated by `/api/v1/db/pool-metrics` |

Values currently emitted for the `kind` label:

- `near_exhaustion`: the background monitor saw utilization above 90%
- `exhausted`: a request failed with `503` because no connection was available (see `error.rs`)

## Where the values come from

- **Background monitor** (`main.rs`, `pool_exhaustion_handle`): runs every 30 s. It reads
  `pool.size()` and `pool.num_idle()`, then calls `set_pool_connections(active, idle, total)`,
  which sets the active, idle and utilization gauges. It also logs a warning and records
  `near_exhaustion` when utilization is above 0.9.
- **Request path** (`error.rs`): records `exhausted` when a pool timeout is converted into a
  `503 Service Unavailable`.
- **`GET /api/v1/db/pool-metrics`**: returns the pool stats as JSON and refreshes the legacy
  gauges.

Because the gauges are sampled every 30 s, short spikes can be missed. Use
`db_pool_errors_total` to catch real exhaustion.

## Suggested alerts (Prometheus)

```yaml
groups:
  - name: db-pool
    rules:
      - alert: DbPoolHighUtilization
        expr: db_pool_utilization > 0.9
        for: 5m
        labels: { severity: warning }
        annotations:
          summary: "DB pool > 90% utilised for 5m"

      - alert: DbPoolExhausted
        expr: increase(db_pool_errors_total{kind="exhausted"}[5m]) > 0
        labels: { severity: critical }
        annotations:
          summary: "Requests are failing because the DB pool is exhausted"

      - alert: DbPoolSlowAcquire
        expr: histogram_quantile(0.95, rate(db_pool_wait_time_seconds_bucket[5m])) > 0.5
        for: 10m
        labels: { severity: warning }
        annotations:
          summary: "p95 DB connection acquire time > 500ms"
```

## Suggested dashboard panels

| Panel                    | Query                                                                 |
|--------------------------|-----------------------------------------------------------------------|
| Active vs idle           | `db_pool_connections_active`, `db_pool_connections_idle` (stacked)    |
| Utilization %            | `db_pool_utilization * 100` (thresholds at 70 and 90)                 |
| Acquire latency p50/p95  | `histogram_quantile(0.5\|0.95, rate(db_pool_wait_time_seconds_bucket[5m]))` |
| Pool errors by kind      | `sum by (kind) (rate(db_pool_errors_total[5m]))`                      |

## Capacity planning

- If `db_pool_utilization` regularly stays above 0.7 at peak, raise the pool's max connections.
  Also check that SQLite or the database can handle the extra concurrency.
- If utilization stays high but wait time is low, the pool size is right.
- If wait time rises while utilization is below 1, look for long-running queries holding
  connections rather than an undersized pool.

## Known gaps / follow-ups

- **`db_pool_wait_time_seconds` is registered but never observed.** Nothing calls
  `observe_pool_wait_time` yet, so the histogram and the `DbPoolSlowAcquire` alert stay empty.
  The fix is to time `pool.acquire()` (or wrap the pool) and call
  `observe_pool_wait_time(elapsed.as_secs_f64())`.
- Only two error kinds are recorded. Connection or open failures (`kind="connect"`) and
  timeouts outside the `503` path are not counted.
- There is no committed Grafana panel or alert rule for the pool yet. Copy the snippets above
  into the monitoring configuration.
