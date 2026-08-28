# Database Migration Workflow

This guide covers how to author, test, and rollback database migrations for the Stellar Insights backend and frontend.

## Overview

- **Backend**: Uses `sqlx` (Rust) with manual SQL migrations in `backend/migrations/`
- **Frontend**: Uses `Prisma` (Node.js) with auto-generated migrations in `frontend/prisma/migrations/`

Both systems support version control and rollback, but the mechanisms differ.

---

## Backend Migrations (Rust + sqlx)

### File Structure

Migrations are stored in `backend/migrations/` with the naming convention:

```
NNN_description.sql         (forward migration)
NNN_description.down.sql    (rollback migration)
```

- **NNN** = sequential 3-digit number (001, 002, 003, etc.)
- **description** = lowercase, underscore-separated, brief description
- Both `.sql` and `.down.sql` files must exist for each migration

**Example:**
```
035_add_token_revocations.sql
035_add_token_revocations.down.sql
```

### Authoring a New Migration

1. **Determine the next migration number**:
   ```bash
   ls backend/migrations/ | grep "\.sql$" | sed 's/_.*//' | sort -n | tail -1
   # Add 1 to get the next number
   ```

2. **Create the forward migration** (`NNN_description.sql`):
   ```sql
   -- Add new column to users table
   ALTER TABLE users ADD COLUMN last_login TIMESTAMP;
   
   -- Create index for performance
   CREATE INDEX idx_users_last_login ON users(last_login DESC);
   ```

3. **Create the rollback migration** (`NNN_description.down.sql`):
   ```sql
   -- Rollback: Drop index
   DROP INDEX IF EXISTS idx_users_last_login;
   
   -- Rollback: Remove column
   ALTER TABLE users DROP COLUMN last_login;
   ```

4. **Test locally**:
   ```bash
   cd backend
   
   # Apply migrations
   sqlx migrate run
   
   # Verify the schema change
   sqlite3 stellar_insights.db ".schema users"
   
   # Test rollback (manual for now)
   sqlite3 stellar_insights.db < migrations/NNN_description.down.sql
   ```

### Backward-Compatible Migrations

When migrating a live system (especially in blue-green deployments), ensure migrations are **backward-compatible** during the cutover window:

**Expand-Contract Pattern:**

1. **Expand Phase** (Deployment 1): Add new column, keep old logic
   ```sql
   ALTER TABLE users ADD COLUMN email_v2 TEXT;
   ```

2. **Contract Phase** (Deployment 2): Switch code to use new column, backfill if needed
3. **Cleanup Phase** (Deployment 3): Remove old column after code fully migrated

This ensures both "blue" (old) and "green" (new) environments work against the same schema during cutover.

---

## Frontend Migrations (Prisma)

### File Structure

Prisma auto-generates migrations in `frontend/prisma/migrations/YYYYMMDDHHMMSS_description/`:

```
frontend/prisma/migrations/
  20260126152426_init/
    migration.sql
```

### Authoring a New Migration

1. **Update schema** in `frontend/prisma/schema.prisma`:
   ```prisma
   model User {
     id    Int     @id @default(autoincrement())
     email String  @unique
     name  String?
     lastLogin DateTime? // New field
   }
   ```

2. **Generate migration**:
   ```bash
   cd frontend
   npx prisma migrate dev --name add_last_login
   ```
   This creates a timestamped migration directory with `migration.sql`.

3. **Review and commit** the migration file.

### Rollback

**Local development:**
```bash
cd frontend
npx prisma migrate resolve --rolled-back YYYYMMDDHHMMSS_description
```

**Production**: Use the runbook in "Rollback Procedures" below.

---

## CI/CD Integration

### Testing Migrations

The `database-migrations-test` CI workflow runs on every push to `backend/` and verifies:

1. Forward migrations apply cleanly against a fresh database
2. Rollbacks execute without errors
3. Schema after rollback matches the state before the migration

**Trigger:**
- Any push to `backend/**`
- Manual trigger via `workflow_dispatch`

**Logs:** Check CI job output for detailed migration status.

### Applying Migrations in Deployment

Migrations are automatically applied when the backend starts via the `sqlx::migrate!()` macro in `src/main.rs`:

```rust
sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .context("Migrations failed")?;
```

This ensures every deployment runs pending migrations before the app becomes healthy.

---

## Rollback Procedures

### Local Development

1. **Identify the migration to roll back**:
   ```bash
   ls backend/migrations/ | tail -5
   ```

2. **Run the down migration**:
   ```bash
   sqlite3 stellar_insights.db < backend/migrations/NNN_description.down.sql
   ```

3. **Restart the app** to re-apply migrations from the current state.

### Staging / Production

1. **Alert on-call / database team** that a rollback is needed.

2. **Trigger the rollback manually** (this requires database access and should be coordinated with DevOps):
   ```bash
   # Connect to the production database
   psql $PRODUCTION_DB_URL < backend/migrations/NNN_description.down.sql
   ```

3. **Verify schema after rollback**:
   ```bash
   # Check the schema matches the previous deployment's expectations
   psql $PRODUCTION_DB_URL -c "\d table_name"
   ```

4. **Redeploy the previous (working) backend image** if needed.

5. **Update the migration version log** (if your system tracks this) to mark the rollback.

### Automated Rollback (Blue-Green Deployment)

In a blue-green deployment, if the green (new) environment fails health checks:

1. Traffic automatically reverts to blue (old) environment
2. **Do NOT run rollback migrations** — the database schema must remain at the "green" state
3. Investigate the issue in the green environment
4. Once fixed, retry deploying green

This is why backward-compatible migrations (expand-contract pattern) are critical.

---

## Best Practices

1. **Always create both `.sql` and `.down.sql` files** — migrations without rollback are liability
2. **Test rollback locally** before committing
3. **Use the expand-contract pattern** for backward-incompatible schema changes in production
4. **Document complex migrations** with comments explaining why a change was needed
5. **Keep migrations small and focused** — one logical change per migration
6. **Never use `DROP TABLE` without `IF EXISTS`** — idempotency matters during troubleshooting
7. **Coordinate with DevOps before deploying large migrations** — they need the runbook

---

## Troubleshooting

### Migration Fails on Apply

**Symptom:** CI job fails with "migration failed" error

**Steps:**
1. Check the CI log for the exact error message
2. Identify which migration(s) failed
3. Reproduce locally: `cd backend && sqlx migrate run`
4. Fix the migration SQL and push a follow-up commit
5. The broken migration needs a new number (can't re-run the same number)

### Rollback Fails

**Symptom:** Down migration executes but schema is wrong

**Steps:**
1. Verify the `.down.sql` file drops exactly what the `.sql` file created
2. Check that `DROP` statements use `IF EXISTS` to avoid errors
3. If the rollback is incomplete, create a manual fix migration

### Out-of-Sync Migration State

**Symptom:** App starts but migration marker is misaligned with actual schema

**Steps:**
1. Compare the schema to the last successful migration number
2. Manually update the migration marker (database-specific; consult DevOps)
3. Run `sqlx migrate run` again
4. Verify the app is healthy

---

## References

- [sqlx Migration Docs](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli)
- [Prisma Migrate Docs](https://www.prisma.io/docs/concepts/components/prisma-migrate)
- [Database Seeding Guide](./database-seeding.md)
