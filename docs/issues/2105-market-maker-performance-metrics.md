# Issue #2105: Market Maker Performance Metrics

**Priority:** Medium  
**Type:** Feature  
**Component:** Backend + Frontend  
**Labels:** `enhancement`, `analytics`, `market-makers`

## Description

Track and analyze market maker performance on Stellar DEX. Measure spread consistency, quote depth, uptime, and profitability indicators.

Payment reliability on Stellar depends on someone standing ready to trade. Path payments cross the DEX, and when the makers quoting a pair widen out or step away, corridors that looked healthy start failing. This issue makes the makers themselves measurable: who quotes which pairs, how tightly, how deep, how consistently, and whether their quoting appears to be working out for them.

Stellar makes this tractable in a way most venues do not — orderbook offers are public and carry a `seller` account, so quotes can be attributed to specific participants without any private feed.

## Current Behavior

- No concept of a market maker anywhere in the codebase.
- `RpcClient::fetch_order_book()` returns aggregated price levels with no account attribution, and nothing polls it on a schedule.
- `fetch_trades()` exists and is used for corridor volume, but trades are never analysed by counterparty.
- No offer-level data is fetched or stored, so there is no record of who was quoting when.
- Liquidity analytics (`liquidity_pool_analyzer.rs`) cover AMM pools only; the orderbook side of the DEX has no participant-level view.
- Nothing distinguishes a pair held up by one maker from a pair with genuine competition — a concentration risk that is currently invisible.

## Expected Behavior

- Automatic identification of market-making accounts from public offer and trade activity.
- Per-maker, per-pair metrics: quoted spread, depth at fixed bands, two-sided presence, and uptime.
- Spread *consistency*, not just an average — variance and time-within-target matter more than a good median.
- Fill and volume statistics, including each maker's share of a pair.
- Profitability **indicators** — realized spread capture, inventory drift, and post-fill markouts — explicitly framed as inferences from public data, never as reported P&L.
- A composite performance score with a fully visible breakdown.
- Per-pair maker rosters showing concentration: how much of a pair rests on a single account.

## Affected Files

**Backend**
- **New file:** `backend/src/services/market_maker.rs` — identification, snapshot capture, attribution.
- **New file:** `backend/src/services/mm_metrics.rs` — spread, depth, uptime, fills, markouts, scoring.
- **New file:** `backend/src/api/market_makers.rs` — handlers and router.
- **New migration:** `backend/migrations/046_create_market_makers.sql`
- **Update:** `backend/src/rpc/stellar.rs` — add `fetch_offers_for_pair()` and `fetch_account_offers()`.
- **Update:** `backend/src/jobs/scheduler.rs` — register `mm_snapshot`, `mm_fill_ingest`, `mm_daily_rollup`.
- **Update:** `backend/src/api/mod.rs`, `backend/src/openapi.rs`
- **Update:** `backend/src/services/mod.rs`

**Frontend**
- **New file:** `frontend/src/app/[locale]/market-makers/page.tsx`
- **New file:** `frontend/src/components/market-makers/MakerLeaderboard.tsx`
- **New file:** `frontend/src/components/market-makers/MakerProfile.tsx`
- **New file:** `frontend/src/components/market-makers/SpreadConsistencyChart.tsx`
- **New file:** `frontend/src/components/market-makers/QuoteDepthChart.tsx`
- **New file:** `frontend/src/components/market-makers/UptimeStrip.tsx`
- **New file:** `frontend/src/components/market-makers/PairMakerRoster.tsx`
- **New file:** `frontend/src/services/marketMakers.ts`
- **Update:** `frontend/src/components/layout/sidebar.tsx`
- **Update:** `frontend/src/app/[locale]/liquidity/page.tsx` — link maker rosters from pair views.

## Data Collection

Two independent streams, both from public Horizon data.

### 1. Quote snapshots (the expensive part)

Horizon `GET /offers?selling_asset_type=…&buying_asset_type=…` returns individual offers **including the `seller` account** — this is what makes per-account attribution possible. For each tracked pair, on each interval:

1. Page offers for both directions of the pair.
2. Group by `seller`.
3. For each seller, compute best bid, best ask, quoted spread, and depth at 10/25/50/100 bps from their own levels.
4. Persist one row per (maker, pair, snapshot).

