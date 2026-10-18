# Security Policy

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report vulnerabilities privately through GitHub's
[private vulnerability reporting](../../security/advisories/new) ("Security" tab → "Report a vulnerability").
Include:

- A description of the issue and its impact
- Steps to reproduce or a proof of concept
- Affected component (backend, frontend, contracts, mobile, SDKs) and version/commit

We aim to acknowledge reports within **3 business days** and to provide a remediation
plan within **10 business days**.

## Supported Versions

Only the latest release on `main` receives security fixes.

## Dependency Vulnerability Management

Dependencies are scanned by the [Dependency Vulnerability Scan](.github/workflows/security-scan.yml)
workflow on every push and pull request to `main`/`develop`, and weekly on Mondays.

| Check | Tool | Fails when |
|-------|------|------------|
| Rust (backend, contracts) | `cargo audit --deny warnings` | Any RustSec advisory not ignored in `backend/.cargo/audit.toml` |
| Frontend | `pnpm audit --prod` | High/critical advisory in a production dependency (on `main` and scheduled runs) |
| Pull requests | `actions/dependency-review-action` | The PR adds a dependency with a high/critical advisory |
| Python SDK | `pip-audit` | Any known vulnerability |
| SBOM | `cargo-cyclonedx`, `@cyclonedx/cyclonedx-npm` | — (CycloneDX SBOMs uploaded as `backend-sbom` / `frontend-sbom` artifacts, kept 90 days) |

When a scan fails on `main` or on the schedule, the workflow opens (or comments on) a
`[Security] Dependency vulnerabilities detected` issue labelled `security`.

On pull requests, the frontend's pre-existing advisories are reported as a warning rather
than blocking, so unrelated PRs are not held up; new vulnerable dependencies are still
blocked by dependency review.

[Dependabot](.github/dependabot.yml) opens weekly update PRs for all Cargo, npm and
GitHub Actions manifests. Dependabot security alerts and security updates should be enabled
in the repository settings (Settings → Code security).

### Patching SLA

| Severity | Target time to patch |
|----------|----------------------|
| Critical | 48 hours |
| High | 7 days |
| Medium | 30 days |
| Low | Next scheduled dependency update |

### Ignoring an Advisory

Ignoring an advisory is a last resort for issues with no available fix or that are provably
unreachable. Add it to `backend/.cargo/audit.toml` with a comment stating:

1. Why the vulnerable code path is not reachable, or why no fix exists
2. How that was confirmed (e.g. `cargo tree -i <crate>`)
3. The condition under which the ignore should be removed

### Running Scans Locally

```bash
# Rust
cargo install cargo-audit --locked
(cd backend && cargo audit --deny warnings)
(cd contracts && cargo audit --deny warnings)

# Frontend
(cd frontend && pnpm audit --prod --audit-level=high)
```
