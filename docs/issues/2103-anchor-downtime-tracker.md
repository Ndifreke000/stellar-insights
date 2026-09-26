# Issue #2103: Anchor Downtime Tracker and Alerting

**Priority:** High  
**Type:** Feature  
**Component:** Backend + Frontend  
**Labels:** `enhancement`, `monitoring`, `anchors`

## Description

Track anchor uptime/downtime with historical records. Alert users when anchors go offline or experience issues. Provide uptime SLA tracking.

An anchor is only useful if its endpoints answer. A wallet routing a deposit to an anchor whose SEP-24 transfer server has been down for six hours will fail the payment and blame the network. Today the platform scores anchors on *payment* reliability after the fact; this issue adds **active probing** so degradation is detected in minutes rather than inferred from failed transactions, and so uptime can be reported against a stated SLA.

## Current Behavior

- Anchor records exist (`backend/migrations/001_create_anchors.sql`, `backend/src/api/anchors.rs`) but carry no liveness state.
- `backend/src/services/stellar_toml.rs` fetches `stellar.toml` on demand only — never on a schedule, never recorded.
- Nothing actively probes anchor endpoints. Anchor signals are all derived after the fact from payment outcomes, so an outage only surfaces once payments have already failed.
- Alerting infrastructure now exists and is anchor-aware in shape but not in substance: `backend/src/alerts.rs` carries `AnchorStatusChange` and `AnchorMetricChange` variants, and `Alert` already has optional `corridor_id` / `anchor_id`. What is missing is any producer of anchor liveness events — the variants fire only on metric deltas, never on an endpoint going down.
- The corridor alerting stack landed in #2210 (`backend/src/services/alert_manager.rs`, `alert_service.rs`, `webhook_event_service.rs`, migrations `040`–`042`) covers corridors only. Anchors have no equivalent config, cooldown, or delivery path.
- No probe history, no incident records, no SLA reporting, and no way for a user to subscribe to a specific anchor.

## Expected Behavior

- Every registered anchor is probed on a fixed interval across all the SEP endpoints it advertises.
- Each probe records status, latency, and HTTP/protocol error detail.
- Consecutive failures open an **incident**; recovery closes it, producing a durable outage history.
- Uptime percentages over 24h / 7d / 30d / 90d rolling windows, computed from probe coverage rather than assumed.
- SLA targets per anchor, with remaining error budget for the current period.
- Users subscribe to anchors and receive alerts on down / degraded / recovered transitions through the existing notification channels.
- A live status board showing every anchor and its per-service health.

## Affected Files

**Backend**
- **New file:** `backend/src/services/anchor_health.rs` — probe execution, status machine, incident lifecycle.
- **New file:** `backend/src/services/anchor_uptime.rs` — uptime/SLA aggregation.
- **New file:** `backend/src/api/anchor_health.rs` — status, uptime, incident, and subscription handlers.
- **New migration:** `backend/migrations/044_create_anchor_health.sql`
- **Update:** `backend/src/services/alert_manager.rs`, `alert_service.rs` — produce anchor liveness alerts through the existing dispatch path.
- **Update:** `backend/src/services/webhook_event_service.rs` — emit `anchor.status_changed` on probe-driven transitions.
- **Update:** `backend/src/services/stellar_toml.rs` — reuse for endpoint discovery during probing.
- **Update:** `backend/src/jobs/scheduler.rs` — register `anchor_health_probe` and `anchor_uptime_rollup`.
- **Update:** `backend/src/api/anchors.rs` — surface `health` on anchor payloads.
- **Update:** `backend/src/api/mod.rs`, `backend/src/websocket.rs`, `backend/src/openapi.rs`
- **Update:** `backend/src/telegram/`, `backend/src/email/` — anchor alert templates.

**Frontend**
- **New file:** `frontend/src/components/anchors/AnchorStatusBoard.tsx`
- **New file:** `frontend/src/components/anchors/UptimeTimeline.tsx` — 90-day bar strip.
- **New file:** `frontend/src/components/anchors/IncidentHistory.tsx`
- **New file:** `frontend/src/components/anchors/SlaCard.tsx`
- **New file:** `frontend/src/components/anchors/AnchorSubscribeButton.tsx`
- **New file:** `frontend/src/services/anchorHealth.ts`
- **Update:** `frontend/src/app/[locale]/anchors/page.tsx`
- **Update:** `frontend/src/components/anchors/AnchorCard.tsx`, `AnchorHeader.tsx`
- **Update:** `frontend/src/components/health/health-dashboard.tsx`, `status-badge.tsx`
- **Update:** `frontend/src/components/AlertNotifications.tsx`, `frontend/src/app/[locale]/notifications/`

## What Gets Probed

Endpoints are discovered from the anchor's `stellar.toml` via the existing `stellar_toml` service, then probed independently so a single broken service does not mark the whole anchor down.

| Service | Probe | Healthy |
|---------|-------|---------|
| TOML | `GET /.well-known/stellar.toml` | 200, parses, expected `NETWORK_PASSPHRASE` |
| SEP-6 | `GET {TRANSFER_SERVER}/info` | 200, JSON with `deposit`/`withdraw` |
| SEP-24 | `GET {TRANSFER_SERVER_SEP0024}/info` | 200, JSON with `deposit`/`withdraw` |
| SEP-31 | `GET {DIRECT_PAYMENT_SERVER}/info` | 200, JSON with `receive` |
| SEP-10 | `GET {WEB_AUTH_ENDPOINT}?account=<probe account>` | 200, returns a parseable challenge tx |
| SEP-12 | `GET {KYC_SERVER}/customer` (unauthenticated) | 400/401 — reachable and rejecting correctly |
| Horizon account | `GET /accounts/{issuer}` | 200, account exists |

SEP-10 probing uses a dedicated read-only probe account (`ANCHOR_PROBE_ACCOUNT`); the challenge is requested and validated but **never signed or submitted**. If the variable is unset, SEP-10 is probed for reachability only.

### Status derivation

Per service:

```
up        -> probe succeeded within ANCHOR_PROBE_TIMEOUT_MS (default 10000)
degraded  -> succeeded but latency > degraded_threshold_ms (default 3000),
             or returned a valid response with a partial/invalid body
down      -> non-2xx (where 2xx is expected), timeout, TLS failure, or DNS failure
unknown   -> not advertised in stellar.toml, or probe skipped
```

Per anchor, rolled up: `down` if TOML or every advertised transfer service is down; `degraded` if any advertised service is down or degraded; otherwise `up`.

### Flap suppression

A status change is only committed after `ANCHOR_FAILURE_THRESHOLD` consecutive probes agree (default `3`, so ~3 minutes at the default 60s interval). Recovery requires `ANCHOR_RECOVERY_THRESHOLD` consecutive successes (default `2`). This keeps a single dropped request from opening an incident and paging every subscriber.

## Alerting

Most of the plumbing already exists — this issue supplies the missing *producer* and follows the conventions #2210 established for corridors rather than inventing parallel ones.

`AlertType` in `backend/src/alerts.rs` already has `AnchorStatusChange` and `AnchorMetricChange`, and `Alert` already carries optional `corridor_id` / `anchor_id`. **No breaking payload change is required.** Anchor liveness maps onto the existing variants, with the transition carried in the alert body:

```rust
// Existing variants, newly produced by the health probe:
AlertType::AnchorStatusChange   // down | degraded | recovered transitions
AlertType::AnchorMetricChange   // SLA error budget exhausted
```

Severity and transition (`down` / `degraded` / `recovered` / `sla_breach`) travel as fields on the alert rather than as new enum variants, keeping the WebSocket payload stable for `AlertNotifications.tsx`.

Delivery reuses what already exists:

