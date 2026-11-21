# Corridor Alert Rules Engine: Composable Conditions with Backtesting

**GitHub Issue**: [#2855](https://github.com/Ndifreke000/stellar-insights/issues/2855)
**Related**: [SLOs_AND_ALERTING.md](./SLOs_AND_ALERTING.md)

---

## Overview

Corridor alerts today are fixed per-metric checks. Each fires when a single number crosses a line, so a brief dip in a low-traffic corridor pages someone, and nothing can express what users actually want:

> success rate < 95% **AND** volume > $50k, sustained for 15 minutes

This issue adds:

1. A **rule AST** (AND/OR/NOT, time windows, aggregations) with a JSON Schema.
2. An **evaluator** that runs rules over the metrics stream.
3. A **backtest endpoint**: how often would this rule have fired over the last 30 days?
4. A **UI rule builder**.

Existing configs keep working; they are converted to equivalent rules.

---

## Current State

| File | What it does |
|---|---|
| `backend/migrations/041_create_corridor_alert_configs.sql` | One row per alert: fixed columns `success_rate_threshold`, `latency_threshold_ms`, `liquidity_threshold_usd`, `*_drop_pct`, `cooldown_seconds`, `notify_*` |
| `backend/migrations/040_create_corridor_performance_snapshots.sql` | `corridor_performance_snapshots(corridor_key, success_rate, avg_settlement_latency_ms, liquidity_depth_usd, volume_usd, total/successful/failed_transactions, snapshot_time)` |
| `backend/src/db/corridor_alerts.rs` | CRUD for configs/events, snapshot queries (`get_latest_…`, `get_previous_…`, `get_snapshots_for_corridor`) |
| `backend/src/services/corridor_performance_monitor.rs` | Spawned every 60 s from `main.rs`. For each active config with a `corridor_key`, it compares the **latest** snapshot against the thresholds and against the **previous** snapshot (relative drop) |
| `backend/src/api/corridor_alerts.rs` | `/api/v1/corridor-alerts/configs`, `/snapshots`, `/summary`, `/events`, `/events/{id}/acknowledge`, `/unread-count` |
| `backend/src/alerts.rs` | Older in-memory `AlertManager::check_and_alert` with hard-coded thresholds (−10 pts, ×1.5, ×0.7), broadcast to WebSocket and webhooks |
| `frontend/src/app/[locale]/corridor-alerts/page.tsx` | Config form with fixed fields |

### Gaps that affect this design

| # | Gap | Consequence |
|---|---|---|
| A1 | **Nothing writes `corridor_performance_snapshots` in production.** `insert_performance_snapshot` has no caller outside tests | The monitor evaluates nothing in a real deployment, and there is no history to backtest. **This is a prerequisite** (§1) |
| A2 | Global configs (`corridor_key IS NULL`) are grouped but skipped in `evaluate_all_corridors` | "All corridors" alerts never fire |
| A3 | Single-sample evaluation, with no window and no "for" duration | Noisy: one bad minute fires |
| A4 | Conditions are OR'd implicitly (each threshold fires its own event); there is no AND | The core user request |
| A5 | Two alert systems (`alerts.rs` hard-coded, and configs) with different thresholds | Confusing duplicate notifications |
| A6 | `success_rate` is stored as 0–1, while the old `alerts.rs` treats it as 0–100 | A unit trap for rule authors; the schema must fix one unit |

---

## Design

### 1. Prerequisite: a metrics history

Add a writer: a job in `backend/src/jobs/` that runs every `CORRIDOR_SNAPSHOT_INTERVAL_SECS` (default 60). It computes per-corridor metrics from the ingestion data the corridor endpoints already use, and calls `insert_performance_snapshot`.

- Retention: 35 days at 1-minute resolution, using the existing `services/retention.rs`. About 1,440 rows per corridor per day. At 200 corridors, 35 days is roughly 10 M narrow rows, which is fine for SQLite with the existing `(corridor_key, snapshot_time DESC)` index.
- Optional downsampling to 5 minutes after 7 days, if storage becomes an issue.
- Treat snapshots as **interval aggregates**: `volume_usd` and the transaction counts cover the interval, while `success_rate` = successful ÷ total for the interval. Document this in the migration comment, because every aggregation below depends on it.

The backtest is only as good as this history. On a fresh deployment, it reports `"coverage": 0.12` (see §5) instead of pretending.

### 2. Rule AST

```jsonc
{
  "version": 1,
  "scope": { "corridors": ["USDC:GA5Z…->NGN:GB…"] },      // or { "all": true } or { "anchor": "…" }
  "condition": {
    "all": [                                             // all = AND, any = OR, not = NOT
      {
        "metric": "success_rate",
        "agg": "avg",
        "window": "15m",
        "op": "<",
        "value": 0.95
      },
      {
        "metric": "volume_usd",
        "agg": "sum",
        "window": "15m",
        "op": ">",
        "value": 50000
      }
    ]
  },
  "for": "15m",            // condition must hold continuously this long before firing (0 = immediately)
  "cooldown": "1h",        // min time between firings for the same (rule, corridor)
  "resolve_after": "10m",  // condition false this long ⇒ emit 'resolved'
  "severity": "critical"
}
```

**Leaf** (`Comparison`):

| Field | Allowed values |
|---|---|
| `metric` | `success_rate` (0–1), `latency_ms` (`avg_settlement_latency_ms`), `liquidity_usd`, `volume_usd`, `tx_count`, `failed_tx_count` |
| `agg` | `last`, `avg`, `min`, `max`, `sum`, `p95` (latency only), `rate_of_change` (percentage change between window start and end), `weighted_avg` (success rate weighted by `tx_count`, the right default for `success_rate`) |
| `window` | `1m` to `24h`; must be a multiple of the snapshot interval |
| `op` | `<`, `<=`, `>`, `>=`, `==`, `!=` |
| `value` | number |
| `compare_to` (optional) | `{ "offset": "7d" }` compares against the same aggregate one offset earlier ("50% below this time last week"). Then `value` is a ratio |

**Branches**: `{ "all": [ … ] }`, `{ "any": [ … ] }`, `{ "not": { … } }`. Maximum depth 4, maximum 10 leaves. Both limits are enforced at validation, which bounds evaluation cost and keeps the UI usable.

**Rust types** (`backend/src/alert_rules/ast.rs`):

```rust
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
    #[serde(untagged)]
    Cmp(Comparison),
}

pub struct Comparison {
    pub metric: Metric,
    pub agg: Aggregation,
    #[serde(with = "humantime_serde")]
    pub window: Duration,
    pub op: Op,
    pub value: f64,
    pub compare_to: Option<CompareTo>,
}
```

**JSON Schema**: generated from these types with `schemars`, served at `GET /api/v1/corridor-alerts/rules/schema`, and checked in at `docs/schemas/corridor-alert-rule.v1.json`. A test fails CI if the checked-in schema drifts from the generated one. The frontend builder validates against the same schema.

**Validation** (`POST`/`PUT` return `422` with a JSON Pointer to the bad node):
- Depth and leaf limits.
- `window` ≥ snapshot interval and ≤ 24 h; `for` ≤ 24 h.
- `agg` is compatible with `metric` (e.g. `p95` only on latency, `sum` not on `success_rate`).
- `value` is in range for the unit (`success_rate` must be 0–1). If a user enters `95` for `success_rate`, return the error "did you mean 0.95?" (gap A6).

### 3. Storage

```sql
CREATE TABLE corridor_alert_rules (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL,
    name              TEXT NOT NULL,
    rule_json         TEXT NOT NULL,          -- validated AST, schema version inside
    is_active         BOOLEAN NOT NULL DEFAULT 1,
    notify_email      BOOLEAN NOT NULL DEFAULT 0,
    notify_webhook    BOOLEAN NOT NULL DEFAULT 0,
    notify_in_app     BOOLEAN NOT NULL DEFAULT 1,
    notify_slack      BOOLEAN NOT NULL DEFAULT 0,
    notify_telegram   BOOLEAN NOT NULL DEFAULT 0,
    migrated_from_config_id TEXT,             -- set for auto-converted legacy configs
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Per (rule, corridor) evaluation state so 'for', cooldown and resolve survive restarts.
CREATE TABLE corridor_alert_rule_state (
    rule_id         TEXT NOT NULL REFERENCES corridor_alert_rules(id) ON DELETE CASCADE,
    corridor_key    TEXT NOT NULL,
    state           TEXT NOT NULL CHECK (state IN ('ok','pending','firing')),
    pending_since   TEXT,
    last_fired_at   TEXT,
    last_true_at    TEXT,
    PRIMARY KEY (rule_id, corridor_key)
);
```

Fired alerts keep going into `corridor_alert_events`. A new nullable `rule_id` column sits beside `config_id`, and `alert_type` becomes `rule_fired` / `rule_resolved`. The existing events, acknowledge and unread-count endpoints then keep working unchanged.

**Legacy conversion**: a one-time migration step converts each `corridor_alert_configs` row into an equivalent rule. Each non-null threshold becomes its own leaf (`last` aggregation, `window` = 1 snapshot) under `any`, which matches today's OR behaviour. The drop percentages become `rate_of_change` leaves over two snapshots. Mark the old rows inactive and keep them for one release. Write a golden test proving that converted rules fire on exactly the same snapshot sequences as the old monitor.

### 4. Evaluator

The evaluator is a pure core with a thin driver:

```rust
// Pure: no I/O, used identically by live evaluation and backtest.
pub fn evaluate(cond: &Condition, series: &SeriesView, at: DateTime<Utc>) -> Option<bool>;

pub fn step(state: &RuleState, cond_true: Option<bool>, at: DateTime<Utc>, rule: &Rule) -> (RuleState, Option<Transition>);
// Transition = Fired | Resolved
```

- `SeriesView` gives per-metric access to the snapshots for one corridor over `[at - max_window, at]`. Each leaf aggregates over its own window.
- **Missing data** returns `None`, not `false`. In `all`, `None` makes the whole result `None` (unknown). In `any`, `true` wins over `None`. When the result is `None`, the state machine keeps its current state and does not advance `for` timers. A data gap therefore can neither fire nor resolve an alert. If more than 50% of a window's snapshots are missing, the window counts as missing.
- **State machine** per (rule, corridor):
  - `ok` → condition true → `pending` (record `pending_since`).
  - `pending` → still true and `now - pending_since ≥ for` → `firing`. Emit **Fired** unless within `cooldown` of `last_fired_at`.
  - `pending` → false → `ok`.
  - `firing` → false for at least `resolve_after` → `ok`. Emit **Resolved**.
- **Driver**: replace the body of `CorridorPerformanceMonitor::evaluate_all_corridors`. Each tick (aligned to the snapshot interval, just after the snapshot job writes):
  1. Load the active rules and expand their scope to a corridor list. `all: true` fixes gap A2.
  2. Group by corridor and fetch each corridor's series once, covering the largest window needed.
  3. Evaluate, step, and persist the state changes in one transaction.
  4. Hand each transition to the existing notification fan-out (`notify_*`, broadcast channel, webhooks).
- Cost: O(rules × corridors-in-scope × leaves × window-points). With the limits above (10 leaves, 24 h ÷ 1 min = 1,440 points), worst case is about 15 k operations per rule-corridor per tick. Add a Prometheus histogram `corridor_rule_eval_seconds` and a gauge for rules in each state.
- Retire `alerts.rs::check_and_alert`'s hard-coded corridor thresholds (gap A5). Anchor alerts (`send_anchor_alert`) stay as they are.

### 5. Backtest endpoint

`POST /api/v1/corridor-alerts/rules/backtest`

```json
{
  "rule": { …AST… },          // or "rule_id": "…"
  "from": "2026-10-22T00:00:00Z",   // default: now - 30d; max range 35d
  "to":   "2026-11-21T00:00:00Z"
}
```

Response:

```json
{
  "fired_count": 7,
  "corridors_fired": 3,
  "total_firing_duration": "5h40m",
  "coverage": 0.97,                       // fraction of expected snapshots present
  "per_corridor": [
    { "corridor_key": "USDC:…->NGN:…", "fired_count": 4, "firing_duration": "3h10m" }
  ],
  "firings": [
    { "corridor_key": "…", "fired_at": "…", "resolved_at": "…", "values": { "0": 0.91, "1": 72000 } }
  ],
  "truncated": false
}
```

- Run the **same** `evaluate` and `step` functions, replaying snapshots in time order through a fresh in-memory state. Live and backtest behaviour are then identical by construction, including `for`, cooldown and missing-data handling.
- `values` holds each leaf's aggregated value at fire time, keyed by leaf index in depth-first order. The UI uses it to show why the rule fired.
- Limits: at most 500 `firings` returned (`truncated: true` beyond that), at most 50 corridors in scope, and a 10 s timeout. Rate limit: 10 backtests per user per minute. If the data reads become slow, run the backtest on a blocking thread over a single streaming query per corridor.
- `coverage < 0.8` adds the warning `"insufficient_history"`. Show it prominently in the UI (see gap A1).

### 6. API summary

| Method & path | Purpose |
|---|---|
| `GET /api/v1/corridor-alerts/rules` | List the user's rules (with current state per corridor) |
| `POST /api/v1/corridor-alerts/rules` | Create (validated) |
| `GET/PUT/DELETE /api/v1/corridor-alerts/rules/{id}` | Read, update, delete. Updates reset state to `ok` |
| `POST /api/v1/corridor-alerts/rules/{id}/toggle` | Activate / deactivate |
| `POST /api/v1/corridor-alerts/rules/validate` | Validate without saving (live feedback for the builder) |
| `POST /api/v1/corridor-alerts/rules/backtest` | §5 |
| `GET /api/v1/corridor-alerts/rules/schema` | JSON Schema |

The `/configs` endpoints keep working for one release and are marked `deprecated` in OpenAPI. Writes to `/configs` are translated into rules.

### 7. UI builder

Replace the fixed form in `frontend/src/app/[locale]/corridor-alerts/page.tsx` with a rule builder:

- **Scope picker**: specific corridors (search), all corridors, or by anchor.
- **Condition tree**: rows of `[metric] [aggregation] over [window] [op] [value]`, grouped by "All of / Any of" blocks that can nest. Show units inline (%, ms, $), and convert % input to 0–1 for `success_rate`.
- **Timing**: "for", cooldown, and resolve-after, as presets plus custom.
- **Plain-English preview**, generated from the AST: *"Fire when, for 15 minutes, average success rate over 15m is below 95% and total volume over 15m is above $50,000."*
- **Backtest panel**: a "Test against last 30 days" button. It shows the fire count, a timeline strip per corridor with firing intervals, and the coverage warning. Users can adjust thresholds and re-run before saving.
- **Advanced**: a JSON editor tab (validated against the schema) for power users. It round-trips with the visual builder.

---

## Test Plan

**Unit (`alert_rules/`)**
- Parse and serialize round-trip for every node type; the JSON Schema test fails CI on drift.
- Validation: depth and leaf limits, metric/agg compatibility, the success-rate unit check, window bounds.
- Each aggregation on fixed series: `avg`, `weighted_avg` (low-volume intervals weigh less), `p95`, `rate_of_change`, `compare_to` offset.
- Three-valued logic: `all`/`any`/`not` with `None`, so a data gap never fires and never resolves.
- State machine: `for` duration, cooldown suppression, `resolve_after`, and flapping input (true/false alternating, which must not fire if `for` > 1 tick).

**Equivalence**
- Golden test: for a recorded snapshot series, converted legacy configs fire on exactly the same ticks as the old `evaluate_corridor`.
- Property test (`proptest`): for random rules and random series, backtest firings equal the firings from step-by-step live evaluation.

**Integration**
- The snapshot job writes rows, the monitor tick fires the rule, and an event appears in `/events` and on the WebSocket.
- A global-scope rule fires for every corridor (regression for A2).
- The backtest respects range limits, `truncated`, coverage, and rate limits.
- Restart mid-`pending`: state persists, and the rule fires on schedule after restart.

**Frontend**
- Builder output validates against the schema; the JSON tab round-trips.
- Percent-to-ratio conversion for success rate.

---

## Rollout

1. Ship the snapshot writer (§1) on its own first. It is useful immediately and starts building backtest history.
2. Ship the AST, evaluator, rule storage and API, with legacy configs auto-converted. The old monitor body is removed in the same PR, guarded by the equivalence test.
3. Ship the backtest endpoint.
4. Ship the UI builder.
5. Update [SLOs_AND_ALERTING.md](./SLOs_AND_ALERTING.md) and the README's alerts section; publish the schema.

## Acceptance Criteria

- [ ] Corridor snapshots are written in production at the configured interval, with retention.
- [ ] Rules support AND/OR/NOT, per-leaf windows and aggregations, `for`, cooldown and resolve.
- [ ] JSON Schema is published and enforced; invalid rules return `422` with a pointer.
- [ ] Live evaluation and backtest share the same evaluator, with property-tested equivalence.
- [ ] The backtest returns fire counts, intervals and coverage for up to 35 days.
- [ ] Legacy configs are converted with identical firing behaviour.
- [ ] The UI builder includes a preview and an inline backtest.

## References

- [Prometheus alerting rules: `for` and `keep_firing_for` semantics](https://prometheus.io/docs/prometheus/latest/configuration/alerting_rules/)
- [schemars](https://docs.rs/schemars)
