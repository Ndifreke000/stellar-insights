# Design note: contract event listener cursor expiry and pagination

Tracking issue: #2858
Status: proposed

## Problem

The contract event listener asks Soroban RPC `getEvents` for everything between
the last ledger it saw and the current head. Two situations have no defined behaviour:

1. **The start ledger falls outside RPC retention.** RPC nodes keep events for a limited
   window (configurable per node, commonly about 7 days). If the listener has been
   down longer than that, `getEvents` returns an error. Today that error is logged and
   the loop retries forever with the same out-of-range ledger.
2. **The response is truncated by the page limit.** `getEvents` returns at most one page
   (the default is 100 events; the server sets the maximum). Anything past the first
   page is never requested.

Both cases drop contract events silently.

## Current state

Code checked on `main` at `d60acbb2`.

| Location | What it does | Issue |
|---|---|---|
| [`services/contract_listener.rs`](../../backend/src/services/contract_listener.rs) `poll_for_events` | Calls `getEvents(last_ledger+1 .. head)` once, then sets `self.last_ledger = current_ledger` | The cursor moves to the head even when the page was truncated or individual events failed to process |
| `get_events_for_ledger_range` | Deserialises `result` directly as `Vec<ContractEvent>` | `getEvents` returns an object (`{ events, latestLedger, cursor, ... }`), not an array. As written, every non-empty response fails to parse |
| `ContractEventListener::last_ledger` | Held in memory only | A restart starts at the network head and skips everything in between |
| [`jobs/contract_event_listener.rs`](../../backend/src/jobs/contract_event_listener.rs) `check_for_missed_events` | Computes a start ledger from `contract_events`, then returns `Ok(0)` | A stub. It never calls RPC |
| `start_contract_event_listener_job` | Re-exported from `jobs/mod.rs` | Not called from `main.rs`, so neither the listener nor the job runs in production |

Related work this design reuses:

