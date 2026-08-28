# Contract Fuzz Verification after the `wasm32v1-none` Migration

Related issue: [#1855](https://github.com/Ndifreke000/stellar-insights/issues/1855).

The contracts build moved from `wasm32-unknown-unknown` to `wasm32v1-none`
(required by the installed Soroban SDK under the current Rust toolchain). A wasm
target change can shift numeric/overflow behaviour in exactly the ways the fuzz
targets exist to catch, so a generic "builds and tests are green" pass is not
sufficient evidence — the fuzz suite runs longer and is not part of the quick
check.

## How to re-run the suite

**In CI** — `.github/workflows/contract-fuzzing.yml` now accepts
`workflow_dispatch`, so it can be triggered on demand without pushing a no-op
commit to `contracts/`:

```bash
gh workflow run contract-fuzzing.yml
```

Then confirm the result:

```bash
gh run list --workflow=contract-fuzzing.yml --limit 5
```

**Locally** — `scripts/run-contract-fuzz.sh` runs the same three commands the
workflow does, with the same `BOLERO_ITERATIONS=1000`:

```bash
./scripts/run-contract-fuzz.sh
```

Equivalent to:

```bash
rustup target add wasm32v1-none
cd contracts
cargo test --package analytics fuzz_ -- --nocapture
cargo test --package governance fuzz_ -- --nocapture
cargo test --workspace -- --nocapture
```

## If something regressed

Bisect the target change against the rest of the session's changes before
assuming the migration is at fault:

1. Re-run the failing fuzz target on the old target to see if it also fails:
   ```bash
   cargo test --target wasm32-unknown-unknown --package analytics fuzz_
   ```
2. If it passes on the old target and fails on `wasm32v1-none`, the migration is
   implicated — capture the failing seed printed by bolero and add it as a
   regression case rather than only re-running the fuzzer.
3. If it fails on both, the cause is elsewhere in the session's changes;
   `git bisect` over `contracts/` narrows it.

## Verification log

| Date | Trigger | Result |
| --- | --- | --- |
| _pending_ | `workflow_dispatch` post-migration | _record the run URL and outcome here_ |
