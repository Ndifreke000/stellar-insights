# Dependency Versioning Strategy

## Pinning Policy

All production and dev dependencies use **exact versions** (no `^` or `~` ranges).

- **Frontend** (`frontend/package.json`): exact versions, managed via `pnpm-lock.yaml`
- **Backend** (`backend/Cargo.toml`): major/minor ranges (e.g. `"1.0"`, `"0.8"`), pinned via `Cargo.lock`

Both lockfiles (`pnpm-lock.yaml`, `Cargo.lock`) are committed to the repository and are the source of truth for reproducible builds.

## Updating Dependencies

1. Update the version in `package.json` or `Cargo.toml`
2. Run `pnpm install` (frontend) or `cargo update` (backend) to regenerate the lockfile
3. Run the full test suite before merging
4. Document the reason for the update in the PR description

## Automated Updates

Use [Dependabot](https://docs.github.com/en/code-security/dependabot) or [Renovate](https://docs.renovatebot.com/) for automated PRs on dependency updates. Review each PR individually — do not auto-merge.

## Security Audits

- Frontend: `pnpm audit` — run in CI on every PR
- Backend: `cargo audit` — run in CI on every PR