- [`ingestion/mod.rs`](../../backend/src/ingestion/mod.rs) has a persisted `ingestion_cursor`
  and `check_and_backfill_gap`, which compares the cursor against retention (#2602).
- [`observability/job_alerts.rs`](../../backend/src/observability/job_alerts.rs) has
  `check_ingestion_lag` and the `ingestion_lag_seconds` gauge (#2604).
- [`jobs/backfill.rs`](../../backend/src/jobs/backfill.rs) and
  [contract-event-backfill.md](../contract-event-backfill.md) cover operator-driven backfill
  and `detect_ledger_gaps`.

## Design

### Overview

```
          ┌──────────────── tick ────────────────┐
          ▼                                      │
   load cursor (DB)                              │
          │                                      │
   getEvents(cursor | startLedger, limit=N) ─────┤ error: start out of range
          │                                      ▼
   process page + persist cursor        record gap, restart at oldestLedger
   in ONE transaction                            │
          │                                      │
   page full or cursor < latestLedger? ── yes ──►│ (loop, bounded by max_pages_per_tick)
          │ no
          ▼
   update lag metrics, sleep
```

The poll logic moves into a service that can be tested without RPC or HTTP:
`ContractEventSync` in `services/contract_event_sync.rs`. It is generic over an
`EventSource` trait so tests can inject a mock RPC:

```rust
#[async_trait]
pub trait EventSource {
    async fn get_events(&self, req: GetEventsRequest) -> Result<GetEventsPage, EventSourceError>;
    async fn latest_ledger(&self) -> Result<u32, EventSourceError>;
}

pub enum EventSourceError {
    /// RPC rejected startLedger/cursor as outside its retention window.
    OutOfRange { oldest: Option<u32>, latest: Option<u32>, raw: String },
    /// Network, 5xx, timeout. Retry with backoff; do not move the cursor.
    Transient(anyhow::Error),
    /// Malformed response or another protocol error. Alert; do not move the cursor.
    Fatal(anyhow::Error),
}
```

`ContractEventListener` becomes a thin loop around `ContractEventSync::run_once`.
The job in `jobs/contract_event_listener.rs` calls the same `run_once`, and `main.rs`
starts it behind the existing `enabled` flag.

### Data model

Migration `052_create_contract_event_cursor.sql`:

```sql
-- One row per (network, contract). Holds the resume position.
CREATE TABLE IF NOT EXISTS contract_event_cursor (
    network          TEXT    NOT NULL,
    contract_id      TEXT    NOT NULL,
    rpc_cursor       TEXT,               -- opaque paging cursor from the last page; NULL = use last_ledger
    last_ledger      INTEGER NOT NULL,   -- highest ledger fully processed
    updated_at       TEXT    NOT NULL,
    PRIMARY KEY (network, contract_id)
);

-- Ranges we know we could not fetch. Append-only; resolved by backfill.
CREATE TABLE IF NOT EXISTS contract_event_gaps (
    id               TEXT PRIMARY KEY,
    network          TEXT    NOT NULL,
    contract_id      TEXT    NOT NULL,
    from_ledger      INTEGER NOT NULL,   -- inclusive
    to_ledger        INTEGER NOT NULL,   -- inclusive
    reason           TEXT    NOT NULL,   -- 'cursor_expired' | 'start_out_of_range'
    detected_at      TEXT    NOT NULL,
    resolved_at      TEXT,
    resolved_by      TEXT                -- backfill job id, or 'unrecoverable'
);
CREATE INDEX IF NOT EXISTS idx_contract_event_gaps_open
    ON contract_event_gaps (network, contract_id) WHERE resolved_at IS NULL;
```

`contract_events` already de-duplicates with `INSERT OR REPLACE` on the event id, so
re-processing a page is safe.

### Atomic cursor persistence

For each page:

1. `BEGIN`
2. Insert or replace every event in the page into `contract_events` (and the snapshot
   side tables for `SNAP_SUB` events).
3. `UPDATE contract_event_cursor SET rpc_cursor = page.cursor, last_ledger = ...`
4. `COMMIT`

If the process dies between pages, the next run resumes at the last committed cursor.
Because events are written idempotently, a page that was processed but not committed
is simply processed again.

**Per-event failures.** A single event that fails to decode must not block the stream,
and it must not be dropped silently either. It is written to `contract_events` with
`processing_status = 'failed'` and the error, inside the same transaction, and
`contract_event_decode_failures_total` is incremented. The cursor still advances.
Today these events are logged and lost.

`last_ledger` is set to `min(page.latestLedger, last event ledger)` when the page is
full, and to `page.latestLedger` only when the page is not full. This means a
truncated page never moves the ledger watermark past events we have not read.

### Pagination loop

```
loop {
    page = source.get_events(req).await?
    commit(page)
    pages += 1
    if page.events.len() < limit && cursor_at_or_past(page.latestLedger) { break }   // caught up
    if pages >= max_pages_per_tick { break }  // yield; the next tick continues
    req = GetEventsRequest::from_cursor(page.cursor, limit)
}
```

- `limit` defaults to 200 (`CONTRACT_EVENT_PAGE_LIMIT`), matching `BACKFILL_PAGE_SIZE`.
- `max_pages_per_tick` (default 50) bounds one tick so the loop cannot monopolise the
  DB pool during a large catch-up. Lag stays visible through metrics while it catches up.
- `cursor` and `startLedger` are mutually exclusive in the RPC API. The first request
  after start-up or recovery uses `startLedger`; every later request uses `cursor`.

### Out-of-range detection and recovery

RPC returns a JSON-RPC error (code `-32600`, invalid request) with a message of the form
`startLedger must be between the oldest ledger: X and the latest ledger: Y`.
Detection:

1. Match on error code and the message substring `"oldest ledger"`. Extract `X` and `Y`
   with a regex. Keep the raw message in `EventSourceError::OutOfRange::raw`.
2. If the message cannot be parsed, call `getLatestLedger` and use
   `latest - CONTRACT_EVENT_RPC_RETENTION_LEDGERS` (default 120_960, about 7 days) as `X`.
   The message format is not part of the RPC API contract, so this fallback is
   required. A test pins the current wording.
3. An expired `cursor` (when the cursor's ledger has aged out) goes down the same path.

Recovery, in one transaction:

1. Insert a `contract_event_gaps` row for `[last_ledger + 1, X - 1]` with reason
   `start_out_of_range` (or `cursor_expired`).
2. Set `rpc_cursor = NULL`, `last_ledger = X - 1`.
3. Emit `contract_event_gaps_total{reason}` and send a `Critical` alert through
   `AlertService` naming the contract and range. A gap is data loss unless someone
   backfills it from an archive RPC, so it must page someone. A warning log is not enough.

The next request uses `startLedger = X`.

If `X - 1 < last_ledger + 1` (a race where retention moved by less than one ledger),
no gap is recorded and the listener just retries.

### Closing gaps

Open gaps are listed at `GET /admin/contract-events/gaps`. The existing
`POST /admin/backfill` can be pointed at an archive RPC (`BACKFILL_RPC_URL`, new and
optional) to fill a gap. On success it marks the gap resolved. If no archive RPC is
available, an operator marks the gap `resolved_by = 'unrecoverable'`, which keeps the
record that it happened.

### Metrics

Added to the Prometheus registry in `observability/metrics.rs` and documented in
[prometheus-metrics.md](../prometheus-metrics.md):

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `contract_event_listener_lag_ledgers` | gauge | `contract_id` | `latestLedger - last_ledger` after each tick |
| `contract_event_listener_lag_seconds` | gauge | `contract_id` | Close-time difference, fed to `check_ingestion_lag` |
| `contract_event_pages_fetched_total` | counter | `contract_id` | Pages fetched |
| `contract_event_gaps_total` | counter | `contract_id`, `reason` | Gaps recorded |
| `contract_event_open_gaps` | gauge | `contract_id` | Unresolved gap rows |
| `contract_event_decode_failures_total` | counter | `contract_id` | Events stored as `failed` |
| `contract_event_rpc_errors_total` | counter | `kind` = `transient`/`out_of_range`/`fatal` | RPC failures |

Alert rules: lag above `INGESTION_LAG_ALERT_THRESHOLD_SECONDS` (reuses #2604), any
increase in `contract_event_gaps_total`, and `contract_event_open_gaps > 0` for 24h.

### Configuration

| Env var | Default | Notes |
|---|---|---|
| `CONTRACT_EVENT_PAGE_LIMIT` | `200` | Clamped to `[1, 10000]` at start-up |
| `CONTRACT_EVENT_MAX_PAGES_PER_TICK` | `50` | |
| `CONTRACT_EVENT_RPC_RETENTION_LEDGERS` | `120960` | Used only when the RPC error cannot be parsed |
| `CONTRACT_EVENT_START_LEDGER` | unset | Existing. Used only when no cursor row exists |
| `BACKFILL_RPC_URL` | unset | Optional archive RPC used for gap backfill |

Invalid values fail start-up with a clear message instead of falling back silently.

## Failure modes

| Failure | Behaviour |
|---|---|
| RPC timeout or 5xx | `Transient`. Exponential backoff (cap 60s). Cursor unchanged |
| Start ledger out of retention | Record gap, alert, restart at the oldest available ledger |
| Malformed response | `Fatal`. Alert. Cursor unchanged. Retry next tick |
| One event fails to decode | Stored as `failed`, counter incremented, cursor advances |
| DB commit fails | Transaction rolls back. Cursor unchanged. The page is re-fetched next tick |
| Crash mid-page | Resume from the last committed cursor. Idempotent writes absorb the replay |
| Two instances running | Cursor update is guarded by `WHERE last_ledger = ?` (compare-and-swap). The loser aborts its tick. See [HORIZONTAL_SCALING.md](../HORIZONTAL_SCALING.md) |
| RPC behind (latestLedger < our cursor), e.g. after a failover | No-op tick, `lag_ledgers` reported as 0, warning logged |

## Test plan

Unit tests against a `MockEventSource` (no network), in `services/contract_event_sync.rs`:

1. **Regression: response shape.** A `getEvents` object response parses. Today this fails.
2. **Main path.** Three pages (full, full, partial) are all fetched, every event is stored,
   and the cursor ends at the last page's cursor.
3. **Cursor expired, recovery and gap.** The mock returns the `-32600 ... oldest ledger: 5000`
   error. Expect a gap row `[last+1, 4999]`, a `start_out_of_range` reason, the alert, and a
   restart at `startLedger=5000` that then succeeds. (This is the issue's test plan.)
4. **Unparseable out-of-range message.** Falls back to `latest - retention`, and a gap is
   still recorded.
5. **Page cap.** With `max_pages_per_tick = 2` and five pages pending, the tick stops after
   two and the next tick resumes from the stored cursor.
6. **Atomicity.** Inject a DB failure after the events are inserted. Expect the cursor
   unchanged and no partial commit.
7. **Per-event decode failure.** Stored as `failed`, cursor advances, counter = 1.
8. **Restart.** A new `ContractEventSync` built on the same DB resumes from the stored cursor
   instead of the head.
9. **Error-message pin.** A fixture with the current RPC error string, so an RPC upgrade that
   changes the wording fails CI instead of silently falling back.

## Rollout

1. Land the migration and `ContractEventSync`, with the job still not started.
2. Start the job in `main.rs` behind `CONTRACT_EVENT_LISTENER_ENABLED` (default `false`).
3. Enable it on testnet and watch the lag and gap metrics for a week.
4. Default to `true`.

## Deferred

- Multi-contract fan-out in one `getEvents` call. One cursor row per contract is enough for now.
- Automatic gap backfill without operator action. This needs a reliable archive RPC first.
- A WebSocket/streaming RPC transport.
