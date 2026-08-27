# Issue #2102: Network Congestion Indicator

**Priority:** High  
**Type:** Feature  
**Component:** Backend + Frontend  
**Labels:** `enhancement`, `monitoring`, `network`

## Description

Real-time network congestion indicator showing transaction queue depth, fee levels, and ledger close times. Help users time their transactions optimally.

Stellar applies **surge pricing** when a ledger receives more transactions than it can include. During surge periods, transactions bidding below the market inclusion fee are dropped rather than queued, so a user who submits with a stale `base_fee` silently fails. A live congestion signal — plus a fee recommendation derived from it — lets wallets and integrators decide whether to submit now, raise the max fee, or wait.

## Current Behavior

- `backend/src/api/network.rs` exposes only static network metadata (`get_network_info`, `get_available_networks`, `switch_network`) — no live load signal.
- `RpcClient` can fetch ledgers (`fetch_latest_ledger`, `fetch_ledgers`) but never reads Horizon's `/fee_stats`.
- No fee guidance anywhere in the product; users are left to guess a `base_fee`.
- Ledger close times are not sampled or stored, so there is no baseline to compare against.
- The network page shows configuration, not conditions.

## Expected Behavior

- A single congestion score (0–100) with a categorical level, refreshed at ledger cadence (~5s).
- Live fee percentiles (p10/p50/p70/p90/p99) sourced from Horizon `/fee_stats`.
- Ledger capacity usage and operation counts per ledger.
- Rolling ledger close-time average with deviation from the 5s nominal.
- Fee recommendations for `low` / `normal` / `high` urgency, with an expected inclusion probability.
- Historical congestion series so users can spot recurring daily/weekly congestion windows.
- Push updates over the existing WebSocket channel; no polling required by the frontend.

## Affected Files

**Backend**
- **New file:** `backend/src/services/congestion.rs` — sampling, scoring, and fee recommendation logic.
- **New file:** `backend/src/api/congestion.rs` — HTTP handlers and router.
- **New migration:** `backend/migrations/023_create_network_congestion.sql`
- **Update:** `backend/src/rpc/stellar.rs` — add `fetch_fee_stats()`.
- **Update:** `backend/src/api/mod.rs` — register the congestion router.
- **Update:** `backend/src/api/network.rs` — link congestion into the network summary payload.
- **Update:** `backend/src/jobs/scheduler.rs` — register the `congestion_sampler` job.
- **Update:** `backend/src/services/mod.rs`, `backend/src/websocket.rs`, `backend/src/openapi.rs`

**Frontend**
- **New file:** `frontend/src/components/network/CongestionGauge.tsx`
- **New file:** `frontend/src/components/network/CongestionTimeline.tsx`
- **New file:** `frontend/src/components/network/FeeRecommendationCard.tsx`
- **New file:** `frontend/src/components/network/LedgerCloseTimeChart.tsx`
- **New file:** `frontend/src/services/congestion.ts`
- **Update:** `frontend/src/app/[locale]/network/page.tsx`
- **Update:** `frontend/src/components/layout/sidebar.tsx` — congestion badge on the Network entry.

## Data Sources

| Signal | Source | Method |
|--------|--------|--------|
| Fee percentiles, `ledger_capacity_usage` | Horizon `GET /fee_stats` | new `fetch_fee_stats()` |
| Operations & transactions per ledger | Horizon `GET /ledgers` | `fetch_ledgers()` |
| Ledger close timestamps | Horizon `GET /ledgers` | `fetch_latest_ledger()` |
| Soroban inclusion fees (optional) | Soroban RPC `getFeeStats` | `fetch_fee_stats()` (soroban variant) |

Sampling runs on a scheduler job at `CONGESTION_SAMPLER_INTERVAL_SECONDS` (default `5`), reusing the existing `JobConfig::from_env` pattern in `backend/src/jobs/scheduler.rs`. All calls go through the existing RPC circuit breaker and rate limiter (`backend/src/rpc/circuit_breaker.rs`, `backend/src/rpc/rate_limiter.rs`).

