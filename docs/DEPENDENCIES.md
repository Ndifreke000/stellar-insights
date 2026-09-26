# Dependency Versioning Strategy

Enforced by `scripts/check-dependency-policy.sh`, run by the
**Dependency Policy** workflow on any change to a manifest and weekly on a
schedule.

## Pinning Policy

| Where | Policy | Enforced by |
|---|---|---|
| `frontend/package.json` | **Exact versions.** No `^`, no `~`. | CI (fails the build) |
| `backend/Cargo.toml` | Major/minor ranges (`"1.0"`, `"0.8"`) are acceptable; `Cargo.lock` pins the resolved version. | `Cargo.lock` committed |
| `contracts/Cargo.toml` | Exact patch version for `soroban-sdk`, via `[workspace.dependencies]`. | reviewed on change |

Both lockfiles (`frontend/package-lock.json`, `Cargo.lock`) are committed and
are the source of truth for reproducible builds.

### Why the frontend is stricter than the backend

`^` on an npm dependency means a `pnpm install` on a machine with a cold cache
can resolve a different minor version than CI did, so a build that passes in CI
can fail locally — or worse, the reverse. The lockfile normally prevents that,
but it stops protecting you the moment anyone runs `install` without
`--frozen-lockfile`, which is the default for most people most of the time.

Cargo does not have the same hazard: `cargo build` will not silently update
`Cargo.lock`, so a range in `Cargo.toml` is pinned in practice by the committed
lockfile.

### Keeping the manifest honest

This document claimed exact versions for some time while 24 frontend
dependencies actually used carets — including `lucide-react ^1.31.0` resolving
to `1.46.0` and `next-intl ^4.8.3` resolving to `4.13.0`, fifteen and five minor
versions ahead of what the manifest said. A documented policy nothing enforces
is not a policy; it is a comment that happens to be wrong, and the longer it
stays wrong the less anyone trusts the rest of the file.

Those are now pinned to the versions the lockfile already resolved — so nothing
changed about what gets installed, only about what the manifest admits to — and
CI fails on a new range specifier.

## Updating Dependencies

1. Change the version in `package.json` or `Cargo.toml`.
2. Run `pnpm install` (frontend) or `cargo update -p <crate>` (backend) to
   regenerate the lockfile. Prefer the targeted `-p` form: a bare
   `cargo update` moves every transitive dependency at once, which turns a
   one-line bump into an unreviewable diff.
3. Run the test suite.
4. Say in the PR description *why* — a version number alone does not tell a
   reviewer whether this is a security fix, a feature you need, or Dependabot.

## Automated Updates

Use [Dependabot](https://docs.github.com/en/code-security/dependabot) or
[Renovate](https://docs.renovatebot.com/). Review each PR individually — do not
auto-merge. For the frontend, configure the bot to write exact versions, or its
PRs will reintroduce the ranges CI rejects.

## Unused Dependencies

`scripts/check-dependency-policy.sh` flags crates declared in
`backend/Cargo.toml` that appear nowhere in `backend/src/`.

It is a grep approximation of [`cargo-udeps`](https://github.com/est31/cargo-udeps),
which needs a nightly toolchain and a full build — too slow and too fragile to
gate every PR on. It catches the case that actually accumulates: a crate added
for something later removed, leaving the declaration behind.

**False positives are possible** — a crate used only through a macro that does
not name it will be flagged. Confirm before removing rather than deleting on the
script's word.

For a thorough audit, run the real tools periodically:

```bash
cargo +nightly udeps --all-targets        # backend and contracts
npx depcheck frontend                     # frontend
```

### Audit of 2026-09-25

| Dependency | Finding | Action |
|---|---|---|
| `tracing-logstash` | No reference anywhere in the repository | removed |
| `ndarray` | Referenced only by its own `Cargo.toml` line | removed |
| `tokio-tungstenite` | Used only by `backend/tests/testnet/smoke_test.rs` | moved to `[dev-dependencies]` |
| `lazy_static` | **In use** — `observability/db_performance.rs`, `observability/frontend_metrics.rs` | kept |
| `@axe-core/cli` | Not present in the repository at all | no action |

The last two were named as suspected-unused in the originating issue. Both
claims were stale: `lazy_static` is used in three files, and `@axe-core/cli` has
never been a dependency — the repository has `@axe-core/playwright`, `axe-core`,
`jest-axe` and `vitest-axe`, all of which are used.

The frontend audit found **no** unused dependencies across its 51 entries.

## Soroban SDK Version

`contracts/Cargo.toml` pins `soroban-sdk` in `[workspace.dependencies]` so every
contract crate resolves the same version.

The scheduled run of the Dependency Policy workflow compares it against the
latest stable on crates.io and **warns** rather than failing. A major SDK bump
changes the compiled WASM, which means a rebuild, a fresh hash, and a redeploy
with any state migration that implies. That is a deliberate release decision,
not something a CI check should pressure anyone into on a Monday morning.

As of 2026-09-25: declared **27.0.2**, latest stable **28.0.0** — one major
behind. Bumping it is tracked separately from the version *check*, because it
needs contract tests and a testnet deploy to validate, not a one-line edit.

## Security Audits

- Frontend: `pnpm audit` — run in CI on every PR
- Backend: `cargo audit` — run in CI on every PR
