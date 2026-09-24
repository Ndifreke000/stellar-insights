# Gas Costs

Gas cost benchmarks for PayRaider Soroban contracts, measured with [Criterion](https://bheisler.github.io/criterion.rs/book/).

## Running benchmarks

```bash
# All contracts
cargo bench --package contract-benches

# Single benchmark
cargo bench --package contract-benches -- analytics::submit_snapshot

# Generate HTML report (opens in browser)
cargo bench --package contract-benches
open contracts/target/criterion/report/index.html
```

## Benchmark descriptions

### Analytics contract

| Benchmark | What it measures |
|---|---|
| `analytics::submit_snapshot` | Single snapshot write (epoch + hash + submitter) |
| `analytics::get_snapshot` | Point read from a 100-snapshot store |
| `analytics::get_latest_snapshot` | Read the latest-epoch pointer |
| `analytics::batch_submit/N` | Sequential write of N snapshots (N = 5, 10, 25, 50) |
| `analytics::history_read/N` | Full history read with N snapshots stored |

### Governance contract

| Benchmark | What it measures |
|---|---|
| `governance::create_proposal` | Proposal creation (title + wasm hash + tally init) |
| `governance::vote` | Single vote cast on a fresh proposal |
| `governance::multi_vote/N` | N votes on one proposal (N = 5, 10, 25, 50) |
| `governance::get_proposal` | Point read of a stored proposal |

### PayRaider contract

| Benchmark | What it measures |
|---|---|
| `payraider::submit_snapshot` | Snapshot write |
| `payraider::get_snapshot` | Point read from a 100-snapshot store |
| `payraider::latest_snapshot` | Read the latest-epoch pointer |

## Baseline

The table below is the authoritative baseline used by CI to detect regressions.
It is updated automatically on every push to `main` by the [gas-regression workflow](../.github/workflows/gas-regression.yml).

<!-- GAS_BASELINE_START -->
| Benchmark | Median time |
|---|---|
<!-- GAS_BASELINE_END -->

## Regression policy

CI fails a pull request if any benchmark regresses by more than **20%** relative to the baseline.

To update the baseline intentionally (e.g. after adding a feature that increases cost):

```bash
# Run locally and update docs/GAS_COSTS.md
cargo bench --package contract-benches 2>&1 | tee /tmp/bench.txt
python3 scripts/check_gas_regression.py /tmp/bench.txt docs/GAS_COSTS.md --update-baseline
```

## Optimization guide

### Storage access patterns

- Prefer `instance` storage for small, frequently-read values (admin, config). It is loaded once per invocation.
- Use `persistent` storage keyed by a specific ID (e.g. `DataKey::Proposal(id)`) rather than loading an entire map.
- Avoid unbounded `Map` or `Vec` reads — cost scales linearly with size.

### Reducing write cost

- Batch multiple writes in a single invocation where possible.
- Only write fields that changed; avoid re-serializing unchanged data.

### Event emission

- Events are cheap but not free. Avoid emitting large payloads in hot paths.
- Use `symbol_short!` for event topics — it avoids a heap allocation.

### TTL extensions

- `extend_ttl` calls add a small fixed cost. Group them at the end of a function.
- Use a high `threshold` value so TTL extensions are rare in practice.