## Congestion Score

Three normalized sub-signals, each clamped to `[0, 1]`, combined into a weighted score:

```
capacity   = ledger_capacity_usage                        // 0..1, straight from /fee_stats
fee_ratio  = clamp((p90_fee - base_fee) / (base_fee * 9), 0, 1)   // base_fee = 100 stroops
close_lag  = clamp((avg_close_time_ms - 5000) / 5000, 0, 1)

score = 100 * (0.50 * capacity + 0.35 * fee_ratio + 0.15 * close_lag)
```

`avg_close_time_ms` is a 12-ledger rolling mean (~1 minute). Weights live in `CongestionWeights` so they can be tuned without touching call sites.

| Score | Level | Meaning |
|-------|-------|---------|
| 0–24 | `low` | Ledgers well under capacity; base fee is sufficient. |
| 25–49 | `moderate` | Occasional surge; use p50 fee. |
| 50–74 | `high` | Sustained surge; bid p90 or defer. |
| 75–100 | `severe` | Ledgers saturated; expect drops below p99. |

## Fee Recommendation

```
low     -> p50 fee, "may take several ledgers"
normal  -> p70 fee, "inclusion in the next few ledgers"
high    -> p99 fee, "inclusion in the next ledger"
```

Each recommendation returns the stroop amount, the percentile it came from, and an `inclusion_probability` estimated from the share of the last 20 ledgers whose `max_fee.min` fell at or below that bid. Recommendations are floored at the 100-stroop network base fee.

## Data Model

`backend/migrations/023_create_network_congestion.sql`:

```sql
CREATE TABLE network_congestion_samples (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    ledger_sequence      INTEGER NOT NULL,
    closed_at            TIMESTAMP NOT NULL,
    close_time_ms        INTEGER NOT NULL,
    operation_count      INTEGER NOT NULL,
    transaction_count    INTEGER NOT NULL,
    failed_transactions  INTEGER NOT NULL DEFAULT 0,
    capacity_usage       REAL NOT NULL,
    fee_p10              INTEGER NOT NULL,
    fee_p50              INTEGER NOT NULL,
    fee_p70              INTEGER NOT NULL,
    fee_p90              INTEGER NOT NULL,
    fee_p99              INTEGER NOT NULL,
    congestion_score     REAL NOT NULL,
    congestion_level     TEXT NOT NULL,
    created_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (ledger_sequence)
);

CREATE INDEX idx_congestion_closed_at ON network_congestion_samples (closed_at DESC);
CREATE INDEX idx_congestion_level ON network_congestion_samples (congestion_level, closed_at DESC);

-- Rolled-up buckets so 30-day history queries do not scan raw ledger rows.
CREATE TABLE network_congestion_buckets (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_start      TIMESTAMP NOT NULL,
    bucket_seconds    INTEGER NOT NULL,
    avg_score         REAL NOT NULL,
    max_score         REAL NOT NULL,
    avg_capacity      REAL NOT NULL,
    avg_close_time_ms INTEGER NOT NULL,
    median_fee        INTEGER NOT NULL,
    p90_fee           INTEGER NOT NULL,
    sample_count      INTEGER NOT NULL,
    UNIQUE (bucket_start, bucket_seconds)
);
```

