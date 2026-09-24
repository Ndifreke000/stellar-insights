# Contributing to PayRaider

Thanks for your interest in contributing! This document covers how to get set
up, the conventions we use, and what to expect from the review process.

## Project layout

This is a monorepo:

- `backend/` — Rust API server
- `contracts/` — Soroban smart contracts (Rust)
- `frontend/` — Next.js app
- `mobile/` — React Native app
- `sdk/` — TypeScript, Python, and MCP-server client SDKs
- `docs/` — architecture, deployment, and operational docs

## Development setup

1. Fork the repo and clone your fork.
2. Backend: `cd backend && cargo build`. Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` before committing.
3. Contracts: `cd contracts && cargo clippy --workspace --all-targets --all-features -- -D warnings`.
4. Frontend: `cd frontend && pnpm install && pnpm dev`.
5. Root-level E2E/acceptance tests: `npm run test:acceptance` (Playwright).

See [`docs/backend-modules.md`](docs/backend-modules.md) and
[`docs/DATABASE_MIGRATIONS.md`](docs/DATABASE_MIGRATIONS.md) for more on the
backend, and [`docs/database-seeding.md`](docs/database-seeding.md) to load
realistic local data via `backend/seed_data.sh`.

## Commit conventions

Commit messages are linted with [commitlint](https://commitlint.js.org/)
against [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<optional scope>): <description>

# examples
fix(backend): handle missing anchor metadata gracefully
feat(frontend): add SEP-31 compliance field validation
docs: clarify local Postgres setup in README
```

Common types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`.

## Opening a pull request

- Keep PRs focused — one logical change per PR is easier to review than a
  large bundle of unrelated fixes.
- Reference the issue(s) your PR resolves with `Closes #123` in the
  description so they close automatically on merge.
- Run the relevant checks locally before pushing (`cargo clippy`, `cargo
  test`, `pnpm lint`, `pnpm build`) — CI runs the same checks and will block
  merge on failures.
- New migrations need a corresponding `.down.sql` rollback file (see
  [`docs/DATABASE_MIGRATIONS.md`](docs/DATABASE_MIGRATIONS.md)).

## Code of conduct

Participation in this project is governed by our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Questions

Use [GitHub Discussions](https://github.com/Ndifreke000/stellar-insights/discussions)
for questions, and GitHub Issues for bugs and feature requests.
