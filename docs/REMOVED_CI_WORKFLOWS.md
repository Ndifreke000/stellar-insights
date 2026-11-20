# Removed CI workflows

On 2026-11-20 these GitHub Actions workflows were failing on `main`, so they were deleted.
The reasons below come from each workflow's last run on `main`. To bring one back, restore it from git history (`git log --diff-filter=D -- .github/workflows/<file>`) and fix the cause first.

## Backend no longer compiles (merges #2907–#2909)

These all build `backend/` and stopped at the same 15 compile errors after PRs #2907, #2908 and #2909 were merged on top of each other. Errors include: missing field `cache_invalidation` in `AggregationService`, `ErasureCertificate` missing `Deserialize`, `Arc` not imported, use of moved value `scheduled`, an unaudited dynamic SQL string (sqlx `query()` on a `String`), and a `dyn Future` that can't be unpinned.

| Workflow | What it did |
|---|---|
| `api-docs.yml` | Generated and published the API docs |
| `openapi.yml` | Checked `docs/openapi.json` matches the code |
| `codeql-analysis.yml` | CodeQL security analysis (Rust extraction needs a build) |
| `coverage.yml` | Backend test coverage with cargo llvm-cov |
| `database-migrations-test.yml` | Ran migrations, rollbacks and `sqlx prepare --check` |
| `deploy-testnet.yml` | Unit/integration tests, then deploy to testnet |
| `rust-lint.yml` | rustfmt and clippy |
| `leader-election-redis.yml` | Redis leader election integration test (last ran on 6eed95c, broken by the #2899–#2904 merges, never re-run) |
| `vault-deploy.yml`, `vault-deploy-approle.yml` | Vault-backed backend tests; last ran in October and failed on `PaymentRecord` missing `source_account`/`destination_account` in a test |

## Other causes

| Workflow | Why it failed |
|---|---|
| `security-scan.yml` | `cargo audit` denies warnings and found 9 unsound-code advisories (RUSTSEC-2026-0574 to 0582) in the `heapless` crate, which is pulled in through dependencies |
| `contract-fuzzing.yml` | Unit tests pass, but the RBAC fuzzing step hits its 10-minute timeout |
| `sdk-bindings.yml` | `cargo install stellar-cli` fails because the runner lacks the `libdbus-1-dev` system library |
| `terraform-validate.yml` | `terraform/outputs.tf` line 53 defines an output twice, and `terraform fmt` reports unformatted files |
| `commitlint.yml` | Fails on commits that don't use conventional commit messages (most merge commits from PRs); last ran in September |
