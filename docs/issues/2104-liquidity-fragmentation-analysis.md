# Issue #2104: Liquidity Fragmentation Analysis

**Priority:** Medium  
**Type:** Feature  
**Component:** Backend + Frontend  
**Labels:** `enhancement`, `analytics`, `liquidity`

## Description

Analyze liquidity fragmentation across the network. Identify where liquidity is concentrated, fragmented, or missing. Provide recommendations for liquidity providers.

Stellar lets the same asset pair trade in several places at once: the classic orderbook, one or more AMM pools, and — for pairs with no direct market — only via multi-hop paths through an intermediate asset. Total value locked looks healthy in aggregate while any single venue is too thin to fill a real payment. Fragmentation analysis measures that gap: how much depth *exists* for a pair, how it is split, what a trader actually gets, and where adding liquidity would do the most good.

## Current Behavior

- `backend/src/services/liquidity_pool_analyzer.rs` and `backend/src/api/liquidity_pools.rs` report pools individually — TVL, volume, fees per pool.
- `backend/migrations/009_create_liquidity_pools.sql` stores pool state; nothing joins pools to the orderbook for the same pair.
- `RpcClient` has the pieces (`fetch_liquidity_pools`, `fetch_liquidity_pool`, `fetch_pool_trades`, `fetch_order_book`, `fetch_assets`) but no caller combines orderbook and AMM depth into one view.
- `LiquidityChart`, `LiquidityHeatmap`, `TVLChart`, and `PoolPerformanceChart` all present per-pool or aggregate TVL, never per-pair distribution.
- No notion of a liquidity *gap* — a pair with real payment demand and insufficient depth is invisible.
- No guidance for liquidity providers on where to deploy.

## Expected Behavior

- Per-pair inventory of every venue holding liquidity for that pair (orderbook + each AMM pool).
- A fragmentation index per pair, plus concentration measured with a standard, defensible statistic.
- Effective depth at fixed slippage bands, computed per venue and for the optimal split across venues.
- A routing gain figure: how many basis points a split execution saves versus the single best venue.
- Liquidity gap detection: pairs where demand (payment and trade volume, path-payment attempts) outruns available depth.
- Ranked, explained recommendations for liquidity providers, optionally scoped to an account's current positions.
- Network-level rollups: which assets anchor the most liquidity, which corridors are underserved.

## Affected Files

**Backend**
- **New file:** `backend/src/services/fragmentation.rs` — venue inventory, HHI, fragmentation index, depth curves.
- **New file:** `backend/src/services/liquidity_recommendations.rs` — gap detection and LP recommendations.
- **New file:** `backend/src/api/fragmentation.rs` — handlers and router.
- **New migration:** `backend/migrations/025_create_liquidity_fragmentation.sql`
- **Update:** `backend/src/rpc/stellar.rs` — add `fetch_order_book_full()` (deeper `limit`) and `fetch_pools_for_pair()`.
- **Update:** `backend/src/services/liquidity_pool_analyzer.rs` — expose per-pair pool lookup.
- **Update:** `backend/src/jobs/scheduler.rs` — register `fragmentation_snapshot`.
- **Update:** `backend/src/api/mod.rs`, `backend/src/api/liquidity_pools.rs`, `backend/src/openapi.rs`

**Frontend**
- **New file:** `frontend/src/components/liquidity/FragmentationScorecard.tsx`
- **New file:** `frontend/src/components/liquidity/VenueBreakdown.tsx`
- **New file:** `frontend/src/components/liquidity/DepthCurveChart.tsx` — per-venue vs consolidated.
- **New file:** `frontend/src/components/liquidity/LiquidityGapTable.tsx`
- **New file:** `frontend/src/components/liquidity/LpRecommendations.tsx`
- **New file:** `frontend/src/services/fragmentation.ts`
- **Update:** `frontend/src/app/[locale]/liquidity/page.tsx`
- **Update:** `frontend/src/app/[locale]/liquidity-pools/page.tsx`
- **Update:** `frontend/src/components/charts/LiquidityHeatmap.tsx`

## Venue Model

For a pair `(A, B)`, a **venue** is any single place a trade can execute:

| Venue kind | Source | Depth from |
|------------|--------|-----------|
| `orderbook` | Horizon `GET /order_book` | Summed bid/ask levels within the band |
| `amm_pool` | Horizon `GET /liquidity_pools` | Constant-product curve from reserves |
| `path` (informational) | Horizon `GET /paths/strict-send` | Best multi-hop route when no direct venue exists |

