# Mutation Testing for Security-Critical Modules

> Tracking issue: [#2851](https://github.com/Ndifreke000/stellar-insights/issues/2851)
> Status: **Design / proposed.**

## Why

Line coverage tells you that code ran during tests, not that a test would fail if the code were wrong. Mutation testing changes the code (for example, flips `<` to `<=`, replaces a function body with `Ok(Default::default())`, or deletes a `!`) and checks that at least one test fails. A mutant that no test catches ("survives") marks an assertion gap.

## Scope

| Module | Lines | What must be pinned down |
| --- | --- | --- |
| `backend/src/auth.rs` | 862 | JWT secret checks (length ≥ 32, no `CHANGE_ME`), `validate_token` expiry and signature, refresh-token store, validate and invalidate, `login` → `complete_2fa_login` gating, `logout`, `sep10_auth_middleware` |
| `backend/src/twofa.rs` | 310 | TOTP verification window, backup-code single use (`verify_backup_code`), `is_2fa_enabled` vs. enrolled-but-not-activated, `disable_2fa`, unused-code count |
| `backend/src/crypto.rs` | 214 | `encrypt_data` / `decrypt_data` round trip, rejection of a wrong key or tampered ciphertext, key-length validation, `is_encrypted` detection |
| `contracts/payraider/src/lib.rs` | 1223 | `submit_snapshot` guards (paused, auth, role, epoch zero, `u64::MAX`, zero hash, duplicate, monotonicity), admin, governance and role management, `pause`/`unpause`, `upgrade`/`migrate` |

## Tooling

[cargo-mutants](https://mutants.rs/) works on both the backend crate and the Soroban contract crate.

```bash
cargo install --locked cargo-mutants

# backend: only the three modules in scope
cd backend
cargo mutants \
  --file src/auth.rs --file src/twofa.rs --file src/crypto.rs \
  --test-tool nextest \
  --timeout-multiplier 3 \
  -j 4 \
  --output ../mutants-out/backend

# contract
cd ../contracts/payraider
cargo mutants --file src/lib.rs --output ../../mutants-out/contract
```

Results go to `mutants.out/` (`caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`, `outcomes.json`).

**Mutation score** = caught ÷ (caught + missed). Timeouts count as caught. Unviable mutants (those that do not compile) are excluded.

### Config: `.cargo/mutants.toml`

Add one per crate so local runs and CI behave the same:

```toml
# backend/.cargo/mutants.toml
examine_globs = ["src/auth.rs", "src/twofa.rs", "src/crypto.rs"]
test_tool = "nextest"
timeout_multiplier = 3.0
exclude_re = [
  # see "Exclusions" below; every entry needs a reason
]
```

## Workflow for killing survivors

1. Run cargo-mutants and open `missed.txt`. Each line names the file, the line and the mutation, for example `src/twofa.rs:201: replace == with != in TwoFAService::verify_backup_code`.
2. For each survivor, decide which case applies:
   - **Missing assertion.** Write a test that fails under the mutant. Name the test after the behaviour, not the mutant (`backup_code_cannot_be_reused`, not `kill_mutant_42`).
   - **Equivalent mutant.** The change cannot alter observable behaviour. Add it to `exclude_re` with a comment that explains why.
   - **Dead code.** Delete the code.
3. Run again with `--in-diff` against the branch base to confirm that the new tests kill the mutants.

Examples of survivors we expect from reading the code:

- `auth.rs` `new`: replacing `>= 32` with `> 32` survives unless a test uses a secret of exactly 32 characters.
- `twofa.rs` `verify_backup_code`: deleting the "mark used" update survives unless a test redeems the same code twice.
- `crypto.rs` `decrypt_data`: replacing the body with `Ok(String::new())` survives unless the tests assert the decrypted plaintext rather than just `is_ok()`.
- `lib.rs` `submit_snapshot`: replacing `epoch <= current_latest` with `epoch < current_latest` survives unless a test resubmits the latest epoch with a different hash. (The `DuplicateEpoch` check runs first and catches the same-hash case.)

## CI

Add a non-blocking job, first as its own workflow and later as a step in the consolidated pipeline (see [CI_PIPELINE.md](CI_PIPELINE.md)):

```yaml
name: Mutation testing
on:
  pull_request:
    paths:
      - 'backend/src/auth.rs'
      - 'backend/src/twofa.rs'
      - 'backend/src/crypto.rs'
      - 'contracts/payraider/src/lib.rs'
  schedule:
    - cron: '0 5 * * 0'   # weekly full run
  workflow_dispatch:

jobs:
  mutants:
    runs-on: ubuntu-latest
    continue-on-error: true          # phase 1: report only
    strategy:
      matrix:
        crate: [backend, contracts/payraider]
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: ${{ matrix.crate }} }
      - uses: taiki-e/install-action@v2
        with: { tool: cargo-mutants,cargo-nextest }
      - name: Run (diff-only on PRs, full on schedule)
        working-directory: ${{ matrix.crate }}
        run: |
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            git diff origin/${{ github.base_ref }}... > /tmp/pr.diff
            cargo mutants --in-diff /tmp/pr.diff --no-shuffle
          else
            cargo mutants --no-shuffle
          fi
      - name: Score
        if: always()
        working-directory: ${{ matrix.crate }}
        run: |
          jq -r '[.outcomes[] | .summary] | group_by(.) | map({(.[0]): length}) | add' \
            mutants.out/outcomes.json | tee -a "$GITHUB_STEP_SUMMARY"
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: mutants-${{ strategy.job-index }}
          path: ${{ matrix.crate }}/mutants.out
```

**Phases:**

1. **Report only** (`continue-on-error: true`). The score appears in the job summary and the artifact is attached.
2. **Block** once all four modules are at or above 80%. Remove `continue-on-error` and fail if the PR diff introduces any missed mutant.

## Exclusions

Record every exclusion here with a reason. An exclusion without a reason should be rejected in review.

| Pattern | Crate | Reason |
| --- | --- | --- |
| _none yet_ | | |

Typical acceptable reasons: log or `tracing` calls only; `Debug`/`Display` impls; `bump_instance` TTL extension in the contract (the effect is not observable in unit tests and is covered by the gas and integration suites); equivalent mutants with a short proof.

## Results

Attach the report to the PR that closes #2851:

| Module | Mutants | Caught | Missed | Timeout | Unviable | Score |
| --- | --- | --- | --- | --- | --- | --- |
| `auth.rs` | | | | | | |
| `twofa.rs` | | | | | | |
| `crypto.rs` | | | | | | |
| `payraider/src/lib.rs` | | | | | | |

## Acceptance criteria

- [ ] Mutation score ≥ 80% on each listed module
- [ ] Mutation job runs in CI (non-blocking at first)
- [ ] Exclusions are documented in the table above
- [ ] CI is green (lint, type-check, tests)
- [ ] No new TODOs without a linked issue
