# Dependency Security Audit — 2026-07-26

Closes #1874. Triage of `cargo audit` findings in `backend/` and `contracts/`.

**Scope note:** `npm audit` in `frontend/` and `mobile/` was not run for this
report — running it needs `npm ci` first, which is out of scope for how this
PR was produced. All three of the existing scheduled workflows below already
run `npm audit` on their normal cadence (weekly + every push/PR); their next
runs will surface current npm findings for a follow-up triage pass the same
way this one covers `cargo audit`.

## Existing CI coverage (already in place — no new workflow needed)

Before adding anything, checked what's already scheduled. Three overlapping
workflows already run dependency audits on push/PR and weekly on a cron:

- `.github/workflows/security-audit.yml` — `cargo audit` (backend + contracts)
  + SBOM generation + `npm audit` (frontend) + dependency-review on PRs.
- `.github/workflows/security-scan.yml` — `cargo audit` (via
  `rustsec/audit-check`) + `npm audit` (root, frontend, mobile,
  `sdk/typescript`) + `pip-audit` (`sdk/python`).
- `.github/workflows/security.yml` — `npm audit` (frontend only), fails the
  build specifically on **critical** findings.

The acceptance criteria's "consider adding this as a scheduled CI check" is
already satisfied three times over. The actual gap is that
`security-audit.yml`'s cargo-audit step uses `continue-on-error: true` and
`security-scan.yml`'s uses `rustsec/audit-check` without `--deny warnings`,
so findings have been accumulating silently without ever being reviewed —
matching this issue's own premise. **Worth a follow-up cleanup to consolidate
these three into one** (out of scope here to avoid touching CI behavior in
the same PR as a triage report), but not blocking this triage.

## `backend/` — `cargo audit` (577 crate dependencies scanned)

| Crate | Advisory | Severity | Reachable? | Disposition |
|---|---|---|---|---|
| `crossbeam-epoch` 0.9.18 | RUSTSEC-2026-0204 (invalid pointer deref in `fmt::Pointer`) | — | `criterion` → `rayon` → **dev-dependency only** (benchmarks) | **Accepted risk.** Never compiled into the shipped binary. |
| `idna` 0.5.0 | RUSTSEC-2024-0421 (accepts invalid Punycode) | — | `validator` → **production dependency** | **Needs upgrade.** `validator` depends on `idna` 0.5; a newer `validator` (or its transitive `idna` bump) is needed. Not applied in this PR — verifying the upgrade doesn't change `validator`'s email/URL validation behavior needs a real build+test pass, which is out of scope for how this PR was produced. |
| `protobuf` 2.28.0 | RUSTSEC-2024-0437 (uncontrolled recursion → crash) | — | `prometheus` → **production dependency** (metrics) | **Needs upgrade.** `prometheus` needs a version pulling in `protobuf` ≥3.7.2. Same verification caveat as `idna` above. |
| `quinn-proto` 0.11.14 | RUSTSEC-2026-0185 (remote memory exhaustion) | 7.5 (high) | **Not reachable** — present in `Cargo.lock` but `cargo tree -i` (incl. `--target all`) finds no path to it from `stellar-insights-backend` | **Accepted risk.** Orphaned lockfile entry from a dependency that no longer activates the feature pulling it in. A `cargo update` would likely drop it, but that's a broader lockfile change than this triage should make unverified. |
| `rsa` 0.9.10 | RUSTSEC-2023-0071 (Marvin timing side-channel) | 5.9 (medium) | **Not reachable** — same as `quinn-proto`: no path found via `cargo tree -i` | **Accepted risk**, same reasoning. Also: upstream has "no fixed upgrade available" regardless. |
| `json` 0.12.4 | RUSTSEC-2022-0081 (unmaintained) | — | `stellar-base`/`xdr-rs-serialize` → `stellar_sdk` → production | **Accepted risk (unmaintained, not a vulnerability).** No CVE, just no longer updated. Pinned transitively via the Stellar SDK; replacing it means replacing `stellar_sdk` itself. |
| `proc-macro-error` 1.0.4 | RUSTSEC-2024-0370 (unmaintained) | — | `utoipa-gen`/`validator_derive` → production (proc-macro, compile-time only) | **Accepted risk.** Compile-time only, not present in the compiled artifact. |
| `sodiumoxide` 0.2.6 | RUSTSEC-2021-0137 (deprecated, author recommends `libsodium-sys`/dalek crates) | — | `stellar-base` → `stellar_sdk` → production | **Accepted risk for now, flag for follow-up.** No CVE, but this is a crypto library — worth a dedicated follow-up issue to migrate off `stellar_sdk` 0.1.4's old `stellar-base` if a newer SDK release drops it. |
| `spin` 0.9.8 | yanked | — | transitive | **Accepted risk.** Yanked (likely a packaging issue on crates.io), not a vulnerability advisory. |

**Action items from this table:**
- [ ] Upgrade `validator` to a version depending on `idna` ≥1.0 (RUSTSEC-2024-0421)
- [ ] Upgrade `prometheus` to a version depending on `protobuf` ≥3.7.2 (RUSTSEC-2024-0437)
- [ ] Follow-up issue: evaluate replacing `stellar_sdk` 0.1.4 (pulls in unmaintained `json`, `sodiumoxide`)

## `contracts/` — `cargo audit` (250 crate dependencies scanned)

No vulnerabilities — only unmaintained/unsound warnings, all accepted:

| Crate | Advisory | Disposition |
|---|---|---|
| `paste` 1.0.15 | RUSTSEC-2024-0436 (unmaintained) | **Accepted risk.** Proc-macro helper, compile-time only. |
| `anyhow` 1.0.102 | RUSTSEC-2026-0190 (unsound `Error::downcast_mut()`) | **Accepted risk.** Contracts don't call `downcast_mut()` on `anyhow::Error`; not a reachable unsound path here. |
| `rand` 0.8.5 / 0.9.2 (both present) | RUSTSEC-2026-0097 (unsound with a custom `rand::rng()` logger) | **Accepted risk.** No custom logger implementation for `rand::rng()` in this codebase. |
| `spin` 0.9.8 | yanked | **Accepted risk**, same as above. |

## `frontend/` and `mobile/` — `npm audit`

Not run in this PR (see scope note above). Tracked as the follow-up half of
this triage once the next scheduled `security-audit.yml`/`security-scan.yml`
run reports current findings.