Path liquidity is reported but **excluded from the fragmentation index** — a two-hop route is not a competing venue for the same pair, it is a fallback. Counting it would understate fragmentation exactly where it is worst.

## Metrics

### Depth at a slippage band

For each venue, the notional of asset `A` that can be sold before the execution price moves more than `x` bps from the venue's mid:

- **Orderbook:** walk levels, accumulate until the marginal price exceeds the band.
- **AMM pool:** solve the constant product `x·y = k` directly. For reserves `(r_a, r_b)` and a band `s = x/10000`, the sellable amount is `Δa = r_a · (1/√(1-s) − 1)`, taking the pool fee into account.

Bands: **10, 25, 50, 100, 200 bps**. Everything is denominated in a common unit (USD via `backend/src/services/price_feed.rs`) so venues and pairs are comparable.

### Concentration — Herfindahl-Hirschman Index

Over venue shares `s_i` of total depth at 50 bps:

```
HHI = Σ (s_i * 100)^2          // 0..10000, 10000 = one venue holds everything
fragmentation_index = 1 - HHI / 10000     // 0..1, higher = more fragmented
effective_venues = 10000 / HHI            // "as if" venue count
```

HHI is used rather than an ad-hoc score because it is standard, bounded, and interpretable: a pair split evenly across 4 venues has `HHI = 2500`, `effective_venues = 4`.

### Routing gain

The real cost of fragmentation is slippage a trader pays that a consolidated market would not charge:

```
best_venue_out   = output executing the full trade on the single deepest venue
split_out        = output executing the optimal split across all venues
routing_gain_bps = (split_out - best_venue_out) / best_venue_out * 10000
```