Interval is `MM_SNAPSHOT_INTERVAL_SECONDS` (default `60`). Pair set is capped by `MM_MAX_PAIRS` (default `100`), selected by trade volume and corridor relevance — the same pressure as #2104, and the two jobs should share a tracked-pair registry rather than each building their own.

### 2. Fills

Horizon `GET /trades` returns `base_account`, `counter_account`, `base_offer_id`, `counter_offer_id`, and `base_is_seller`. Ingested continuously by cursor; a trade is attributed to a maker when their account appears as the resting side. Fills feed volume, fill rate, and markouts.

## Identifying a Market Maker

An account is classified as a maker for a pair when, over a 7-day observation window, it meets **all** of:

| Criterion | Default threshold | Env |
|-----------|------------------|-----|
| Snapshots with a live offer on the pair | ≥ 25% | `MM_MIN_PRESENCE_PCT` |
| Distinct days with activity | ≥ 3 | `MM_MIN_ACTIVE_DAYS` |
| Offer updates (place/cancel/modify) | ≥ 50 | `MM_MIN_OFFER_UPDATES` |
| Two-sided quoting in some snapshots | ≥ 10% | `MM_MIN_TWO_SIDED_PCT` |

Thresholds are conservative by design: a long-lived single resting offer is a passive holder, not a maker, and classifying it as one pollutes every pair-level statistic. Classification is recomputed daily and is per-pair — an account can make one pair and merely hold another.

## Metrics

### Spread consistency

Per maker, per pair, over a window, sampled at each snapshot where the maker is two-sided:

```
quoted_spread_bps = (best_ask - best_bid) / mid * 10000

median_spread_bps
p90_spread_bps
spread_stdev_bps
time_within_target_pct = snapshots with spread <= target_bps / two_sided_snapshots
consistency_score      = 100 * (1 - clamp(spread_stdev_bps / median_spread_bps, 0, 1))
```

`consistency_score` is a coefficient-of-variation measure: a maker holding 30 bps steadily scores far above one alternating between 5 and 90 bps, even though their medians may match. That is the intended ranking — predictable quotes are what a payment router can rely on.

### Quote depth

Time-weighted mean of the maker's own notional (USD, via `price_feed`) resting within each band, on each side:

```
depth_{band}_bid_usd, depth_{band}_ask_usd   for band in {10, 25, 50, 100} bps
depth_imbalance = (bid - ask) / (bid + ask)      // -1 all ask, +1 all bid
```

Persistent one-sided imbalance is reported plainly — it usually signals inventory constraints rather than a strategy choice.

### Uptime and presence

```
presence_pct   = snapshots with any live offer / total snapshots
two_sided_pct  = snapshots quoting both sides / total snapshots
longest_gap_minutes
gap_count      = distinct absences longer than MM_GAP_MINUTES (default 5)
```

Snapshots missed because *our* job did not run are excluded from the denominator and surfaced as `coverage_pct`, exactly as in #2103. A maker must never be penalised for our outage.

### Fills and share

```
fill_count, fill_volume_usd
maker_volume_share  = maker fill volume / total pair volume
fill_rate           = fills / offer updates       // churn indicator
quote_to_fill_ratio = time quoting / fills        // how much quoting a fill costs
```

### Profitability indicators

Framed throughout as inferred, not reported:

- **Realized spread capture** — for each fill, signed distance from the pair mid at fill time, volume-weighted in bps. Positive means filling on the favourable side of mid.
- **Markout** — mid price movement at 1m / 5m / 30m after each fill, signed by trade direction. Persistently negative markouts indicate adverse selection: the maker is being picked off by informed flow.
- **Inventory drift** — net position change in the pair's base asset over the window, from attributed fills. Large drift means the maker is accumulating rather than turning inventory.

```
inferred_edge_bps = realized_spread_capture_bps + markout_5m_bps
```

This is a directional indicator built from public trade data. It excludes rebalancing costs, hedges held off-network, and any off-DEX activity, and the UI says so wherever the number appears.

### Composite score

```
mm_score = 100 * (0.30 * norm(consistency_score)
                + 0.25 * norm(two_sided_pct)
                + 0.20 * norm(depth_50bps_usd)
                + 0.15 * norm(presence_pct)
                + 0.10 * norm(1 - abs(depth_imbalance)))
```

