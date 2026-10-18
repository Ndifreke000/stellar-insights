# Gas Costs

Gas costs for the `payraider` Soroban contract — the only contract in the
off-ramp product (everything else is in [`contracts/archive/`](../contracts/archive/README.md)).

CI gates on **host-budget CPU instructions and memory bytes** per entrypoint,
measured by [`contracts/benches/benches/gas_benchmarks.rs`](../contracts/benches/benches/gas_benchmarks.rs).
These are deterministic for a given contract build — the same figures every
run, on any machine — so a tight per-entrypoint threshold doesn't flake the way
wall-clock timings would.

## Running locally

```bash
cd contracts

# Per-entrypoint CPU / memory (what CI checks)
cargo bench -p contract-benches --bench gas_benchmarks | tee gas_output.txt

# Compare against the baseline below using the per-entrypoint thresholds
python3 ../scripts/check_gas_regression.py gas_output.txt ../docs/GAS_COSTS.md \
  --thresholds benches/gas-thresholds.toml

# Wall-clock timings (informational only, not gated in CI)
cargo bench -p contract-benches --bench contract_benchmarks
open target/criterion/report/index.html
```

`check_gas_regression.py` needs Python 3.11+ (for `tomllib`).

## What is measured

Each entrypoint is measured once, against a contract with one authorised
submitter and 50 snapshots already stored, so map-backed reads and writes are
costed at a realistic size.

| Entrypoint | What it measures |
|---|---|
| `payraider::initialize` | First-time setup of admin, version and pause flag |
| `payraider::submit_snapshot` | Anchoring snapshot 51 as an authorised submitter |
| `payraider::get_snapshot` | Point read from the 50-snapshot store |
| `payraider::latest_snapshot` | Read of the latest-epoch snapshot |
| `payraider::bump_ttl` | Permissionless TTL extension of instance, snapshots and admin role |
| `payraider::add_submitter` | Adding a key to the authorised submitter set |
| `payraider::rotate_submitter` | Atomic swap of one submitter key for another |
| `payraider::remove_submitter` | Removing a key from the submitter set |
| `payraider::grant_role` / `revoke_role` | Granting / revoking `PauseManager` |
| `payraider::pause` / `unpause` | Toggling the emergency pause |
| `payraider::set_admin` | Two-party admin transfer |

## Thresholds

Per-entrypoint thresholds live in
[`contracts/benches/gas-thresholds.toml`](../contracts/benches/gas-thresholds.toml).
Every entrypoint is currently held to **10%**:

```toml
[entrypoints]
"payraider::submit_snapshot" = 0.10
# ...
```

A threshold applies to CPU instructions **and** memory bytes — exceeding either
fails CI. The check also fails if an entrypoint is measured but has no
threshold, or has a threshold but is no longer measured, so adding an entrypoint
to the benchmark means adding its threshold in the same PR.

The current threshold for each entrypoint is also shown in the **Max regression**
column of the baseline below.

## Baseline

The table below is the authoritative baseline used by CI to detect regressions.
It is regenerated automatically on every push to `main` by the
[gas-regression workflow](../.github/workflows/gas-regression.yml). If no table is
present, the check is skipped until the first push to `main` writes one.

<!-- GAS_BASELINE_START -->
| Entrypoint | CPU instructions | Memory bytes | Max regression |
|---|---:|---:|---:|
<!-- GAS_BASELINE_END -->

## Regression policy

CI fails a pull request if any entrypoint's CPU instructions or memory bytes rise
by more than its threshold (10% by default) relative to the baseline. The job
prints a per-entrypoint table and, on failure, lists each offending entrypoint
with its old cost, new cost and allowed increase.

If the extra cost is intended (for example, a new security check), update the
baseline in the same PR so reviewers see the cost change explicitly:

```bash
cd contracts
cargo bench -p contract-benches --bench gas_benchmarks | tee gas_output.txt
python3 ../scripts/check_gas_regression.py gas_output.txt ../docs/GAS_COSTS.md \
  --thresholds benches/gas-thresholds.toml --update-baseline
```

Prefer updating the baseline over raising a threshold. Thresholds should only be
tightened.

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
