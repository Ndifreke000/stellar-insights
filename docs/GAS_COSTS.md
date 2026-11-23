# Gas Costs

Gas costs for the `payraider` Soroban contract — the only contract in the
off-ramp product (everything else is in [`contracts/archive/`](../contracts/archive/README.md)).

CI gates on **host-budget CPU instructions, memory bytes, ledger entries
read/written and bytes read/written** for **every public entrypoint**,
measured by [`contracts/benches/benches/gas_benchmarks.rs`](../contracts/benches/benches/gas_benchmarks.rs).
These are deterministic for a given contract build — the same figures every
run, on any machine — so a tight per-entrypoint threshold doesn't flake the way
wall-clock timings would.

## Running locally

```bash
cd contracts

# Per-entrypoint CPU / memory (what CI checks)
cargo bench -p contract-benches --bench gas_benchmarks | tee gas_output.txt

# Compare against the baseline below using the per-entrypoint thresholds,
# writing JSON results and a markdown diff table
python3 ../scripts/check_gas_regression.py gas_output.txt ../docs/GAS_COSTS.md \
  --thresholds benches/gas-thresholds.toml \
  --json-out gas_results.json --markdown-out gas_diff.md

# Unit tests for the checker
python3 -m unittest discover -s ../scripts/tests

# Wall-clock timings (informational only, not gated in CI)
cargo bench -p contract-benches --bench contract_benchmarks
open target/criterion/report/index.html
```

`check_gas_regression.py` needs Python 3.11+ (for `tomllib`).

## What is measured

Each entrypoint is measured once, against a contract with one authorised
submitter and 50 snapshots already stored, so map-backed reads and writes are
costed at a realistic size.

The benchmark keeps a list of every public entrypoint (`ENTRYPOINTS` in
`gas_benchmarks.rs`) and exits non-zero if any is not measured exactly once, so
a new entrypoint can't ship unbenchmarked. Notable setups:

| Entrypoint | What it measures |
|---|---|
| `payraider::submit_snapshot` | Anchoring snapshot 51 as an authorised submitter |
| `payraider::get_snapshot` / `latest_snapshot` | Reads from the 50-snapshot store |
| `payraider::bump_ttl` | TTL extension of instance, snapshots and every indexed role entry |
| `payraider::propose_admin` / `accept_admin` / `cancel_admin_transfer` | Two-step admin transfer (#2783) |
| `payraider::schedule_upgrade` / `cancel_upgrade` / `set_upgrade_delay` | Timelocked upgrade bookkeeping |
| `payraider::execute_upgrade` | Everything up to the Wasm install, which the bench host rejects because no real Wasm is uploaded |
| `payraider::migrate` | v0 → v1 storage migration |
| views (`get_*`, `is_paused`, `has_role`) | Single reads against the populated fixture |

`Entries read` counts every ledger entry the invocation touched; `Bytes read`
is the disk-read figure, which is almost always 0 because live state is held in
memory by the host.

## Thresholds

Per-entrypoint thresholds live in
[`contracts/benches/gas-thresholds.toml`](../contracts/benches/gas-thresholds.toml).
Every entrypoint is currently held to **5%**:

```toml
[entrypoints]
"payraider::submit_snapshot" = 0.05
# ...
```

A threshold applies to every gated metric — exceeding any one fails CI. Small
counts move in big steps, so one extra storage read or write (e.g. 4 → 5
entries, +25%) always fails, which is the point. A metric that was 0 in the
baseline and is now non-zero also fails. The check also fails if an entrypoint is measured but has no
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
| Entrypoint | CPU instructions | Memory bytes | Entries read | Entries written | Bytes read | Bytes written | Max regression |
|---|---:|---:|---:|---:|---:|---:|---:|
| `payraider::accept_admin` | 464237 | 166291 | 6 | 4 | 0 | 1180 | 5% |
| `payraider::add_submitter` | 269613 | 109139 | 4 | 2 | 0 | 1076 | 5% |
| `payraider::bump_ttl` | 197112 | 97286 | 4 | 0 | 0 | 0 | 5% |
| `payraider::cancel_admin_transfer` | 271316 | 109271 | 4 | 2 | 0 | 1036 | 5% |
| `payraider::cancel_upgrade` | 291260 | 116566 | 4 | 2 | 0 | 1076 | 5% |
| `payraider::execute_upgrade` | 308475 | 131104 | 6 | 1 | 0 | 0 | 5% |
| `payraider::get_admin` | 100178 | 58844 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_contract_info` | 127628 | 60964 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_governance` | 100952 | 58950 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_latest_epoch` | 102060 | 58951 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_metadata` | 73848 | 56199 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_pending_admin` | 100748 | 58952 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_pending_upgrade` | 100748 | 58954 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_role_holders` | 103406 | 58951 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_snapshot` | 119272 | 75885 | 2 | 0 | 0 | 0 | 5% |
| `payraider::get_storage_version` | 102060 | 58954 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_submitters` | 100952 | 58950 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_upgrade_delay` | 100014 | 58952 | 1 | 0 | 0 | 0 | 5% |
| `payraider::get_version` | 99968 | 58844 | 1 | 0 | 0 | 0 | 5% |
| `payraider::grant_role` | 367196 | 143188 | 5 | 3 | 0 | 1220 | 5% |
| `payraider::has_role` | 94540 | 66078 | 2 | 0 | 0 | 0 | 5% |
| `payraider::initialize` | 109341 | 42816 | 3 | 2 | 0 | 572 | 5% |
| `payraider::is_paused` | 97762 | 58844 | 1 | 0 | 0 | 0 | 5% |
| `payraider::latest_snapshot` | 139204 | 78189 | 2 | 0 | 0 | 0 | 5% |
| `payraider::migrate` | 312566 | 119709 | 5 | 2 | 0 | 1036 | 5% |
| `payraider::pause` | 303609 | 120525 | 5 | 2 | 0 | 1036 | 5% |
| `payraider::propose_admin` | 265579 | 110881 | 4 | 2 | 0 | 1108 | 5% |
| `payraider::remove_submitter` | 267413 | 109547 | 4 | 2 | 0 | 1036 | 5% |
| `payraider::revoke_role` | 320152 | 122250 | 5 | 3 | 0 | 1036 | 5% |
| `payraider::rotate_submitter` | 416275 | 158062 | 7 | 3 | 0 | 1148 | 5% |
| `payraider::schedule_upgrade` | 330124 | 134480 | 5 | 2 | 0 | 1300 | 5% |
| `payraider::set_admin_by_governance` | 430012 | 157989 | 6 | 4 | 0 | 1180 | 5% |
| `payraider::set_governance` | 257115 | 107473 | 4 | 2 | 0 | 1036 | 5% |
| `payraider::set_paused_by_governance` | 272753 | 115153 | 4 | 2 | 0 | 1036 | 5% |
| `payraider::set_upgrade_delay` | 269792 | 110325 | 4 | 2 | 0 | 1076 | 5% |
| `payraider::submit_snapshot` | 429885 | 155005 | 5 | 3 | 0 | 1412 | 5% |
| `payraider::unpause` | 306690 | 120797 | 5 | 2 | 0 | 1036 | 5% |
<!-- GAS_BASELINE_END -->

## Regression policy

CI fails a pull request if any gated metric of any entrypoint rises by more
than its threshold (5% by default) relative to the baseline. The job:

- prints a per-entrypoint markdown diff table and, on failure, lists each
  offending metric with its old value, new value and allowed increase;
- adds the table to the job summary and posts (or updates) it as a PR comment;
- uploads `gas_output.txt`, `gas_results.json` and `gas_diff.md` as the
  `gas-results-<sha>` artifact.

If the extra cost is intended (for example, a new security check), update the
baseline in the same PR so reviewers see the cost change explicitly:

```bash
cd contracts
cargo bench -p contract-benches --bench gas_benchmarks | tee gas_output.txt
python3 ../scripts/check_gas_regression.py gas_output.txt ../docs/GAS_COSTS.md \
  --thresholds benches/gas-thresholds.toml --update-baseline
```

Commit the regenerated `docs/GAS_COSTS.md` with a sentence in the PR saying why
the cost went up. Prefer updating the baseline over raising a threshold. Thresholds should only be
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