Computed at a standard trade size per pair (default: the pair's 30-day median trade size, floored at $1,000). A high routing gain means fragmentation is materially costing traders who do not split — and that consolidation, or better routing, would pay.

### Liquidity gap score

```
demand   = normalized 30d payment volume + trade volume + failed path-payment attempts
supply   = consolidated depth at 100 bps
gap      = demand / max(supply, epsilon)
```

Pairs are ranked by `gap`. Failed path-payment attempts are the strongest signal in the numerator — they are demand that *could not be served*, which is precisely what a gap is.

### Coverage classes

| Class | Condition |
|-------|-----------|
| `concentrated` | `effective_venues < 1.5` and depth adequate |
| `healthy` | depth adequate, `1.5 ≤ effective_venues ≤ 4` |
| `fragmented` | `effective_venues > 4` and `routing_gain_bps > 25` |
| `thin` | consolidated depth at 100 bps below the pair's demand threshold |
| `missing` | no direct venue; path-only or unroutable |

## Recommendations

`liquidity_recommendations.rs` produces ranked, *explained* suggestions. Every recommendation carries the inputs behind it — never a bare score.

| Kind | Trigger | Suggested action |
|------|---------|------------------|
| `underserved_pair` | high gap score, `thin` or `missing` | Seed or deepen a pool for this pair |
| `high_yield_pool` | high volume/TVL ratio, sustained | Add to this pool; fee capture per unit is above median |
| `consolidation_candidate` | `fragmented`, several sub-scale pools | Concentrate into the deepest pool rather than adding another |
| `imbalanced_pool` | reserve ratio far from traded mid | Rebalance; the pool is quoting off-market |
| `declining_pool` | TVL and volume both trending down | Consider exit; the pair is losing traction |

Optional `account` scoping compares an account's existing LP positions (from Horizon account balances) against the ranked list, so recommendations account for what the provider already holds.

## Data Model

`backend/migrations/025_create_liquidity_fragmentation.sql`:

```sql
CREATE TABLE pair_venue_snapshots (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key           TEXT NOT NULL,       -- canonical "CODE:ISSUER/CODE:ISSUER", sorted
    base_asset         TEXT NOT NULL,
    counter_asset      TEXT NOT NULL,
    venue_kind         TEXT NOT NULL,       -- orderbook | amm_pool
    venue_id           TEXT NOT NULL,       -- pool id, or "orderbook"
    depth_10bps_usd    REAL NOT NULL,
    depth_25bps_usd    REAL NOT NULL,
    depth_50bps_usd    REAL NOT NULL,
    depth_100bps_usd   REAL NOT NULL,
    depth_200bps_usd   REAL NOT NULL,
    mid_price          REAL,
    tvl_usd            REAL,
    fee_bps            INTEGER,
    captured_at        TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_venue_pair_time ON pair_venue_snapshots (pair_key, captured_at DESC);
CREATE INDEX idx_venue_time ON pair_venue_snapshots (captured_at DESC);

CREATE TABLE pair_fragmentation (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key              TEXT NOT NULL,
    venue_count           INTEGER NOT NULL,
    effective_venues      REAL NOT NULL,
    hhi                   REAL NOT NULL,
    fragmentation_index   REAL NOT NULL,
    consolidated_depth_usd REAL NOT NULL,
    best_venue_depth_usd  REAL NOT NULL,
    routing_gain_bps      REAL NOT NULL,
    reference_trade_usd   REAL NOT NULL,
    coverage_class        TEXT NOT NULL,
    demand_score          REAL NOT NULL,
    gap_score             REAL NOT NULL,
    computed_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (pair_key, computed_at)
);

CREATE INDEX idx_frag_gap ON pair_fragmentation (gap_score DESC, computed_at DESC);
CREATE INDEX idx_frag_class ON pair_fragmentation (coverage_class, computed_at DESC);

CREATE TABLE liquidity_recommendations (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    pair_key       TEXT NOT NULL,
    venue_id       TEXT,
    kind           TEXT NOT NULL,
    priority       REAL NOT NULL,
    rationale      TEXT NOT NULL,        -- JSON: the inputs behind the recommendation
    generated_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_reco_priority ON liquidity_recommendations (priority DESC, generated_at DESC);
```

`pair_key` is canonical and sorted so `A/B` and `B/A` never produce two rows. Snapshots run every `FRAGMENTATION_SNAPSHOT_INTERVAL_SECONDS` (default `300`) over a tracked-pair set; raw snapshots are retained 14 days, with daily aggregates kept long-term.

### Pair selection

Analysing every pair on the network per interval is not feasible against Horizon rate limits. The tracked set is: all pairs with an AMM pool above a TVL floor, plus all pairs appearing in corridor definitions, plus the top N by 30-day trade volume, plus any pair with recent failed path payments. Size is capped by `FRAGMENTATION_MAX_PAIRS` (default `250`).

## API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/liquidity/fragmentation` | Network summary: distribution of coverage classes, worst offenders. |
| GET | `/api/liquidity/fragmentation/pairs?sort=gap&limit=50` | Ranked pairs. `sort`: `gap`\|`fragmentation`\|`depth`\|`routing_gain`. |
| GET | `/api/liquidity/fragmentation/pairs/:pair_key` | Full breakdown: venues, depth curve, metrics, history. |
| GET | `/api/liquidity/fragmentation/pairs/:pair_key/depth?bands=10,25,50,100,200` | Per-venue and consolidated depth. |
| GET | `/api/liquidity/gaps?limit=25` | Pairs classed `thin` or `missing`, ranked by gap score. |
| GET | `/api/liquidity/recommendations?account=G...&limit=20` | Ranked LP recommendations; `account` optional. |
| GET | `/api/liquidity/fragmentation/assets` | Per-asset rollup: total depth, venue spread, pairs served. |

### Response — `GET /api/liquidity/fragmentation/pairs/:pair_key`

```json
{
  "pair_key": "USDC:GA5Z.../XLM:native",
  "coverage_class": "fragmented",
  "metrics": {
    "venue_count": 6,
    "effective_venues": 3.4,
    "hhi": 2941,
    "fragmentation_index": 0.71,
    "consolidated_depth_usd": 1840200.0,
    "best_venue_depth_usd": 902400.0,
    "routing_gain_bps": 34.2,
    "reference_trade_usd": 25000.0,
    "demand_score": 0.62,
    "gap_score": 0.18
  },
  "venues": [
    { "kind": "amm_pool",  "venue_id": "abc123...", "share": 0.49, "depth_50bps_usd": 902400.0, "fee_bps": 30 },
    { "kind": "orderbook", "venue_id": "orderbook", "share": 0.31, "depth_50bps_usd": 571000.0, "fee_bps": 0 },
    { "kind": "amm_pool",  "venue_id": "def456...", "share": 0.13, "depth_50bps_usd": 239100.0, "fee_bps": 30 }
  ],
  "recommendations": [
    {
      "kind": "consolidation_candidate",
      "priority": 0.78,
      "rationale": {
        "effective_venues": 3.4,
        "routing_gain_bps": 34.2,
        "sub_scale_pools": 3,
        "note": "Three pools below $250k depth; splitting costs traders ~34 bps at the reference size."
      }
    }
  ],
  "computed_at": "2026-08-27T11:00:00Z"
}
```

### Errors

- **404** — unknown or untracked `pair_key`; response lists how to request tracking.
- **422** — malformed asset in `pair_key` (expected `CODE:ISSUER` or `XLM:native`).
- **503** — price feed unavailable, so USD normalization cannot be performed.

## UI Structure

```
┌──────────────────────────────────────────────────────────────┐
│ Liquidity Fragmentation                     250 pairs traced │
├──────────────────────────────────────────────────────────────┤
│ Concentrated  Healthy   Fragmented   Thin    Missing         │
│      38         104         61        39        8            │
│  ▓▓▓▓▓▓      ▓▓▓▓▓▓▓▓▓▓▓  ▓▓▓▓▓▓▓   ▓▓▓▓▓     ▓              │
├──────────────────────────────────────────────────────────────┤
│ USDC / XLM                                     Fragmented    │
│                                                              │
│  Consolidated depth  $1.84M     Effective venues     3.4     │
│  Best single venue   $902K      Routing gain      34 bps     │
│                                                              │
│  Where the liquidity sits (50 bps)                           │
│  Pool abc123…  ████████████████████████  49%        $902K    │
│  Orderbook     ███████████████           31%        $571K    │
│  Pool def456…  ██████                    13%        $239K    │
│  3 others      ███                        7%        $128K    │
├──────────────────────────────────────────────────────────────┤
│ Depth curve                       [consolidated] [per venue] │
│                                                              │
│ $2M┤                              ╭──── consolidated         │
│    │                      ╭───────╯                          │
│ $1M┤            ╭─────────╯   ╭──────── best venue           │
│    │     ╭──────╯      ╭──────╯                              │
│  $0└─────┴──────┴──────┴──────┴──────                        │
│      10     25     50    100    200  bps                     │
│                                                              │
│  At a $25,000 trade, splitting across venues returns 34 bps  │
│  more than routing to the deepest venue alone.               │
├──────────────────────────────────────────────────────────────┤
│ Liquidity gaps                                               │
│                                                              │
│ Pair              Demand   Depth 100bps   Gap    Class       │
│ NGNT / USDC        high        $12.4K     8.9    thin        │
│ ARST / USDC        med          $0        —      missing     │
│ BRL  / XLM         high        $48.2K     3.1    thin        │
├──────────────────────────────────────────────────────────────┤
│ For liquidity providers                                      │
│                                                              │
│ ▸ Seed NGNT/USDC — 214 failed path payments in 30d, no       │
│   direct venue above $15K depth.                             │
│ ▸ Consolidate USDC/XLM — 3 pools under $250K; traders lose   │
│   ~34 bps to the split.                                      │
│ ▸ Pool ghi789… yields above median — volume/TVL 4.2× for 14d.│
└──────────────────────────────────────────────────────────────┘
```

- Every recommendation states its evidence inline; the UI never shows a score without the inputs that produced it.
- Depth curves plot consolidated against best-single-venue so the fragmentation cost is visible as the gap between two lines.
- Class labels are text, not colour-only.

## Acceptance Criteria

- [ ] Migration `025_create_liquidity_fragmentation.sql` applies cleanly and is idempotent
- [ ] `fetch_order_book_full()` and `fetch_pools_for_pair()` added to `RpcClient` with tests
- [ ] Orderbook depth computed by walking levels within each bps band
- [ ] AMM depth computed from the constant-product curve including pool fee, verified against a worked example in tests
- [ ] All depth normalized to USD via `price_feed`; a missing price yields 503, never a silently wrong number
- [ ] HHI, `effective_venues`, and `fragmentation_index` computed and unit-tested against known distributions
- [ ] Routing gain computed from an optimal split; split solver unit-tested against a hand-checked two-venue case
- [ ] Path liquidity reported but excluded from the fragmentation index
- [ ] Coverage class assigned per the documented rules
- [ ] Gap score incorporates failed path-payment attempts
- [ ] Tracked-pair selection respects `FRAGMENTATION_MAX_PAIRS` and Horizon rate limits
- [ ] Snapshot job runs on the scheduler; raw snapshots pruned past retention with daily aggregates kept
- [ ] All five recommendation kinds implemented, each with structured `rationale`
- [ ] `account` scoping filters recommendations against existing LP positions
- [ ] All seven endpoints implemented with documented error cases
- [ ] `pair_key` canonicalization prevents duplicate `A/B` and `B/A` rows
- [ ] Scorecard, venue breakdown, depth curve, gap table, and recommendations shipped
- [ ] Existing `LiquidityHeatmap` extended with a fragmentation dimension
- [ ] Backend and frontend tests added; `docs/LIQUIDITY_FRAGMENTATION.md` written
- [ ] OpenAPI spec updated

## Implementation Steps

1. **RPC layer** — `fetch_order_book_full()` with a deeper level limit; `fetch_pools_for_pair()` filtering by reserve assets.
2. **Depth maths** — pure functions for orderbook walking and constant-product depth, unit-tested independently of any I/O. This is the part most likely to be subtly wrong; test it first.
3. **Venue inventory** — assemble all venues for a pair, normalize to USD, persist `pair_venue_snapshots`.
4. **Fragmentation metrics** — HHI, effective venues, index, optimal-split solver, routing gain; persist `pair_fragmentation`.
5. **Demand + gaps** — join 30-day payment/trade volume and failed path attempts; compute gap scores and coverage classes.
6. **Pair selection + scheduling** — build the tracked set, register `fragmentation_snapshot` in `backend/src/jobs/scheduler.rs`, respect rate limits.
7. **Recommendations** — `liquidity_recommendations.rs`, one rule per kind, each emitting structured rationale.
8. **API** — `backend/src/api/fragmentation.rs`; register in `mod.rs`; cache headers via `http_cache` (5 min TTL).
9. **Frontend service + components** — scorecard, venue breakdown, depth curve, gap table, recommendations.
10. **Page integration** — extend the liquidity and liquidity-pools pages; add the fragmentation dimension to the heatmap.
11. **Testing** — depth maths, HHI, split solver, classification, API integration, component tests.

## Considerations

- **Depth is not TVL.** A $10M pool at a bad reserve ratio can be shallower at 50 bps than a $2M balanced one. Every headline figure is depth-at-a-band, never raw TVL.
- **The constant-product depth formula must account for the pool fee** or every AMM venue reads deeper than it is. Verify against a worked example in tests.
- **USD normalization is a dependency, not a detail.** If `price_feed` cannot price an asset, that pair is reported as unpriceable rather than compared on native units.
- **Fragmentation is not automatically bad.** Competing venues can tighten spreads. The metric that matters for users is `routing_gain_bps` — the cost actually paid — so lead with it and treat the index as context.
- **Path liquidity is a fallback, not a venue.** Including it in the index would make the worst-served pairs look best served.
- **Recommendations are financial guidance about someone else's capital.** Show the evidence, state the observation window, and never imply a return.
- **Horizon rate limits bound the design.** The tracked-pair cap and snapshot interval are the levers; raising either must be a deliberate, measured change.

## References

- [Stellar Liquidity Pools](https://developers.stellar.org/docs/learn/fundamentals/stellar-data-structures/liquidity-pools)
- [Horizon `/liquidity_pools`](https://developers.stellar.org/docs/data/apis/horizon/api-reference/resources/liquidity-pools)
- [Horizon `/order_book`](https://developers.stellar.org/docs/data/apis/horizon/api-reference/aggregations/order-books)
- [Horizon path finding](https://developers.stellar.org/docs/data/apis/horizon/api-reference/aggregations/paths)
- [Herfindahl-Hirschman Index](https://www.justice.gov/atr/herfindahl-hirschman-index)
- Internal: `docs/RPC_DATA_SOURCES.md`, `backend/src/services/liquidity_pool_analyzer.rs`

## Related Issues

- Related to: #2105 Market Maker Performance Metrics (orderbook depth attribution is shared groundwork)
- Related to: Issue #027 Asset Velocity Metrics and Analysis
- Extends: existing liquidity pool analytics in `backend/src/services/liquidity_pool_analyzer.rs`
- Feeds: corridor health scoring in `backend/src/services/analytics.rs`

## Estimated Effort

- Depth maths + RPC layer: 1.5 days
- Venue inventory, metrics, split solver: 1.5 days
- Gap detection + recommendations engine: 1 day
- API + scheduling + persistence: 1 day
- Frontend components and integration: 1.5 days
- Testing, docs, polish: 0.5 days
- **Total: 6 days**
