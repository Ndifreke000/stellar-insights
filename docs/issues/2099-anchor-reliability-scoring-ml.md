# [Backend] Anchor Reliability Scoring Algorithm Enhancement

**Issue:** #2099
**Priority:** High
**Type:** Feature
**Component:** Backend
**Labels:** `enhancement`, `analytics`, `anchors`
**Estimated effort:** 8 days

## 🎯 Problem Statement

There are currently **three different, disconnected notions of anchor reliability** in the backend, and none of them are learned:

1. [db/anchors.rs](backend/src/db/anchors.rs#L423) `get_recent_anchor_performance` — sets `reliability_score: success_rate` verbatim (a straight copy, no weighting at all). This is what [services/anchor_monitor.rs](backend/src/services/anchor_monitor.rs) polls every 5 minutes for alerting, comparing only against the *previous* poll (>10% success-rate drop, >50% latency increase — [anchor_monitor.rs:158-186](backend/src/services/anchor_monitor.rs#L158-L186)).
2. [analytics.rs](backend/src/analytics.rs#L48) `compute_anchor_metrics` — a fixed-weight heuristic, `reliability_score = 0.7 * success_rate + 0.3 * settlement_time_score`. This is the one actually wired into the write path, called from [handlers.rs:192](backend/src/handlers.rs#L192) and [api/anchors.rs:201](backend/src/api/anchors.rs#L201) before `update_anchor_metrics` persists the score.
3. [analytics.rs](backend/src/analytics.rs#L154) `compute_anchor_reliability_score` — a more elaborate composite (`0.6 * asset_performance_score + 0.3 * volume_score + 0.1 * asset_diversity_score`, with log-scaled volume and success weighted by per-asset volume). This one is fully built and unit-tested but **not called from any handler or API route** — it's dead code outside its own test module.

All three are hand-picked fixed weights, not learned from data, and none of them use transaction *patterns* (time-of-day, streaks, volatility) or network conditions (congestion, ledger close times) — only the current snapshot's success/failure counts and settlement time.

This blocks any "predict which anchor to route through" feature, since every existing score is purely reactive to the last snapshot and can't warn users an anchor is degrading before it actually fails a payment.

## 💡 Solution

Pick one of the three existing scoring paths as the real one (recommend consolidating on `compute_anchor_reliability_score` in `analytics.rs` since it's the most complete and already unit-tested — then wire it into `handlers.rs`/`api/anchors.rs` in place of `compute_anchor_metrics`, and make `get_recent_anchor_performance` in `db/anchors.rs` call the same code path instead of assigning `success_rate` directly). Then replace its fixed weights with a learned model that predicts reliability from historical performance data, transaction patterns, and network conditions, keeping the existing Green/Yellow/Red status thresholds ([models.rs](backend/src/models.rs#L127) `AnchorStatus::from_metrics`) as a fallback/sanity check during rollout.

Reuse the pattern already established for payment prediction in [ml.rs](backend/src/ml.rs) (`SimpleMLModel`, `PredictionFeatures`/`PredictionResult`) rather than inventing a second ML abstraction — either extend that module or add a sibling `anchor_ml.rs` with the same shape.

## 📁 Files to Create/Modify

### New Files
```
backend/src/ml/anchor_scoring.rs      # feature extraction + model for anchor reliability
backend/migrations/0XX_anchor_score_history.sql   # persisted score + feature snapshots for training/backtesting
```

### Modified Files
```
backend/src/analytics.rs              # compute_anchor_metrics / compute_anchor_reliability_score — replace fixed weights with model, or wire the unused composite fn in and model that
backend/src/db/anchors.rs             # get_recent_anchor_performance — stop hardcoding reliability_score = success_rate, call the consolidated scorer instead
backend/src/services/anchor_monitor.rs # feed model instead of/alongside the raw threshold diff
backend/src/handlers.rs               # update call site if compute_anchor_metrics signature changes
backend/src/models.rs                 # AnchorMetrics — add score_confidence / trend fields if needed
backend/src/api/anchors.rs            # expose model version / confidence in anchor metrics responses
backend/src/ml.rs                     # optionally shared feature-engineering helpers (hour_of_day, day_of_week already exist here)
```

## 🔧 Technical Implementation

### Candidate features (from data already ingested)

- Rolling success rate over multiple windows (1h / 24h / 7d) — currently only a single 60-minute window is queried ([db/anchors.rs](backend/src/db/anchors.rs#L377))
- Settlement-time trend (`avg_settlement_time_ms` history is already stored in `anchor_metrics_history`, see [models.rs](backend/src/models.rs#L82))
- Transaction volume and volume trend (`corridor_metrics.volume_usd`, [migrations/003_create_ingestion_and_payments.sql](backend/migrations/003_create_ingestion_and_payments.sql))
- Time-of-day / day-of-week seasonality (mirror the encoding already used in `PredictionFeatures` in [ml.rs](backend/src/ml.rs#L6))
- Consecutive-failure streaks / volatility of success rate (currently unused signal — `AnchorMonitor` only looks at `last_metrics` from one prior poll)

### Model

Start with the same "simple linear model + sigmoid" approach as `SimpleMLModel` in [ml.rs](backend/src/ml.rs#L22) for consistency and easy offline retraining, with a `model_version` field carried through to the API response (already the pattern in [api/ml.rs](backend/src/api/ml.rs#L19) `PredictionResponse`). Leave room to swap in a heavier model later without changing the call sites.

## ✅ Acceptance Criteria

- [ ] There is exactly one reliability-scoring code path; `db/anchors.rs`, `analytics.rs`'s two functions, and any handler are consolidated instead of three independent formulas
- [ ] `reliability_score` is produced by the model, not a fixed-weight heuristic or a copy of `success_rate`
- [ ] Model consumes multi-window historical data, not just the single 60-minute snapshot currently queried in `get_recent_anchor_performance`
- [ ] `AnchorMetrics` (or the API response) surfaces a confidence value and model version, matching the pattern in `PredictionResult`
- [ ] Existing Green/Yellow/Red thresholds still work as a sanity bound on the model's output (no silent contradiction between score and status)
- [ ] `AnchorMonitor::check_anchors` alerting keeps working — degradation alerts should fire off the model's trend, not only the single-poll diff
- [ ] Backtest against historical `anchor_metrics_history` rows shows the model score correlates with actual subsequent failures better than the raw success rate baseline
- [ ] Retraining path documented/scripted (mirror `MLService::retrain_weekly` in [ml.rs](backend/src/ml.rs#L174))
- [ ] Existing tests in `analytics.rs`'s `#[cfg(test)] mod tests` (16 tests covering both scoring functions) are updated or superseded, not left testing dead code

## 🧪 Testing Strategy

- Unit tests for feature extraction against fixed `anchor_metrics_history` fixtures
- Backtest harness comparing model score vs. baseline (`success_rate`) against actual next-window failures
- Integration test: `GET` anchor endpoint in [api/anchors.rs](backend/src/api/anchors.rs) returns the new fields
- Regression test that `AnchorStatus::from_metrics` thresholds are unaffected

## 🔗 Dependencies

**Related:** [#2100 Payment Success Prediction Model Enhancement](2100-payment-success-prediction-model.md) — shares the feature-engineering and ML-versioning pattern in `ml.rs`; anchor status is one of the features that issue wants to add to payment prediction, so sequencing matters (this issue should land the richer anchor signal first, or the two should agree on a shared feature set).

## ⏱️ Estimated Effort

**Total: 8 days** (per issue tracker estimate)