- **Alert dispatch** — `backend/src/services/alert_manager.rs` and `alert_service.rs`, the same path corridors use
- **WebSocket** — `backend/src/websocket.rs`, topic `anchor.status`
- **Webhooks** — `WebhookEventType::AnchorStatusChanged` (`anchor.status_changed`), already wired through `backend/src/services/webhook_event_service.rs`
- **Telegram** — `backend/src/telegram/`
- **Email** — `backend/src/email/`

Subscriptions follow the `corridor_alert_configs` shape from migration `041` — boolean `notify_*` columns and a `cooldown_seconds`, not a JSON channel array — so both alert families are configured and rate limited the same way. Cooldown defaults to `ANCHOR_ALERT_REMINDER_HOURS` (default `6`) while an incident stays open; recovery always notifies.

## Uptime and SLA

Uptime is computed from probe coverage, not from wall-clock assumption:

```
uptime_pct = (probes_up + probes_degraded * degraded_weight) / probes_total * 100
```

`degraded_weight` defaults to `0.5` and is configurable per anchor. Windows where the probe itself did not run (our outage, not theirs) are excluded from both numerator and denominator, and reported separately as `coverage_pct` so an SLA figure can never be inflated by our own downtime.

Error budget for a target `T` over a period of `N` probe-minutes:

```
budget_minutes    = N * (1 - T)
consumed_minutes  = downtime_minutes_in_period
remaining_pct     = (budget_minutes - consumed_minutes) / budget_minutes * 100
```

`AnchorSlaBreach` fires when `remaining_pct` crosses zero within the current period.

## Data Model

`backend/migrations/044_create_anchor_health.sql`:

```sql
CREATE TABLE anchor_health_checks (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    anchor_id      TEXT NOT NULL,
    service        TEXT NOT NULL,          -- toml | sep6 | sep24 | sep31 | sep10 | sep12 | horizon
    status         TEXT NOT NULL,          -- up | degraded | down | unknown
    latency_ms     INTEGER,
    http_status    INTEGER,
    error_kind     TEXT,                   -- timeout | dns | tls | http | body | none
    error_detail   TEXT,
    checked_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anchor_id) REFERENCES anchors (id) ON DELETE CASCADE
);

CREATE INDEX idx_health_anchor_time ON anchor_health_checks (anchor_id, checked_at DESC);
CREATE INDEX idx_health_service_time ON anchor_health_checks (anchor_id, service, checked_at DESC);

CREATE TABLE anchor_incidents (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    anchor_id         TEXT NOT NULL,
    service           TEXT NOT NULL,
    severity          TEXT NOT NULL,       -- degraded | down
    started_at        TIMESTAMP NOT NULL,
    resolved_at       TIMESTAMP,
    duration_seconds  INTEGER,
    failed_probes     INTEGER NOT NULL DEFAULT 0,
    error_kind        TEXT,
    error_detail      TEXT,
    notified_at       TIMESTAMP,
    FOREIGN KEY (anchor_id) REFERENCES anchors (id) ON DELETE CASCADE
);

CREATE INDEX idx_incident_anchor ON anchor_incidents (anchor_id, started_at DESC);
CREATE INDEX idx_incident_open ON anchor_incidents (resolved_at) WHERE resolved_at IS NULL;

CREATE TABLE anchor_uptime_daily (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    anchor_id        TEXT NOT NULL,
    service          TEXT NOT NULL,
    day              DATE NOT NULL,
    probes_total     INTEGER NOT NULL,
    probes_up        INTEGER NOT NULL,
    probes_degraded  INTEGER NOT NULL,
    probes_down      INTEGER NOT NULL,
    uptime_pct       REAL NOT NULL,
    coverage_pct     REAL NOT NULL,
    avg_latency_ms   INTEGER,
    p95_latency_ms   INTEGER,
    UNIQUE (anchor_id, service, day),
    FOREIGN KEY (anchor_id) REFERENCES anchors (id) ON DELETE CASCADE
);

CREATE TABLE anchor_sla_targets (
    anchor_id           TEXT PRIMARY KEY,
    target_uptime_pct   REAL NOT NULL DEFAULT 99.0,
    period              TEXT NOT NULL DEFAULT 'monthly',   -- monthly | quarterly
    degraded_weight     REAL NOT NULL DEFAULT 0.5,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anchor_id) REFERENCES anchors (id) ON DELETE CASCADE
);

-- Mirrors corridor_alert_configs (migration 041) so both alert families
-- share one configuration and cooldown model.
CREATE TABLE anchor_alert_configs (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL,
    anchor_id         TEXT NOT NULL,
    name              TEXT NOT NULL,
    min_severity      TEXT NOT NULL DEFAULT 'down',   -- degraded | down
    cooldown_seconds  INTEGER DEFAULT 21600,
    notify_email      BOOLEAN NOT NULL DEFAULT 0,
    notify_webhook    BOOLEAN NOT NULL DEFAULT 0,
    notify_in_app     BOOLEAN NOT NULL DEFAULT 1,
    notify_slack      BOOLEAN NOT NULL DEFAULT 0,
    notify_telegram   BOOLEAN NOT NULL DEFAULT 0,
    is_active         BOOLEAN NOT NULL DEFAULT 1,
    last_triggered_at TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (user_id, anchor_id),
    FOREIGN KEY (anchor_id) REFERENCES anchors (id) ON DELETE CASCADE
);
```

