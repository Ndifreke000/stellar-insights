# Property-Based Tests for Aggregation Math

> Tracking issue: [#2850](https://github.com/Ndifreke000/stellar-insights/issues/2850)
> Status: **Design / proposed.**

## Why

Aggregation results feed the snapshots that are hashed and anchored on-chain through `submit_snapshot`. A bug there is permanent once it is anchored. Today the aggregation tests use a handful of hand-written fixtures, which check examples, not invariants.

`proptest` is already a backend dev-dependency (`backend/Cargo.toml`, `proptest = "=1.11.0"`), so no new dependency is needed.

## Code under test

| Function | File | What it does |
| --- | --- | --- |
| `AggregationService::group_by_hour_bucket` | `backend/src/services/aggregation.rs` | Groups `CorridorMetrics` by `(corridor_key, hour)` and merges them |
| `merge_hourly_metric` | same | Sums the counts and volume, combines latency and liquidity |
| `merge_latency` | same | Transaction-weighted mean of the average latency |
| `recalculate_success_rate` | same | `successful / total * 100` |
| `truncate_to_hour` | same | Floors a timestamp to the hour |
| `compute_volume_trends` | same | Per-corridor total, mean and first-half/second-half trend % |
| `compute_anchor_metrics` | `backend/src/analytics.rs` | Success and failure rate, reliability score, status |
| `calculate_settlement_time_score` | same | Linear 1 s → 10 s mapped to 100 → 0 |
| `compute_anchor_reliability_score` | same | 60/30/10 composite of the volume-weighted success rate, log volume and diversity |

> Note: success rates are on a **0–100** scale in this code, not [0, 1] as the issue text says. Neither module computes percentiles yet. The percentile monotonicity invariant applies when latency percentiles are added, and is listed below so that work picks it up.

## Invariants

### Hourly bucketing (`group_by_hour_bucket` and the merge helpers)

| # | Invariant |
| --- | --- |
| A1 | **Conservation:** for each corridor, the sums of `total_transactions`, `successful_transactions`, `failed_transactions` and `volume_usd` across output buckets equal the sums across the input |
| A2 | **Order independence:** shuffling the input produces the same multiset of buckets, ignoring `id` |
| A3 | **Partition equivalence (replay vs. incremental):** splitting the input at any point, aggregating each part and merging gives the same result as aggregating everything at once |
| A4 | **Bucket alignment:** every `hour_bucket` has minute, second and nanosecond all zero, and `hour_bucket <= input.date < hour_bucket + 1h` for each contributing input |
| A5 | **Uniqueness:** at most one output per `(corridor_key, hour_bucket)` |
| A6 | **Success rate bounds:** `0 <= success_rate <= 100`, and it equals `successful / total * 100` whenever `total > 0` |
| A7 | **Counts consistent:** `successful + failed <= total` in the output, given the same holds for every input |
| A8 | **Latency bounds:** the merged `avg_settlement_latency_ms` is between the min and max of the contributing non-`None` latencies |
| A9 | **Latency weighting:** the merged latency equals `Σ(lat_i · n_i) / Σ n_i` over the inputs that have a latency, within integer truncation |
| A10 | **Hourly → daily:** summing the 24 hourly buckets for a day equals aggregating the same day's input in one daily bucket. Run this at every granularity we expose (hour, day; and week and month if rollups use the same merge) |

### Volume trends (`compute_volume_trends`)

| # | Invariant |
| --- | --- |
| T1 | `total_volume == Σ volume_usd` for the corridor, and `data_points == number of hourly rows` |
| T2 | `avg_volume * data_points == total_volume` (within ε) |
| T3 | Constant volume gives `trend_percentage == 0` |
| T4 | Scaling every volume by k > 0 leaves `trend_percentage` unchanged |
| T5 | The result does not depend on input order (the function sorts by time) |

### Anchor metrics (`analytics.rs`)

| # | Invariant |
| --- | --- |
| M1 | `success_rate` and `failure_rate` are in [0, 100]. If `successful + failed == total`, they sum to 100 (within the 0.01 rounding) |
| M2 | `reliability_score` is in [0, 100] |
| M3 | `calculate_settlement_time_score` is in [0, 100] and **monotonically non-increasing** in latency |
| M4 | `reliability_score` is monotonically non-decreasing in `successful_transactions` when the total is held fixed |
| M5 | `compute_anchor_reliability_score`: every sub-score is in [0, 100], `composite == 0.6·perf + 0.3·vol + 0.1·div` |
| M6 | `weighted_success_rate` lies between the min and max per-asset success rates (among assets with volume > 0) |
| M7 | Permuting `asset_performances` does not change any score |
| M8 | `volume_score` is monotonically non-decreasing in `total_volume_usd` and equals 100 when `total == network_max` |

### Percentiles (future)

| # | Invariant |
| --- | --- |
| P1 | `p50 <= p90 <= p95 <= p99 <= max` for every bucket |
| P2 | `min <= p50` |

## Known issues these properties will catch

Reading the current code shows at least two invariants that will **fail**. That is expected; these are the first regression tests to land.

1. **Liquidity-depth merge depends on order (breaks A2 and A3).** `merge_hourly_metric` sets `liquidity_depth_usd = (existing + new) / 2`. With three inputs a, b and c that gives `((a+b)/2 + c)/2`, which weights c twice as much as a and b and depends on `HashMap` iteration order. Decide on the intended semantics (time-weighted mean, last value, or max) and fix the code.
2. **`None` latency treated as 0 (breaks A8 and A9).** If the first metric in a bucket has `avg_settlement_latency_ms = None` and `total_transactions > 0`, `merge_latency` treats its latency as 0 but still counts its transactions as weight, which pulls the average down. Only transactions that have a latency should contribute weight.

Also check for these:

- `merge_latency` accumulates in `i64` and truncates when it casts back to `i32`. A3 must allow ±1 ms per merge step, or the merge should keep the sum and count and divide once at the end.
- `f64` summation of `volume_usd` is not associative. Compare with a relative ε (for example `1e-9 · max(|a|, |b|, 1)`) rather than with `==`.

## Generators

Put shared strategies in `backend/tests/common/strategies.rs`:

```rust
use proptest::prelude::*;
use chrono::{DateTime, Duration, TimeZone, Utc};

pub fn corridor_key() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "USDC:GA5Z->EURC:GDHU", "XLM:native->USDC:GA5Z", "NGNT:GAWO->USDC:GA5Z",
    ]).prop_map(str::to_owned)
}

pub fn timestamp_in_window() -> impl Strategy<Value = DateTime<Utc>> {
    // 7 days at second resolution; keeps bucket collisions frequent
    (0i64..7 * 24 * 3600).prop_map(|s| Utc.timestamp_opt(1_700_000_000 + s, 0).unwrap())
}

pub fn corridor_metric() -> impl Strategy<Value = CorridorMetrics> {
    (corridor_key(), timestamp_in_window(), 0i64..10_000, 0.0f64..1e9,
     prop::option::of(1i32..60_000), 0.0f64..1e9)
        .prop_flat_map(|(key, date, total, vol, lat, liq)| {
            (Just((key, date, total, vol, lat, liq)), 0..=total)
        })
        .prop_flat_map(|((key, date, total, vol, lat, liq), ok)| {
            (Just((key, date, total, vol, lat, liq, ok)), 0..=(total - ok))
        })
        .prop_map(|((key, date, total, vol, lat, liq, ok), failed)| {
            // build a CorridorMetrics from these values; success_rate = ok / total * 100
            make_corridor_metric(key, date, total, ok, failed, vol, lat, liq)
        })
}

pub fn payment_stream() -> impl Strategy<Value = Vec<CorridorMetrics>> {
    prop::collection::vec(corridor_metric(), 0..500)
}
```

Edge cases the generators must hit often (use `prop_oneof!` with weights):

- `total_transactions == 0`
- latency `None`
- timestamps exactly on `HH:00:00.000` and at `HH:59:59.999999999`
- a single corridor with many rows in one bucket
- volume `0.0` and very large values (`1e15`)

## Test layout

```
backend/tests/
  common/strategies.rs
  aggregation_properties.rs      # A1–A10, T1–T5
  analytics_properties.rs        # M1–M8
backend/proptest-regressions/    # committed; see below
```

`group_by_hour_bucket`, `merge_*` and `compute_volume_trends` are private methods. Either make them `pub(crate)` and test them from a `#[cfg(test)] mod properties` in `aggregation.rs`, or extract the pure math into `services/aggregation/math.rs` with `pub` functions. Extraction is preferred, because it also removes the `&self` that `truncate_to_hour` and `compute_volume_trends` do not use.

Example:

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: std::env::var("PROPTEST_CASES").ok()
            .and_then(|v| v.parse().ok()).unwrap_or(256),
        ..ProptestConfig::default()
    })]

    #[test]
    fn a3_replay_equals_incremental(stream in payment_stream(), split in any::<prop::sample::Index>()) {
        let at = split.index(stream.len() + 1);
        let (left, right) = stream.split_at(at);
        let full  = normalize(group_by_hour_bucket(stream.clone()));
        let incr  = normalize(merge_bucket_sets(
            group_by_hour_bucket(left.to_vec()),
            group_by_hour_bucket(right.to_vec()),
        ));
        prop_assert_eq_approx!(full, incr);
    }
}
```

`normalize` sorts by `(corridor_key, hour_bucket)` and clears `id`.

## Turning shrunk failures into regression tests

- Commit `backend/proptest-regressions/`. proptest writes the seed of every failing case there and replays it first on later runs.
- After a fix, also add the **shrunk** counterexample as a plain `#[test]` with a descriptive name (for example `liquidity_merge_is_order_independent_three_inputs`), so the case stays documented even if the generators change.

## CI

- PRs: 256 cases per property (the default, kept fast).
- Nightly and the main branch: `PROPTEST_CASES=10000 cargo nextest run --test aggregation_properties --test analytics_properties`.

Once the consolidated pipeline lands, this becomes a step in `_rust.yml` (see [CI_PIPELINE.md](CI_PIPELINE.md)).

## Acceptance criteria

- [ ] Invariant tests for every aggregation function listed above
- [ ] Invariants hold for 10k generated cases in CI
- [ ] The two known issues above are fixed, with regression tests
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue
