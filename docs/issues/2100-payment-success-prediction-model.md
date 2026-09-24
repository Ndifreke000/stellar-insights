# [Backend] Payment Success Prediction Model Enhancement

**Issue:** #2100
**Priority:** High
**Type:** Feature
**Component:** Backend
**Labels:** `enhancement`, `ml`, `prediction`
**Estimated effort:** 7 days

## 🎯 Problem Statement

There are currently **two separate, disconnected prediction endpoints**:

1. [api/prediction.rs](backend/src/api/prediction.rs) — `POST /api/predict/success`, a pure mock that returns `rng.gen_range(0.8..0.98)` with hardcoded `alternative_routes` strings. No real inputs are used at all.
2. [api/ml.rs](backend/src/api/ml.rs) → [ml.rs](backend/src/ml.rs) — a real (if simple) linear model, `SimpleMLModel::predict`, using 6 features: `corridor_hash`, `amount_usd` (log-scaled), `hour_of_day`, `day_of_week`, `liquidity_depth`, `recent_success_rate`.

The real model in `ml.rs` also has two mocked feature sources that were never wired to actual data:
- `get_corridor_liquidity` — returns `corridor.len() * 100.0 + 1000.0`, i.e. a function of the string length, not real liquidity ([ml.rs:164](backend/src/ml.rs#L164))
- `get_recent_success_rate` — returns `0.8 + (corridor.len() % 20) / 100.0`, also a function of string length ([ml.rs:169](backend/src/ml.rs#L169))

And the issue's requested features — time-of-day patterns, network congestion, historical corridor performance, and anchor status — are only partially present: `hour_of_day`/`day_of_week` exist but network congestion and anchor status are entirely missing from `PredictionFeatures`.

## 💡 Solution

1. Consolidate to a single real prediction path — deprecate or redirect `api/prediction.rs`'s mock endpoint to the `MLService`-backed one in `api/ml.rs`, so there's one source of truth for "predict payment success."
2. Wire `get_corridor_liquidity` and `get_recent_success_rate` to real queries (corridor liquidity from `corridor_metrics`, success rate from `payments`/`corridor_metrics.success_rate` — see [db/corridors.rs](backend/src/db/corridors.rs)) instead of string-length placeholders.
3. Extend `PredictionFeatures` with the four features named in the issue: time-of-day (already present), network congestion (new), historical corridor performance (upgrade from mocked to real), and anchor status (new — depends on [#2099](2099-anchor-reliability-scoring-ml.md)'s reliability score, or at minimum the existing `AnchorStatus` enum in [models.rs](backend/src/models.rs#L110)).

## 📁 Files to Create/Modify

### Modified Files
```
backend/src/ml.rs                # PredictionFeatures — add congestion + anchor_status fields; replace mocked getters with real DB queries
backend/src/api/ml.rs            # PredictionResponse — surface any new fields needed by the frontend
backend/src/api/prediction.rs    # remove or delegate to MLService instead of rand::gen_range mock
backend/src/db/corridors.rs      # add/reuse queries for corridor liquidity + historical success rate
backend/src/api/mod.rs           # route wiring if /api/predict/success is merged into the ml.rs handler
```

### Possibly New Files
```
backend/src/services/congestion.rs   # network congestion signal (e.g. recent ledger close times / tx throughput)
```

## 🔧 Technical Implementation

### Current model (baseline to preserve behavior of during migration)

```rust
// backend/src/ml.rs
pub struct PredictionFeatures {
    pub corridor_hash: f32,
    pub amount_usd: f32,
    pub hour_of_day: f32,
    pub day_of_week: f32,
    pub liquidity_depth: f32,
    pub recent_success_rate: f32,
}
```

`SimpleMLModel::predict` is a 6-weight linear model + sigmoid ([ml.rs:47-74](backend/src/ml.rs#L47-L74)); `weights` are currently hardcoded (`vec![0.1, 0.3, 0.05, 0.02, 0.4, 0.6]`) rather than learned — `train()` is a documented placeholder that doesn't actually update weights ([ml.rs:76-83](backend/src/ml.rs#L76-L83)). Retraining should either implement real gradient descent or call out to an offline training job that produces new weights.

### New features to add

- **Network congestion**: derive from recent ledger throughput / average settlement latency (there's precedent — `AnchorMetrics.avg_settlement_time_ms` already tracks this per-anchor in [models.rs](backend/src/models.rs#L92); a network-wide equivalent is new).
- **Anchor status**: fold in `AnchorStatus` (Green/Yellow/Red, [models.rs:110](backend/src/models.rs#L110)) or the enhanced score from #2099 for the anchors on either side of the corridor.

### Replace mocked data getters

```rust
// ml.rs — currently:
fn get_corridor_liquidity(&self, corridor: &str) -> Option<f64> {
    Some((corridor.len() as f64).mul_add(100.0, 1000.0)) // placeholder
}
fn get_recent_success_rate(&self, corridor: &str) -> Option<f32> {
    Some(0.8 + (corridor.len() as f32 * 0.01) % 0.2) // placeholder
}
```
Both need to become real `self.db` queries against `corridor_metrics` (see [migrations/003_create_ingestion_and_payments.sql](backend/migrations/003_create_ingestion_and_payments.sql#L21) — `volume_usd`, `success_rate` are already columns there). Note `MLService.db` is currently `#[allow(dead_code)]` with a comment "Reserved for future ML model training from database" ([ml.rs:88](backend/src/ml.rs#L88)) — this issue is exactly that future work.

## ✅ Acceptance Criteria

- [ ] Single prediction endpoint backed by real data — mock `rand`-based endpoint in `api/prediction.rs` is removed or delegates to `MLService`
- [ ] `get_corridor_liquidity` and `get_recent_success_rate` query real tables, not string length
- [ ] `PredictionFeatures` includes network congestion and anchor status signals
- [ ] Model weights are either genuinely trained (real gradient descent replacing the `train()` placeholder) or the retraining job is documented as external/offline
- [ ] `model_version` continues to be surfaced in the response so clients can detect model changes
- [ ] Existing `/api/predict/success` and `/api/ml/predict` callers (frontend) are updated to the consolidated endpoint

## 🧪 Testing Strategy

- Unit tests for the new congestion/anchor-status feature extraction
- Regression tests confirming corridor liquidity/success-rate queries match expected values from seeded `corridor_metrics` fixtures
- Backtest: compare predicted probability vs. actual payment outcomes on historical data
- Contract test on the API response shape after consolidating the two endpoints (frontend consumers should not break)

## 🔗 Dependencies

**Related:** [#2099 Anchor Reliability Scoring Algorithm Enhancement](2099-anchor-reliability-scoring-ml.md) — anchor status as a feature here should reuse whatever score #2099 produces rather than duplicating logic.

## ⏱️ Estimated Effort

**Total: 7 days** (per issue tracker estimate)