`norm()` is a percentile rank against other makers on the same pair, so a thin pair's best maker is not punished for the pair's size. Weights live in a config struct; the API always returns the component breakdown alongside the score. **Profitability indicators are deliberately excluded from the score** — they are the least certain figures here and should not drive a public ranking.

## Data Model

`backend/migrations/046_create_market_makers.sql`:

```sql
CREATE TABLE mm_quote_snapshots (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id         TEXT NOT NULL,
    pair_key           TEXT NOT NULL,
    captured_at        TIMESTAMP NOT NULL,
    best_bid           REAL,
    best_ask           REAL,
    mid_price          REAL,
    spread_bps         REAL,
    two_sided          INTEGER NOT NULL DEFAULT 0,
    depth_10bps_bid_usd  REAL NOT NULL DEFAULT 0,
    depth_10bps_ask_usd  REAL NOT NULL DEFAULT 0,
    depth_25bps_bid_usd  REAL NOT NULL DEFAULT 0,
    depth_25bps_ask_usd  REAL NOT NULL DEFAULT 0,
    depth_50bps_bid_usd  REAL NOT NULL DEFAULT 0,
    depth_50bps_ask_usd  REAL NOT NULL DEFAULT 0,
    depth_100bps_bid_usd REAL NOT NULL DEFAULT 0,
    depth_100bps_ask_usd REAL NOT NULL DEFAULT 0,
    offer_count        INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_mmq_account_pair ON mm_quote_snapshots (account_id, pair_key, captured_at DESC);
CREATE INDEX idx_mmq_pair_time ON mm_quote_snapshots (pair_key, captured_at DESC);

CREATE TABLE mm_fills (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    trade_id           TEXT NOT NULL UNIQUE,
    account_id         TEXT NOT NULL,
    pair_key           TEXT NOT NULL,
    side               TEXT NOT NULL,        -- bid | ask
    price              REAL NOT NULL,
    base_amount        REAL NOT NULL,
    volume_usd         REAL,
    mid_at_fill        REAL,
    spread_capture_bps REAL,
    markout_1m_bps     REAL,
    markout_5m_bps     REAL,
    markout_30m_bps    REAL,
    offer_id           TEXT,
    executed_at        TIMESTAMP NOT NULL
);

CREATE INDEX idx_mmf_account ON mm_fills (account_id, executed_at DESC);
CREATE INDEX idx_mmf_pair ON mm_fills (pair_key, executed_at DESC);
CREATE INDEX idx_mmf_pending_markout ON mm_fills (executed_at) WHERE markout_30m_bps IS NULL;

CREATE TABLE mm_daily_metrics (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id             TEXT NOT NULL,
    pair_key               TEXT NOT NULL,
    day                    DATE NOT NULL,
    snapshots_total        INTEGER NOT NULL,
    snapshots_present      INTEGER NOT NULL,
    snapshots_two_sided    INTEGER NOT NULL,
    presence_pct           REAL NOT NULL,
    two_sided_pct          REAL NOT NULL,
    coverage_pct           REAL NOT NULL,
    median_spread_bps      REAL,
    p90_spread_bps         REAL,
    spread_stdev_bps       REAL,
    consistency_score      REAL,
    time_within_target_pct REAL,
    avg_depth_50bps_usd    REAL,
    depth_imbalance        REAL,
    longest_gap_minutes    INTEGER,
    fill_count             INTEGER NOT NULL DEFAULT 0,
    fill_volume_usd        REAL NOT NULL DEFAULT 0,
    maker_volume_share     REAL,
    spread_capture_bps     REAL,
    markout_5m_bps         REAL,
    inventory_drift        REAL,
    mm_score               REAL,
    UNIQUE (account_id, pair_key, day)
);

CREATE INDEX idx_mmd_score ON mm_daily_metrics (pair_key, day DESC, mm_score DESC);

CREATE TABLE mm_registry (
    account_id       TEXT NOT NULL,
    pair_key         TEXT NOT NULL,
    first_seen       TIMESTAMP NOT NULL,
    last_seen        TIMESTAMP NOT NULL,
    classified_at    TIMESTAMP NOT NULL,
    is_active        INTEGER NOT NULL DEFAULT 1,
    label            TEXT,                  -- optional human label, admin-set
    PRIMARY KEY (account_id, pair_key)
);

CREATE INDEX idx_mmr_pair ON mm_registry (pair_key, is_active);
```

