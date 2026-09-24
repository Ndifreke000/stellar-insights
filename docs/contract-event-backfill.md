# Contract Event Backfill

Tracking issue: #2411

## Summary

Historical contract events can be backfilled on demand. The implementation lives in:

- [`backend/src/jobs/backfill.rs`](../backend/src/jobs/backfill.rs): the `BackfillJob` worker, its progress state and its limits
- [`backend/src/api/backfill.rs`](../backend/src/api/backfill.rs): the admin HTTP endpoints
- [`backend/src/services/event_indexer.rs`](../backend/src/services/event_indexer.rs): `detect_ledger_gaps`, `get_indexed_ledger_range` and `index_event`

The routes are mounted at `/admin` in `main.rs`.

## How it works

1. **Validation**: requires `from_ledger <= to_ledger` and a range of at most **100,000**
   ledgers (`MAX_BACKFILL_RANGE`). Only one job runs at a time.
2. **Gap detection**: `detect_ledger_gaps(from, to)` scans `contract_events` and returns
   contiguous ledger ranges that have no indexed rows. The count is reported as `gaps_detected`.
3. **Fetch and index**: the worker pages through the RPC in pages of 200 events
   (`BACKFILL_PAGE_SIZE`) for each gap. If no gaps were found, it walks the whole range.
4. **Rate limiting**: the worker sleeps `delay_ms` between RPC pages. The default is 250 ms.
5. **Idempotency**: events are written with `INSERT OR REPLACE`, so re-running a range is safe.
6. **Progress**: an in-memory `BackfillState` behind `Arc<RwLock<_>>` is updated as the job
   runs and read by the status endpoint.

## API

### Start a backfill

```bash
curl -X POST http://localhost:8080/admin/backfill \
  -H 'Content-Type: application/json' \
  -d '{"from_ledger": 1000, "to_ledger": 2000, "contract_id": null, "delay_ms": 250}'
```

| Field         | Required | Notes                                      |
|---------------|----------|--------------------------------------------|
| `from_ledger` | yes      | Inclusive                                  |
| `to_ledger`   | yes      | Inclusive; the range must be ≤ 100,000     |
| `contract_id` | no       | Restricts the backfill to one contract     |
| `delay_ms`    | no       | Pause between RPC pages; default 250       |

Responses:

- `202 Accepted`: the job started
- `400 Bad Request`: the range is invalid or too large, or a job is already running
- `500 Internal Server Error`: the job failed to start

### Check progress

```bash
curl http://localhost:8080/admin/backfill/status
```

The response fields come from `BackfillState`:

```json
{
  "status": "running",
  "from_ledger": 1000,
  "to_ledger": 2000,
  "current_ledger": 1420,
  "events_indexed": 3812,
  "ledgers_processed": 420,
  "ledgers_total": 1001,
  "gaps_detected": 3,
  "started_at": "2026-09-24T10:00:00Z",
  "finished_at": null,
  "error": null
}
```

`status` is one of `idle`, `running`, `completed` or `failed`.

## Operating procedures

### Recovering after downtime

1. Find the last ledger indexed before the outage. You can use `get_indexed_ledger_range`, or
   run `SELECT MAX(ledger) FROM contract_events`.
2. Get the current network ledger from Horizon or RPC.
3. Start a backfill for `[last_indexed + 1, current]`. Split ranges larger than 100k into
   several sequential jobs.
4. Poll `/admin/backfill/status` until `status` is `completed`. Then start the next chunk.

### Choosing `delay_ms`

- Public or shared RPC: keep the default of 250 ms, or raise it if you see 429 or 503 responses.
- Dedicated RPC node: 0–50 ms is usually safe.

### If a job fails

Read the `error` field from the status endpoint, fix the cause (RPC availability, database
locks and so on), then submit the same range again. Idempotent writes and gap detection
mean ledgers that were already indexed are skipped.

## Known limitations / follow-ups

- **Reorgs:** Stellar has deterministic finality, so there are no chain reorganisations in the
  PoW sense. The remaining risk is RPC nodes returning partial or late data near the tip.
  `INSERT OR REPLACE` makes corrective re-runs safe, but nothing re-verifies recent ledgers
  automatically. Follow-up: re-scan the last N ledgers on a schedule.
- **Empty ledgers look like gaps:** gap detection uses the ledgers present in
  `contract_events`, so ledgers that genuinely had no events are reported as gaps. The worker
  records a per-ledger processed marker to reduce this, but the first run over a sparse range
  will still re-query it.
- **State is in memory:** progress is lost on restart and the job does not resume by itself.
  Re-submit the range; gap detection skips work that is already done.
- **One job at a time**, with no cancel endpoint.
- **Access control:** make sure `/admin/*` sits behind admin auth or an IP allow-list in each
  deployment.