Raw `anchor_health_checks` rows are retained 30 days (`ANCHOR_PROBE_RETENTION_DAYS`); the nightly rollup fills `anchor_uptime_daily`, which is kept indefinitely and backs all long-window queries.

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/anchors/status` | Live status board: every anchor, per-service status, current incidents. |
| GET | `/api/anchors/:id/health` | Current per-service status and latest probe detail. |
| GET | `/api/anchors/:id/uptime?window=30d` | Uptime, coverage, and latency percentiles. `24h`\|`7d`\|`30d`\|`90d`. |
| GET | `/api/anchors/:id/uptime/daily?days=90` | Per-day series for the timeline strip. |
| GET | `/api/anchors/:id/incidents?limit=50&status=open` | Incident history. |
| GET | `/api/anchors/:id/sla` | Target, current-period uptime, error budget remaining. |
| PUT | `/api/anchors/:id/sla` | Set target and period (admin; audited via `admin_audit_log`). |
| POST | `/api/anchors/:id/subscribe` | Subscribe the caller; body mirrors `corridor_alert_configs` (`notify_*`, `min_severity`, `cooldown_seconds`). |
| DELETE | `/api/anchors/:id/subscribe` | Unsubscribe. |
| GET | `/api/anchors/uptime/leaderboard?window=30d` | Anchors ranked by uptime. |

**WebSocket:** topic `anchor.status`, emitted on every committed transition.

### Response — `GET /api/anchors/:id/uptime?window=30d`

```json
{
  "anchor_id": "anchor_example",
  "window": "30d",
  "uptime_pct": 99.62,
  "coverage_pct": 99.98,
  "downtime_minutes": 164,
  "incident_count": 3,
  "longest_incident_minutes": 92,
  "mttr_minutes": 54.7,
  "services": {
    "toml":  { "uptime_pct": 100.0, "avg_latency_ms": 142, "p95_latency_ms": 310 },
    "sep24": { "uptime_pct": 99.41, "avg_latency_ms": 486, "p95_latency_ms": 2180 },
    "sep10": { "uptime_pct": 99.88, "avg_latency_ms": 221, "p95_latency_ms": 640 }
  },
  "sla": {
    "target_uptime_pct": 99.0,
    "period": "monthly",
    "period_uptime_pct": 99.62,
    "error_budget_remaining_pct": 62.1,
    "status": "within_target"
  }
}
```

### Errors

- **404** — unknown anchor id.
- **409** — `PUT /sla` with a target outside `[90.0, 100.0)`.
- **422** — subscribe with an unconfigured channel (e.g. Telegram before the user links a chat).

## UI Structure

```
┌────────────────────────────────────────────────────────────┐
│ Anchor Status                       3 of 24 degraded       │
├────────────────────────────────────────────────────────────┤
│ Anchor            TOML  SEP-6 SEP-24 SEP-31 SEP-10  30d    │
│ ───────────────────────────────────────────────────────    │
│ Example Anchor     ●     ●      ●      –      ●    99.62%  │
│ Acme Money         ●     –      ▲      ●      ●    98.10%  │
│ Beta Transfer      ●     ●      ✕      –      ●    91.44%  │
│                                                            │
│ ● up   ▲ degraded   ✕ down   – not advertised              │
├────────────────────────────────────────────────────────────┤
│ Beta Transfer                                  [Subscribe] │
│                                                            │
│ SEP-24 transfer server down · started 1h 12m ago           │
│ HTTP 502 from https://transfer.beta.example/info           │
├────────────────────────────────────────────────────────────┤
│ Uptime · last 90 days                              91.44%  │
│                                                            │
│ ████████████████▌███████████▌██  ██████▌███████████████    │
│  ▲ Jun 14           ▲ Jul 02      ▲ Aug 19                 │
│ 90 days ago                                       today    │
├────────────────────────────────────────────────────────────┤
│ SLA · monthly target 99.0%                                 │
│                                                            │
│  Period uptime   91.44%          Error budget              │
│  Downtime      6h 09m            ▓▓▓▓▓▓▓▓▓▓  exhausted     │
│                                                            │
│  ⚠ Target breached on Aug 19                               │
├────────────────────────────────────────────────────────────┤
│ Incident history                                           │
│                                                            │
│ Aug 27 10:02  SEP-24  down       ongoing      HTTP 502     │
│ Aug 19 03:41  SEP-24  down       4h 22m       timeout      │
│ Jul 02 15:10  TOML    degraded   38m          slow (4.1s)  │
└────────────────────────────────────────────────────────────┘
```

- Status is never conveyed by colour alone — every cell carries a glyph and an `aria-label` naming the state.
- `UptimeTimeline` renders one bar per day, greyed where `coverage_pct` was low, so gaps in our own probing read as unknown rather than as uptime.
- The subscribe control reuses the existing notification-preferences flow rather than introducing a second one.

## Acceptance Criteria

- [ ] Migration `044_create_anchor_health.sql` applies cleanly and is idempotent
- [ ] `anchor_health` service discovers endpoints from `stellar.toml` and probes each advertised service independently
- [ ] Probes honour timeout, and classify `timeout` / `dns` / `tls` / `http` / `body` failures distinctly
- [ ] Flap suppression: status commits only after N consecutive agreeing probes; thresholds are env-configurable
- [ ] Incidents open on committed failure and close on committed recovery, with duration persisted
- [ ] SEP-10 probing validates the challenge without ever signing or submitting it
- [ ] Probe transitions produce `AnchorStatusChange` alerts, and SLA breach produces `AnchorMetricChange`, through the existing `alert_manager` path
- [ ] Existing WebSocket alert payload shape preserved — no change required in `AlertNotifications.tsx`
- [ ] `anchor.status_changed` webhook emitted on probe-driven transitions via `webhook_event_service`
- [ ] `anchor_alert_configs` mirrors the `corridor_alert_configs` shape; cooldown honoured by the same logic
- [ ] Alerts delivered over WebSocket, webhook, Telegram, and email per subscription
- [ ] Alert rate limiting: one per transition, plus a configurable reminder cadence while open
- [ ] Uptime excludes windows with no probe coverage and reports `coverage_pct` separately
- [ ] SLA error budget computed per configured period; breach fires `AnchorSlaBreach`
- [ ] Nightly rollup fills `anchor_uptime_daily` and prunes raw probes past retention
- [ ] All ten endpoints implemented with documented error cases
- [ ] `PUT /api/anchors/:id/sla` restricted to admins and written to `admin_audit_log`
- [ ] Status board, uptime timeline, SLA card, incident history, and subscribe control shipped
- [ ] `AnchorCard` and `health-dashboard` show live status
- [ ] Probe traffic is rate limited and backs off per host; an anchor is never hammered during an outage
- [ ] Backend and frontend tests added; `docs/ANCHOR_UPTIME.md` written
- [ ] OpenAPI spec updated

## Implementation Steps

1. **Schema** — write migration `044`, add query helpers under `backend/src/db/`.
2. **Probe engine** — `backend/src/services/anchor_health.rs`: endpoint discovery via `stellar_toml`, concurrent per-service probes with a bounded worker pool, per-host backoff, error classification.
3. **Status machine** — consecutive-agreement thresholds, incident open/close, transition events. Keep the state transition logic pure and unit-tested against synthetic probe sequences.
4. **Scheduler** — register `anchor_health_probe` (default 60s) and `anchor_uptime_rollup` (nightly) in `backend/src/jobs/scheduler.rs`.
5. **Alerting** — extend `alerts.rs`; wire the four new types into WebSocket, webhook, Telegram, and email; implement subscription lookup and rate limiting.
6. **Uptime service** — `backend/src/services/anchor_uptime.rs`: window aggregation, coverage, MTTR, SLA error budget.
7. **API** — `backend/src/api/anchor_health.rs`; register in `mod.rs`; extend the anchor payloads in `anchors.rs` with a `health` block.
8. **Frontend service** — `frontend/src/services/anchorHealth.ts` with socket subscription and REST fallback.
9. **Frontend components** — status board, uptime timeline, SLA card, incident history, subscribe button; integrate into the anchors page and health dashboard.
10. **Testing** — probe classification tests, state-machine tests over synthetic sequences, alert dedupe tests, API integration tests, component tests.

## Considerations

- **Probing is outbound traffic to third parties.** Rate limit per host, back off on repeated failure, and set a descriptive `User-Agent` so anchor operators can identify us. Probing an anchor harder *because* it is down is exactly the wrong behaviour.
- **Distinguish their outage from ours.** If the probe job did not run, that window is `unknown`, not `down`. Conflating the two produces uptime figures we cannot defend.
- **SLA numbers are quasi-public claims about someone else's service.** Every figure must be reproducible from stored probe records, and the methodology (probe interval, degraded weight, coverage handling) stated in the UI.
- **A single dropped request is not an outage.** Flap suppression is not optional; without it the alerting is noise and users mute it.
- **Never sign anything during a probe.** SEP-10 challenges are requested and parsed, never signed — a probe account that signs is a probe account that can be replayed.
- **Anchors advertise different service sets.** `unknown` for a service an anchor never claimed must not drag its uptime down.

## References

- [SEP-1 stellar.toml](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0001.md)
- [SEP-10 Stellar Authentication](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0010.md)
- [SEP-24 Interactive Deposit and Withdrawal](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0024.md)
- Internal: `docs/SEP24.md`, `docs/SEP31.md`, `docs/SEP6_UI.md`, `docs/OBSERVABILITY.md`
- Prior art: Atlassian Statuspage uptime semantics; Google SRE workbook on error budgets

## Related Issues

- Related to: #2102 Network Congestion Indicator (shares the monitoring surface and alert plumbing)
- Related to: Issue #022 Anchor Reliability Scoring Algorithm Enhancement — probe history is a strong new feature for that model
- Depends on: existing `backend/src/services/stellar_toml.rs`
- Builds on: #2210 corridor performance alerts — reuses `alert_manager`, `alert_service`, and the `corridor_alert_configs` conventions
- Completes: `AnchorStatusChangedEvent`, which is wired for delivery but has no liveness producer

## Estimated Effort

- Schema + probe engine: 1.5 days
- Status machine, incidents, flap suppression: 1 day
- Alerting via existing dispatch + subscription configs: 0.5 days
- Uptime/SLA aggregation + API: 1 day
- Frontend components and integration: 1 day
- Testing, docs, polish: 0.5 days
- **Total: 5.5 days**
