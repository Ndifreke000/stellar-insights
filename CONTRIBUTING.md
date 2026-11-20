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

## Database migrations

### Overview

We use the **expand/contract pattern** for database migrations to support
rolling deployments. This ensures that old and new versions of the application
can run simultaneously without schema incompatibility issues.

### Types of Migrations

#### Expand Phase (Always Safe)

These changes are safe to apply during rolling deployments:

- `CREATE TABLE` — New tables don't affect existing code
- `ALTER TABLE ADD COLUMN` — Adding columns is backward compatible
- `CREATE INDEX` — Indexes don't affect query results
- Data migrations — `INSERT`/`UPDATE` that don't change schema

**Example:**
```sql
-- migrations/061_add_trust_score.sql
-- contract-phase: none (expansion is always safe)

ALTER TABLE anchors ADD COLUMN trust_score REAL DEFAULT 0;
CREATE INDEX idx_anchors_trust_score ON anchors(trust_score);
```

#### Contract Phase (Requires Marker)

These changes are **destructive** and require a `contract-phase:` marker:

| Operation | Example |
|-----------|---------|
| `DROP TABLE` | `DROP TABLE old_table` |
| `DROP COLUMN` | `ALTER TABLE DROP COLUMN old_col` |
| `RENAME COLUMN` | `ALTER TABLE RENAME COLUMN a TO b` |
| `DROP INDEX` | `DROP INDEX old_index` |

**Example:**
```sql
-- migrations/062_remove_legacy_trust_score.sql
-- contract-phase: after all pods are upgraded to v1.2.0

-- Remove legacy trust score column after all pods use trust_score
ALTER TABLE anchors DROP COLUMN legacy_trust_score;
```

### The Contract Phase Marker

Every destructive migration must include the `contract-phase:` comment:

```sql
-- contract-phase: <description>
```

**Description should include:**
- Which version all pods must be upgraded to
- Any data migration steps
- Verification criteria

### Migration Workflow

1. **Add new columns/tables (expand phase)**
   - Deploy: Old pods work, new pods can read new columns
   - No downtime

2. **Update application code to use new schema**

3. **Remove old columns/tables (contract phase)**
   - Deploy: Only after all pods are on new version
   - Uses `contract-phase:` marker for CI linting

### CI Linter

The migration linter (`backend/scripts/lint-migrations.sh`) checks for:

- Destructive statements without `contract-phase:` marker → Error
- `contract-phase:` marker on safe migrations → Warning

Run it locally:
```bash
cd backend
./scripts/lint-migrations.sh
```

### Best Practices

1. **Always expand first, contract later**
   - Add columns before removing old ones
   - Keep both schemas compatible during transition

2. **Document the contract phase**
   - Specify version requirement clearly
   - Include data migration steps if any

3. **Test with both versions**
   - Old code must read new schema
   - New code must handle missing columns gracefully

4. **Rollback planning**
   - Each migration has a `.down.sql` file
   - Test rollback in staging before production

### CI/CD Integration

See `.github/workflows/migration-safety.yml` for the migration safety pipeline.

### Examples

See `backend/migrations/061_add_trust_score.sql` and
`backend/migrations/062_remove_legacy_trust_score.sql` for complete examples.
