# [Backend + Frontend] Asset Velocity Metrics and Analysis

**Issue:** #2101
**Priority:** Medium
**Type:** Feature
**Component:** Backend + Frontend
**Labels:** `enhancement`, `analytics`, `assets`
**Estimated effort:** 5 days

## 🎯 Problem Statement

There is no concept of asset "velocity" anywhere in the codebase today — this is new ground, not an enhancement of an existing metric (unlike #2099/#2100).

What exists today, for reference:
- [migrations/026_create_verified_assets.sql](backend/migrations/026_create_verified_assets.sql) — `verified_assets` table has `trustline_count`, `transaction_count`, `total_volume_usd`, but these are point-in-time totals, not rates or turnover.
- [migrations/003_create_ingestion_and_payments.sql](backend/migrations/003_create_ingestion_and_payments.sql) — `payments` table (per-payment rows with `asset_code`, `asset_issuer`, `amount`, `created_at`) and `corridor_metrics` (daily aggregates per asset pair) are the only raw material available to compute velocity from.
- [components/dashboard/TopAssetsTable.tsx](frontend/src/components/dashboard/TopAssetsTable.tsx) and [components/dashboard/TopAssetsCard.tsx](frontend/src/components/dashboard/TopAssetsCard.tsx) — frontend only renders `symbol`, `name`, `volume24h`, `price`, `change24h`. No turnover/holder-churn fields exist in these types.
- [components/anchors/AssetPortfolio.tsx](frontend/src/components/anchors/AssetPortfolio.tsx), [AssetDistributionChart.tsx](frontend/src/components/anchors/AssetDistributionChart.tsx) — per-anchor asset breakdowns, again static snapshots.

Without velocity/turnover metrics, users can't tell whether an asset's volume is a few wallets churning the same balance repeatedly vs. broad, healthy circulation — which matters for assessing whether an asset (or anchor-issued token) is actually liquid or just noisy.

## 💡 Solution

Add a backend analytics module that computes, per asset (`asset_code` + `asset_issuer`):

1. **Transaction frequency** — payments per unit time (e.g., tx/hour, tx/day), from `payments` grouped by `asset_code`/`asset_issuer`/time bucket.
2. **Holder turnover** — the fraction of distinct holders (accounts with a trustline, or accounts that appear as source/destination in `payments`) that transacted in a given window vs. total holders.
3. **Circulation velocity** — a classic velocity-of-money style ratio: `volume moved in period / average balance (or total supply) in circulation over that period`.

Expose these through a new API endpoint and surface them in the frontend asset tables/detail views alongside the existing volume/price fields.

## 📁 Files to Create/Modify

### New Files (Backend)
```
backend/src/analytics/asset_velocity.rs      # frequency, turnover, circulation-velocity calculations
backend/src/api/asset_velocity.rs            # GET /api/assets/:code/:issuer/velocity (and a list/ranking endpoint)
backend/migrations/0XX_asset_velocity_history.sql   # time-bucketed velocity snapshots for trend charts
```

### Modified Files (Backend)
```
backend/src/analytics.rs                     # register `asset_velocity` submodule (mirrors `pub mod corridor;`)
backend/src/api/mod.rs                        # route registration
backend/src/db/mod.rs or db/assets.rs         # queries: distinct holders per window, volume per window
```

### New/Modified Files (Frontend)
```
frontend/src/lib/analytics-api.ts             # fetchAssetVelocity() — follow the existing fetchAnalyticsMetrics() pattern (line 70)
frontend/src/components/dashboard/TopAssetsTable.tsx  # add velocity/turnover column
frontend/src/components/anchors/AssetPortfolio.tsx     # surface per-asset velocity in anchor asset breakdown
frontend/src/components/anchors/AssetDetailModal.tsx   # velocity detail/trend chart
```

## 🔧 Technical Implementation

### Data sources

- `payments` table ([migrations/003_create_ingestion_and_payments.sql](backend/migrations/003_create_ingestion_and_payments.sql#L1)) is per-transaction, so it's the source for frequency and the numerator of turnover/velocity — group by `asset_code`, `asset_issuer`, and a time bucket (hour/day).
- Distinct holder counts need `source_account`/`destination_account` distinct counts per asset per window — there's no existing "holders" table; `verified_assets.trustline_count` ([migrations/026](backend/migrations/026_create_verified_assets.sql#L15)) is the closest existing proxy for total holder count but is not time-windowed.
- `corridor_metrics` ([migrations/003](backend/migrations/003_create_ingestion_and_payments.sql#L21)) already aggregates daily `volume_usd`/`success_rate` per asset pair and could seed the circulation-velocity denominator if a full balance snapshot isn't available.

### Suggested formulas

```text
transaction_frequency(asset, window) = COUNT(payments WHERE asset AND created_at IN window) / window_duration

holder_turnover(asset, window) = DISTINCT(accounts in payments WHERE asset AND window) / total_holders(asset)

circulation_velocity(asset, window) = SUM(amount WHERE asset AND window) / avg_balance_in_circulation(asset, window)
```

`avg_balance_in_circulation` will likely need to come from Horizon/RPC balance snapshots rather than derived purely from local tables — flag this as a design decision before implementation (see Dependencies below).

## ✅ Acceptance Criteria

- [ ] New endpoint returns transaction frequency, holder turnover, and circulation velocity for a given asset over a configurable window (e.g., 1h/24h/7d)
- [ ] A ranking/list endpoint surfaces top assets by velocity (for a "trending" style view)
- [ ] Frontend `TopAssetsTable`/`TopAssetsCard` show a velocity or turnover indicator, not just `volume24h`
- [ ] `AssetDetailModal` shows a velocity trend over time (requires the history table)
- [ ] Metrics degrade gracefully for low-activity assets (avoid divide-by-zero on `total_holders` or `avg_balance`)
- [ ] Documented definition of each metric (frequency vs. turnover vs. velocity are easy to conflate — the API/docs should be explicit)

## 🧪 Testing Strategy

- Unit tests for each formula against fixed `payments`/`corridor_metrics` fixtures, including zero-activity and single-holder edge cases
- Integration test for the new API endpoint(s)
- Frontend component test confirming the new column/field renders and formats correctly (e.g., "2.3x/day")

## 🔗 Dependencies / Open Questions

- **Balance data source**: circulation velocity needs a total-supply or average-balance figure per asset. Confirm whether this should come from live Horizon/RPC queries (see [rpc/stellar.rs](backend/src/rpc/stellar.rs)) or a periodic snapshot job, since it's not derivable from `payments` alone.
- Loosely related to [#2099](2099-anchor-reliability-scoring-ml.md)/[#2100](2100-payment-success-prediction-model.md) in that all three want richer historical/behavioral features — but this is net-new instrumentation, not an upgrade of an existing score.

## ⏱️ Estimated Effort

**Total: 5 days** (per issue tracker estimate)
