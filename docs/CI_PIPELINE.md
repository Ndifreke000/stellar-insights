# CI Pipeline Consolidation

> Tracking issue: [#2849](https://github.com/Ndifreke000/stellar-insights/issues/2849)
> Status: **Design / proposed.** Nothing in this doc is implemented yet. It records the current state and the target layout.

## Problem

`.github/workflows/` contains **34** workflow files. (The issue says 35. One was removed after it was written.) Many of them install the same toolchain, warm or skip the same caches, and run on overlapping triggers. A frontend-only PR can still start Rust jobs, and a backend PR compiles the crate several times in separate workflows. The result is slow, flaky and expensive CI.

## Current inventory

Below, "PR" means the workflow runs on `pull_request`. "Paths" means it already has a `paths:` filter. "Cache" means it uses `actions/cache`, `Swatinem/rust-cache` or `setup-*` `cache:`.

### Runs on every PR (no path filter)

| Workflow | Jobs | Cache | Notes |
| --- | --- | --- | --- |
| `rust-lint.yml` | `fmt`, `clippy` | ✗ | Builds backend **and** contracts on every PR, even frontend-only ones |
| `coverage.yml` | `backend-coverage`, `frontend-coverage` | ✓ | Full backend compile plus frontend test run |
| `security-scan.yml` | `cargo-audit`, `cargo-sbom`, `npm-audit`, `npm-sbom`, `pip-audit`, `dependency-review`, `alert`, `resolve` | partial | Also weekly cron |
| `codeql-analysis.yml` | `analyze` | ✗ | Also nightly cron |
| `commitlint.yml` | `commitlint` | ✗ | Cheap |
| `enforce-folder-size.yml` | `policy_check` | ✗ | Also runs on every `push` to any branch |
| `secret-scanning.yml` | `trufflehog`, `tracked-credentials` | ✗ | Cheap |

### PR, path-filtered

| Workflow | Paths | Jobs | Cache |
| --- | --- | --- | --- |
| `frontend-build.yml` | `frontend/**` | `build` | ✓ |
| `frontend-lint.yml` | `frontend/**` | `lint` | ✓ |
| `accessibility.yml` | `frontend/**` | `a11y`, `playwright-a11y` | ✓ |
| `performance-budget.yml` | `frontend/**` | `bundle-budget` | ✓ |
| `api-docs.yml` | `backend/**` | `openapi` | ✗ |
| `openapi.yml` | `backend/src/**`, `Cargo.*` | `check-openapi` | ✓ |
| `database-migrations-test.yml` | `backend/migrations/**`, … | `test-migrations` | ✓ |
| `leader-election-redis.yml` | leader election sources | `redis-leader-election` | ✓ |
| `rate-limit-redis.yml` | rate limit sources | `redis-rate-limit` | ✓ |
| `contract-fuzzing.yml` | `contracts/**` | `fuzz` | ✓ |
| `gas-benchmarks.yml` | `contracts/**` | `gas` | ✗ |
| `gas-regression.yml` | `contracts/**` | `gas-benchmarks` | ✓ |
| `sdk-bindings.yml` | `contracts/**/*.rs`, `Cargo.toml` | `generate-typescript-bindings` | ✗ |
| `plugin.yml` | `sdk/typescript/**`, `sdk/mcp-server/**`, `plugins/**` | `test`, `docker` | ✓ |
| `dependency-policy.yml` | manifests + policy script | `policy` | ✗ |

### Not on PRs (deploy, scheduled, manual)

| Workflow | Trigger |
| --- | --- |
| `deploy.yml` | push to `main` (`backend/**`), dispatch |
| `deploy-testnet.yml` | push to `main` (`backend/**`, `k8s/**`), dispatch |
| `deploy-mainnet.yml` | `v*` tag, dispatch |
| `rollback.yml` | dispatch |
| `changelog.yml` | `v*` tag, dispatch |
| `backup-database.yml` | daily and weekly cron, dispatch |
| `e2e-nightly.yml` | nightly cron, dispatch |
| `zap-scan.yml` | nightly cron, dispatch |
| `load-tests.yml` | dispatch, cron, release |
| `terraform-validate.yml` | dispatch |
| `vault-deploy.yml`, `vault-deploy-approle.yml` | dispatch |

### Overlaps

1. **Duplicate gas benchmarks.** `gas-benchmarks.yml` and `gas-regression.yml` have the same trigger and the same purpose.
2. **Duplicate OpenAPI checks.** `api-docs.yml` (`openapi`) and `openapi.yml` (`check-openapi`) both regenerate and check the spec on backend changes.
3. **Repeated backend compiles.** A backend PR compiles the backend in `rust-lint` (clippy), `coverage`, `openapi`, `api-docs`, `security-scan` (`cargo-sbom`) and up to three integration workflows. There is no shared compile cache.
4. **Repeated frontend installs.** A frontend PR runs `npm ci` four times: build, lint, a11y and bundle budget.
5. **Unfiltered Rust lint.** `rust-lint.yml` compiles both Rust workspaces with no path filter.
6. **`enforce-folder-size.yml`** runs on every push to every branch as well as on PRs, so each PR commit runs it twice.

## Target layout

```
.github/workflows/
  ci.yml                    # the single PR / push-to-main entry point
  _rust.yml                 # reusable: fmt, clippy, test, coverage (workflow_call)
  _node.yml                 # reusable: install, lint, typecheck, test, build (workflow_call)
  _python.yml               # reusable: lint, test (workflow_call)
  security.yml              # codeql + audits + secret scan (PR + cron)
  deploy.yml / deploy-testnet.yml / deploy-mainnet.yml / rollback.yml   # unchanged
  nightly.yml               # e2e, zap, load (merged cron jobs)
  ops.yml                   # backup verify, terraform, vault (dispatch / cron)
  release.yml               # changelog + publish (see SUPPLY_CHAIN_SECURITY.md)
```

That is roughly **12 files instead of 34**. The deploy workflows stay separate on purpose, because they have their own approvals and environments.

### `ci.yml`: one entry point with change detection

```yaml
name: CI
on:
  pull_request:
  push:
    branches: [main]

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

jobs:
  changes:
    runs-on: ubuntu-latest
    outputs:
      backend:   ${{ steps.f.outputs.backend }}
      contracts: ${{ steps.f.outputs.contracts }}
      frontend:  ${{ steps.f.outputs.frontend }}
      sdk_ts:    ${{ steps.f.outputs.sdk_ts }}
      sdk_py:    ${{ steps.f.outputs.sdk_py }}
      mobile:    ${{ steps.f.outputs.mobile }}
    steps:
      - uses: actions/checkout@v4
      - id: f
        uses: dorny/paths-filter@v3
        with:
          filters: .github/path-filters.yml

  backend:
    needs: changes
    if: needs.changes.outputs.backend == 'true'
    uses: ./.github/workflows/_rust.yml
    with: { working-directory: backend, coverage: true, openapi-check: true }

  contracts:
    needs: changes
    if: needs.changes.outputs.contracts == 'true'
    uses: ./.github/workflows/_rust.yml
    with: { working-directory: contracts, gas-benchmarks: true, fuzz-smoke: true }

  frontend:
    needs: changes
    if: needs.changes.outputs.frontend == 'true'
    uses: ./.github/workflows/_node.yml
    with: { working-directory: frontend, a11y: true, bundle-budget: true }

  # sdk_ts, sdk_py, mobile follow the same pattern

  ci-ok:
    if: always()
    needs: [changes, backend, contracts, frontend, sdk-ts, sdk-py, mobile]
    runs-on: ubuntu-latest
    steps:
      - name: Fail if any required job failed or was cancelled
        run: |
          echo '${{ toJSON(needs) }}' | jq -e '
            [.[] | .result] | all(. == "success" or . == "skipped")'
```

### Path filters (`.github/path-filters.yml`)

Each package declares the paths that should trigger it, including shared files that affect it:

```yaml
backend:
  - 'backend/**'
  - '.github/workflows/ci.yml'
  - '.github/workflows/_rust.yml'
contracts:
  - 'contracts/**'
  - '.github/workflows/_rust.yml'
frontend:
  - 'frontend/**'
  - 'package-lock.json'
  - '.github/workflows/_node.yml'
sdk_ts:
  - 'sdk/typescript/**'
  - 'sdk/react/**'
  - 'sdk/mcp-server/**'
  - 'plugins/**'
  - 'contracts/**/*.rs'   # generated bindings
sdk_py:
  - 'sdk/python/**'
mobile:
  - 'mobile/**'
```

Changes to `.github/path-filters.yml` itself should trigger **every** package.

### Reusable workflows and caching

**`_rust.yml`** (`on: workflow_call`):

- `dtolnay/rust-toolchain` pinned to the version in `rust-toolchain.toml`.
- **sccache** backed by the GitHub Actions cache (`mozilla-actions/sccache-action`, `SCCACHE_GHA_ENABLED=true`, `RUSTC_WRAPPER=sccache`).
- `Swatinem/rust-cache` for `~/.cargo/registry` and `target/`, keyed on `Cargo.lock` and the working directory.
- Compile once, then run `fmt --check`, `clippy`, `nextest`, and optionally `llvm-cov`, the OpenAPI diff and gas benchmarks as steps in **one job**, so the `target/` directory is reused.
- Integration suites that need Redis (`leader-election-redis`, `rate-limit-redis`) become a matrix job in `_rust.yml` with a `services: redis` container. They are gated on their own narrower sub-filter.

**`_node.yml`** (`on: workflow_call`):

- `actions/setup-node` with `cache: npm` and `cache-dependency-path` set to the package's lockfile.
- One `npm ci`, then `lint`, `typecheck`, `test` and `build` as steps. The a11y and bundle-budget steps reuse the build output instead of rebuilding.
- Playwright browsers cached under `~/.cache/ms-playwright`, keyed on the Playwright version.

**`_python.yml`** (`on: workflow_call`):

- `actions/setup-python` with `cache: pip`, then `ruff` and `pytest` for `sdk/python`.

### Required checks

Branch protection should require **only** `CI / ci-ok` plus `Security / summary`. Because `ci-ok` treats `skipped` as passing, path-filtered jobs that do not run never block a merge. Adding or renaming an inner job then needs no change to branch protection.

## Migration plan

1. Add `ci.yml`, the reusable workflows and `path-filters.yml` **alongside** the existing workflows. Mark `ci-ok` as not required yet.
2. Run both systems for about a week and compare results and timings (see the test plan).
3. Switch branch protection to `ci-ok`.
4. Delete the superseded workflows: `rust-lint`, `coverage`, `frontend-build`, `frontend-lint`, `accessibility`, `performance-budget`, `api-docs`, `openapi`, `gas-benchmarks`, `gas-regression`, `contract-fuzzing` (the PR smoke run; the long fuzz run moves to `nightly.yml`), `sdk-bindings`, `plugin`, `leader-election-redis`, `rate-limit-redis`, `database-migrations-test`, `enforce-folder-size`, `commitlint` (moves into `ci.yml` as a cheap job).
5. Merge the cron workflows into `nightly.yml` and `ops.yml`.
6. Update `CONTRIBUTING.md` with the new check name and how to add a package to `path-filters.yml`.

## Test plan: measuring the improvement

Collect the median wall-clock time from PR open to all checks finished, for the last 50 merged PRs before and after the switch:

```bash
gh run list --workflow <name> --event pull_request --limit 200 \
  --json databaseId,createdAt,updatedAt,conclusion,headBranch \
  > runs.json
# group by headBranch and take max(updatedAt) - min(createdAt) per PR
```

Report these, split by PR type (frontend-only, backend-only, contracts-only, mixed):

| Metric | Before | After |
| --- | --- | --- |
| Median PR CI wall-clock | _tbd_ | _tbd_ |
| Billable runner minutes per PR (median) | _tbd_ | _tbd_ |
| Workflow files | 34 | ~12 |
| Flaky re-runs per week | _tbd_ | _tbd_ |

Fill in the table and attach it to the PR that closes #2849.

## Acceptance criteria

- [ ] Fewer workflows (target is about 12, from 34)
- [ ] Faster median CI time, reported in the table above
- [ ] `ci-ok` is the single required check and is green
- [ ] No new TODOs without a linked issue