Raw samples are retained for 7 days; a nightly job rolls them into 5-minute and 1-hour buckets and prunes. Retention is configurable via `CONGESTION_RAW_RETENTION_DAYS`.

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/network/congestion` | Current score, level, sub-signals, latest ledger. |
| GET | `/api/network/congestion/history?window=24h&interval=5m` | Bucketed time series. `window`: `1h`\|`6h`\|`24h`\|`7d`\|`30d`. |
| GET | `/api/network/congestion/fees` | All three recommendations plus raw percentiles. |
| GET | `/api/network/congestion/fees?priority=high` | Single recommendation. |
| GET | `/api/network/congestion/windows?tz=UTC` | Hour-of-week heatmap of average congestion (best/worst times to transact). |

**WebSocket:** topic `network.congestion`, emitted on every sample where the level changes or every 12th sample, whichever comes first.

### Response — `GET /api/network/congestion`

```json
{
  "score": 38.4,
  "level": "moderate",
  "ledger_sequence": 54231889,
  "closed_at": "2026-08-27T11:04:12Z",
  "signals": {
    "capacity_usage": 0.42,
    "avg_close_time_ms": 5180,
    "close_time_deviation_ms": 180,
    "operations_per_ledger": 812,
    "transactions_per_ledger": 214,
    "failed_transactions": 3
  },
  "fees": {
    "base_fee": 100,
    "p10": 100, "p50": 120, "p70": 200, "p90": 500, "p99": 1500
  },
  "trend": "rising",
  "sampled_at": "2026-08-27T11:04:14Z"
}
```

`trend` compares the current score with the mean of the previous 12 samples: `rising` / `falling` / `stable` (±3 points).

### Errors

- **503 Service Unavailable** — RPC circuit breaker open; response includes `last_known` sample and its age.
- **400 Bad Request** — unsupported `window` / `interval` / `priority` value.

## UI Structure

```
┌──────────────────────────────────────────────────────┐
│ Network Congestion                    ● Moderate     │
├──────────────────────────────────────────────────────┤
│         ╭───────────╮                                │
│         │    38     │   Capacity used        42%     │
│         │  /  100   │   Ledger close      5.18 s     │
│         ╰───────────╯   Ops / ledger         812     │
│         ▲ rising        Failed tx              3     │
├──────────────────────────────────────────────────────┤
│ Recommended fee                                      │
│                                                      │
│  Economy      120 stroops   ~4 ledgers      p50      │
│  Standard     200 stroops   ~2 ledgers      p70   ◀  │
│  Priority   1,500 stroops   next ledger     p99      │
│                                                      │
│  Base fee is 100 stroops. Bids below p10 are being   │
│  dropped in the current surge window.                │
├──────────────────────────────────────────────────────┤
│ Last 24 hours                        [1h 6h 24h 7d]  │
│                                                      │
│ 100┤                                                 │
│    │              ▄▄                                 │
│  50┤        ▄▄▄▄▄███▄▄        ▄▄▄                    │
│    │   ▄▄▄▄███████████▄▄▄▄▄▄▄█████▄▄▄▄               │
│   0└────────────────────────────────────             │
│    00:00      06:00      12:00      18:00            │
├──────────────────────────────────────────────────────┤
│ Best times to transact (last 30 days, UTC)           │
│      00 03 06 09 12 15 18 21                         │
│ Mon  ░░ ░░ ░░ ▒▒ ▓▓ ██ ▓▓ ▒▒                         │
│ Tue  ░░ ░░ ▒▒ ▓▓ ██ ██ ▓▓ ▒▒                         │
│ ...                                                  │
└──────────────────────────────────────────────────────┘
```

- The gauge colour tracks the level; never colour alone — the level label and the numeric score are always present for accessibility.
- `LedgerCloseTimeChart` overlays the 5s nominal as a reference line.
- The sidebar Network entry carries a dot in the current level colour with an `aria-label` naming the level.

## Acceptance Criteria

- [ ] `fetch_fee_stats()` added to `RpcClient` with unit tests over a recorded Horizon `/fee_stats` payload
- [ ] `CongestionService` computes score, level, and trend from live samples
- [ ] Scheduler job samples at the configured interval and persists to `network_congestion_samples`
- [ ] Migration `023_create_network_congestion.sql` applies cleanly and is idempotent
- [ ] `GET /api/network/congestion` returns current conditions in under 50 ms (served from cache, not RPC)
- [ ] `GET /api/network/congestion/history` supports all documented windows and intervals
- [ ] `GET /api/network/congestion/fees` returns three recommendations with inclusion probabilities
- [ ] `GET /api/network/congestion/windows` returns a 7×24 heatmap grid
- [ ] WebSocket `network.congestion` topic broadcasts on level change
- [ ] Circuit-breaker-open path returns 503 with a stale-but-labelled `last_known` sample
- [ ] Rollup job buckets raw samples and prunes beyond the retention window
- [ ] Congestion gauge, timeline, fee card, and close-time chart rendered on the network page
- [ ] Sidebar congestion badge reflects live level
- [ ] Frontend degrades to the last received sample when the socket drops, with a staleness label
- [ ] Charts follow the existing export pattern (`ChartExportButton`)
- [ ] Components pass the accessibility checks used elsewhere in the app (see `ACCESSIBILITY_SUMMARY.md`)
- [ ] OpenAPI spec updated in `backend/src/openapi.rs`
- [ ] Backend and frontend tests added; docs page `docs/NETWORK_CONGESTION.md` written

## Implementation Steps

1. **RPC layer** — add `fetch_fee_stats()` to `backend/src/rpc/stellar.rs`, with the `FeeStats` model and error mapping consistent with the other fetchers.
2. **Scoring service** — `backend/src/services/congestion.rs`: rolling window buffer, score computation, trend detection, fee recommendations. Pure functions for the maths so they are unit-testable without RPC.
3. **Persistence** — write migration `023`, add queries to `backend/src/db/`, wire the sampler job into `backend/src/jobs/scheduler.rs`.
4. **API** — `backend/src/api/congestion.rs` with the five endpoints; register in `backend/src/api/mod.rs`; add cache headers via the existing `http_cache` middleware (1s TTL on current, 60s on history).
5. **Broadcast** — publish level changes through `backend/src/websocket.rs` on the `network.congestion` topic.
6. **Rollup + retention** — nightly job to fill `network_congestion_buckets` and prune raw samples.
7. **Frontend service** — `frontend/src/services/congestion.ts`, typed against the API responses, with socket subscription and REST fallback.
8. **Frontend components** — gauge, timeline, fee card, close-time chart, hour-of-week heatmap.
9. **Page integration** — extend `frontend/src/app/[locale]/network/page.tsx`; add the sidebar badge.
10. **Testing** — service unit tests, API integration tests, component tests, and a load check that the sampler holds up at 5s cadence.

## Considerations

- **Surge pricing is per-ledger and bursty.** Score off a rolling window, not a single ledger, or the gauge will flicker between levels every few seconds.
- **Horizon `/fee_stats` covers the last 5 ledgers only.** Longer baselines must come from our own stored samples.
- **Testnet fee stats are not representative.** Gate the hour-of-week heatmap on mainnet; on testnet show live signals only.
- **Never present a fee recommendation as a guarantee.** The UI copy states an expected inclusion probability, not a promise.
- **Soroban inclusion fees differ from classic.** Phase 1 covers classic; the Soroban split is called out as a follow-up rather than silently averaged in.

## References

- [Fees, surge pricing, and fee strategies](https://developers.stellar.org/docs/learn/fundamentals/fees-resource-limits-metering)
- [Horizon `/fee_stats`](https://developers.stellar.org/docs/data/apis/horizon/api-reference/aggregations/fee-stats)
- [Soroban RPC `getFeeStats`](https://developers.stellar.org/docs/data/apis/rpc/api-reference/methods/getFeeStats)
- Internal: `docs/RPC.md`, `docs/RPC_DATA_SOURCES.md`, `docs/OBSERVABILITY.md`

## Related Issues

- Related to: #2103 Anchor Downtime Tracker and Alerting (shares the health/status surface)
- Feeds: Issue #023 Payment Success Prediction Model Enhancement — congestion is a named missing feature there
- Related to: existing alerting in `backend/src/alerts.rs`

## Estimated Effort

- RPC + scoring service: 1.5 days
- Persistence, sampler job, rollups: 1 day
- API endpoints + WebSocket: 1 day
- Frontend components and page: 2 days
- Testing, docs, polish: 0.5 days
- **Total: 6 days**