`mm_quote_snapshots` is the highest-volume table in the system: 100 pairs × ~10 makers × 1,440 snapshots/day is on the order of 1.4M rows/day. Raw snapshots are retained `MM_SNAPSHOT_RETENTION_DAYS` (default `14`); `mm_daily_metrics` is the long-term record and backs every window longer than 14 days.

Markouts are filled by a deferred pass: the `mm_fill_ingest` job writes fills immediately, and a follow-up job computes 1m/5m/30m markouts once enough time has elapsed, using the partial index above.

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/market-makers?pair=<pair_key>&window=30d` | Makers active on a pair, with metrics. |
| GET | `/api/market-makers/leaderboard?window=30d&limit=50` | Ranked by `mm_score` across all pairs. |
| GET | `/api/market-makers/:account` | Profile: pairs made, aggregate metrics, activity span. |
| GET | `/api/market-makers/:account/pairs/:pair_key?window=30d` | Full per-pair metric set with score breakdown. |
| GET | `/api/market-makers/:account/spread-history?pair=…&window=7d&interval=1h` | Spread series for the consistency chart. |
| GET | `/api/market-makers/:account/uptime?pair=…&window=30d` | Presence, two-sided, gaps, coverage. |
| GET | `/api/market-makers/:account/fills?pair=…&limit=100` | Recent attributed fills with markouts. |
| GET | `/api/pairs/:pair_key/makers` | Roster and concentration: HHI over maker depth share. |

### Response — `GET /api/market-makers/:account/pairs/:pair_key`

```json
{
  "account_id": "GABC...",
  "pair_key": "USDC:GA5Z.../XLM:native",
  "window": "30d",
  "classified_since": "2026-03-14T00:00:00Z",
  "spread": {
    "median_bps": 28.4,
    "p90_bps": 61.2,
    "stdev_bps": 14.9,
    "consistency_score": 47.5,
    "time_within_target_pct": 71.2,
    "target_bps": 40
  },
  "depth": {
    "avg_depth_50bps_usd": 184300.0,
    "depth_imbalance": -0.12,
    "bands": { "10bps": 42100.0, "25bps": 98400.0, "50bps": 184300.0, "100bps": 291800.0 }
  },
  "uptime": {
    "presence_pct": 96.4,
    "two_sided_pct": 88.1,
    "coverage_pct": 99.9,
    "longest_gap_minutes": 74,
    "gap_count": 6
  },
  "activity": {
    "fill_count": 4182,
    "fill_volume_usd": 8420100.0,
    "maker_volume_share": 0.31,
    "fill_rate": 0.084
  },
  "profitability_indicators": {
    "spread_capture_bps": 11.4,
    "markout_1m_bps": -2.1,
    "markout_5m_bps": -3.8,
    "markout_30m_bps": -5.2,
    "inferred_edge_bps": 7.6,
    "inventory_drift": -142000.0,
    "disclaimer": "Inferred from public DEX trades. Excludes hedges, off-DEX activity, and rebalancing costs. Not reported P&L."
  },
  "mm_score": 68.3,
  "score_components": {
    "consistency": 47.5, "two_sided": 88.1, "depth": 74.0,
    "presence": 96.4, "balance": 88.0
  }
}
```

### Errors

- **404** — account not classified as a maker on the requested pair; response states the classification criteria.
- **422** — malformed `pair_key`, or `window` longer than raw retention on a snapshot-level endpoint.
- **503** — price feed unavailable, so USD normalization cannot be performed.

## UI Structure

```
┌───────────────────────────────────────────────────────────────┐
│ Market Makers                            USDC/XLM · 30 days   │
├───────────────────────────────────────────────────────────────┤
│ #  Maker         Score  Spread  Depth 50bps  2-sided  Share   │
│ ─────────────────────────────────────────────────────────     │
│ 1  GABC…7K2       68.3    28bps      $184K     88%      31%   │
│ 2  GDEF…9M4       61.7    34bps      $142K     81%      24%   │
│ 3  GHIJ…3P1       55.2    22bps       $61K     94%      12%   │
│                                                               │
│ Concentration: top maker holds 31% of volume, HHI 2140        │
├───────────────────────────────────────────────────────────────┤
│ GABC…7K2 · USDC/XLM                    Making since Mar 2026  │
│                                                               │
│  Score 68.3                                                   │
│  Consistency  ███████▌            47.5                        │
│  Two-sided    ██████████████▌     88.1                        │
│  Depth        ████████████        74.0                        │
│  Presence     ███████████████▌    96.4                        │
│  Balance      ██████████████▌     88.0                        │
├───────────────────────────────────────────────────────────────┤
│ Spread consistency · 7 days                    target 40 bps  │
│                                                               │
│ 80┤        ╷                                                  │
│   │     ╷  │     ╷                    p90 ─ ─ ─ ─ ─ ─ ─       │
│ 40┤─ ─ ─┼─ ┼ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ target       │
│   │ ▄▄▄▄█▄▄█▄▄▄▄▄█▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄  median                │
│  0└──────────────────────────────────────                     │
│    Aug 21        Aug 24        Aug 27                         │
│                                                               │
│  71% of two-sided time within the 40 bps target.              │
├───────────────────────────────────────────────────────────────┤
│ Presence · 30 days                              96.4% quoted  │
│                                                               │
│ ███████████▌██████████████ ▌███████████████████▌█████████     │
│            ▲ 74m gap        ▲ 31m gap                         │
│ ░ = no data from us (coverage 99.9%)                          │
├───────────────────────────────────────────────────────────────┤
│ Quote depth by band                          [bid] [ask]      │
│                                                               │
│  10 bps  ████▌                              $42.1K            │
│  25 bps  ██████████▌                        $98.4K            │
│  50 bps  ███████████████████▌              $184.3K            │
│ 100 bps  ██████████████████████████████▌   $291.8K            │
├───────────────────────────────────────────────────────────────┤
│ Profitability indicators                          ⓘ inferred  │
│                                                               │
│  Spread capture    +11.4 bps      Markout 5m    -3.8 bps      │
│  Inferred edge      +7.6 bps      Inventory     -142K XLM     │
│                                                               │
│  Inferred from public DEX trades. Excludes hedges, off-DEX    │
│  activity, and rebalancing costs. Not reported P&L.           │
└───────────────────────────────────────────────────────────────┘
```

- Accounts are shown truncated with a copy control and a link to the account view; any human label is admin-set and marked as such.
- The uptime strip distinguishes maker absence from our own missing coverage — different glyph, stated in the legend.
- The profitability panel carries its disclaimer inline, not in a footnote, and its numbers never appear in the leaderboard ranking.

## Acceptance Criteria

- [ ] Migration `046_create_market_makers.sql` applies cleanly and is idempotent
- [ ] `fetch_offers_for_pair()` and `fetch_account_offers()` added to `RpcClient`, paging correctly, with tests
- [ ] Offers grouped by `seller`; per-maker best bid/ask, spread, and per-band depth computed per snapshot
- [ ] Maker classification implements all four criteria, is per-pair, and is recomputed daily
- [ ] A single long-lived resting offer is **not** classified as market making (explicit test case)
- [ ] Spread consistency computed as median, p90, stdev, CV-based score, and time-within-target
- [ ] Depth time-weighted and USD-normalized; missing price yields 503 rather than a wrong figure
- [ ] Presence/uptime excludes snapshots our job missed and reports `coverage_pct` separately
- [ ] Fills ingested by cursor from `/trades` and attributed to the resting side without duplication (`trade_id` unique)
- [ ] Markouts computed by a deferred pass at 1m/5m/30m using the partial index
- [ ] Spread capture, markouts, and inventory drift computed and labelled as inferred everywhere they appear
- [ ] Profitability indicators excluded from `mm_score`
- [ ] `mm_score` returned with its full component breakdown; weights configurable in one place
- [ ] `norm()` percentile-ranks within a pair, not across the whole network
- [ ] Pair roster endpoint reports maker concentration via HHI over depth share
- [ ] Snapshot job respects `MM_MAX_PAIRS` and Horizon rate limits; shares the tracked-pair registry with #2104
- [ ] Daily rollup fills `mm_daily_metrics`; raw snapshots pruned past retention
- [ ] Windows longer than raw retention are served from daily metrics, never silently truncated
- [ ] All eight endpoints implemented with documented error cases
- [ ] Leaderboard, profile, spread chart, depth chart, uptime strip, and pair roster shipped
- [ ] Market Makers page added to the sidebar; pair views link to rosters
- [ ] Backend and frontend tests added; `docs/MARKET_MAKERS.md` written, documenting every formula and threshold
- [ ] OpenAPI spec updated

## Implementation Steps

1. **RPC layer** — `fetch_offers_for_pair()` with paging and `fetch_account_offers()`; model the offer resource including `seller`.
2. **Snapshot pipeline** — group offers by seller, compute per-maker spread and band depths, persist. Get the volume and retention story right here before adding metrics, since this table dominates storage.
3. **Fill ingestion** — cursor-based `/trades` ingest with idempotent upsert on `trade_id`; attribute to the resting side.
4. **Classification** — `mm_registry` population from the four criteria, recomputed daily, per pair.
5. **Metrics** — `mm_metrics.rs`: spread statistics, time-weighted depth, presence/gaps, fill statistics. Pure functions, unit-tested against fixture snapshot sequences.
6. **Markouts** — deferred job computing 1m/5m/30m mid movement per fill; inventory drift from attributed fills.
7. **Scoring** — percentile normalization within pair, weighted composite, component breakdown in the response.
8. **Rollups + retention** — `mm_daily_rollup`; route long windows to daily metrics; prune raw snapshots.
9. **API** — `backend/src/api/market_makers.rs`; register in `mod.rs`; cache via `http_cache` (60s on live, 5 min on windowed).
10. **Frontend service + components** — leaderboard, profile, spread consistency chart, depth chart, uptime strip, pair roster.
11. **Page + navigation** — new market-makers page, sidebar entry, links from liquidity pair views.
12. **Testing** — classification edge cases, spread/depth maths, attribution correctness, markout timing, API integration, component tests.

## Considerations

- **These are public claims about identifiable participants.** Every metric must be reproducible from stored public data, the methodology must be documented, and inferred figures must be labelled inferred wherever they render. An account is an account, not a named firm, unless an admin has explicitly labelled it.
- **Profitability is an inference, not a measurement.** Hedges, off-DEX activity, and rebalancing costs are invisible to us. Keeping these indicators out of the ranked score is a deliberate choice, not an oversight.
- **Snapshot volume is the main engineering risk.** ~1.4M rows/day at default settings. Retention, rollup, and index design need to be settled before the metrics layer is built, not retrofitted after.
- **A resting offer is not a quote.** Without the classification thresholds, every passive holder becomes a "market maker" and every pair-level statistic is wrong.
- **Sampling at 60s misses fast quoting.** A maker cancelling and replacing within the interval looks steadier than they are. State the sampling interval in the UI so the figures are read correctly.
- **Our downtime is not their absence.** Coverage is tracked and excluded, exactly as in #2103.
- **Normalize within a pair.** A thin pair's best maker should rank on how well they serve that pair, not on absolute depth against USDC/XLM.
- **Horizon rate limits are shared.** The tracked-pair registry must be common with #2104 or the two jobs will compete for the same budget.

## References

- [Horizon `/offers`](https://developers.stellar.org/docs/data/apis/horizon/api-reference/resources/offers) — includes `seller`, the basis for attribution
- [Horizon `/trades`](https://developers.stellar.org/docs/data/apis/horizon/api-reference/resources/trades)
- [Stellar DEX and order books](https://developers.stellar.org/docs/learn/fundamentals/stellar-data-structures/orderbook)
- Markout and adverse selection: standard market-microstructure measures of realized spread
- Internal: `docs/RPC_DATA_SOURCES.md`, `backend/src/rpc/stellar.rs`

## Related Issues

- Depends on: shared tracked-pair registry with #2104 Liquidity Fragmentation Analysis
- Related to: #2104 — orderbook depth attribution is common groundwork
- Related to: Issue #027 Asset Velocity Metrics and Analysis
- Feeds: corridor health and path-payment reliability in `backend/src/services/analytics.rs`

## Estimated Effort

- RPC layer + snapshot pipeline: 1.5 days
- Fill ingestion + attribution: 1 day
- Classification + metrics: 1.5 days
- Markouts, profitability indicators, scoring: 1 day
- API + rollups + retention: 1 day
- Frontend components and page: 1.5 days
- Testing, docs, polish: 0.5 days
- **Total: 7 days**
