# Migration Safety with Expand/Contract Pattern

## Overview

This document describes the migration safety system used in Stellar Insights to ensure rolling deployments work correctly. We use the **expand/contract pattern** to maintain backward compatibility during database schema changes.

## The Problem

In rolling deployments, old and new versions of the application run simultaneously. If a destructive migration (DROP COLUMN, RENAME COLUMN, etc.) runs at startup:

1. **Old version** reads schema before migration → works fine
2. **Migration runs** → schema changes, old version can't read new schema
3. **New version** reads schema after migration → works fine

This causes **intermittent failures** when old pods try to query after migration but before they're replaced.

## The Solution: Expand/Contract

### Phase 1: Expand (Backward Compatible)

Add new columns/tables without removing old ones. Both old and new code can work.

```sql
-- 061_add_new_column.sql (EXPAND PHASE)
ALTER TABLE anchors ADD COLUMN trust_score REAL DEFAULT 0;
```

### Phase 2: Contract (Destructive)

Remove old columns/tables only after all pods are upgraded.

```sql
-- 062_remove_old_column.sql (CONTRACT PHASE)
ALTER TABLE anchors DROP COLUMN legacy_score;
```

### Contract Phase Marker

Every destructive migration must include the contract phase marker:

```sql
-- contract-phase: after all pods are upgraded to v1.2.0
-- This migration removes the legacy_score column which is no longer used
-- after v1.2.0. All pods must be running v1.2.0 or later before running this.
DROP TABLE IF EXISTS legacy_metrics;
```

## Types of Migrations

### Safe (No Contract Phase Required)

These are always safe to run during rolling deployments:

- `CREATE TABLE` - New tables don't affect existing code
- `ALTER TABLE ADD COLUMN` - Adding columns is backward compatible
- `CREATE INDEX` - Indexes don't affect query results
- `INSERT`/`UPDATE` data migrations - Don't change schema

### Destructive (Contract Phase Required)

These require explicit contract phase marker:

| Operation | Example | Contract Phase Required |
|-----------|---------|------------------------|
| `DROP TABLE` | `DROP TABLE old_table` | Yes |
| `DROP COLUMN` | `ALTER TABLE DROP COLUMN col` | Yes |
| `RENAME COLUMN` | `ALTER TABLE RENAME COLUMN a TO b` | Yes |
| `DROP INDEX` | `DROP INDEX idx_name` | Yes |
| `DROP CONSTRAINT` | `ALTER TABLE DROP CONSTRAINT pk` | Yes |

## CI Linter

The linter (`scripts/lint-migrations.sh`) checks each migration:

```bash
#!/bin/bash
# Check for destructive statements without contract-phase marker

for file in migrations/*.sql; do
    if grep -qiE "^(DROP\s+(TABLE|COLUMN|INDEX|CONSTRAINT)|ALTER\s+TABLE.*DROP|RENAME\s+COLUMN)" "$file"; then
        if ! grep -q "contract-phase:" "$file"; then
            echo "ERROR: $file contains destructive statement without contract-phase marker"
            exit 1
        fi
    fi
done
```

### Running the Linter

```bash
# Check all migrations
./scripts/lint-migrations.sh

# In CI (GitHub Actions)
- name: Lint migrations
  run: ./scripts/lint-migrations.sh
```

## Migration Workflow

### Adding a New Column

**Step 1: Expand (Add column)**

```sql
-- migrations/061_add_trust_score.sql
-- contract-phase: none (expansion is always safe)

ALTER TABLE anchors ADD COLUMN trust_score REAL DEFAULT 0;
CREATE INDEX idx_anchors_trust_score ON anchors(trust_score);
```

**Step 2: Deploy old version → new version**

Old code: Works (ignores new column)  
New code: Works (reads new column)

**Step 3: Contract (Optional - remove old column)**

```sql
-- migrations/062_remove_legacy_trust_score.sql
-- contract-phase: after all pods are upgraded to v1.2.0
-- Legacy trust score column is replaced by trust_score in v1.2.0

ALTER TABLE anchors DROP COLUMN legacy_trust_score;
```

## CI Testing Strategy

### Test Old Code Against New Schema

```yaml
# .github/workflows/migration-tests.yml
name: Migration Tests

on:
  pull_request:
    paths:
      - 'backend/migrations/**'

jobs:
  test-old-code-new-schema:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup database with latest schema
        run: |
          # Apply all migrations
          sqlx migrate run
          
      - name: Run tests with old app version
        run: |
          # Build old version from last release
          cargo build --release --bin payraider-backend
          
          # Run against new schema
          ./test-old-schema-compatibility.sh
```

### Test Plan

1. **Schema Compatibility**: Old app version can read new schema
2. **Rolling Deployment**: Simulate old/new pods running simultaneously
3. **Data Migration**: Verify data is correctly migrated
4. **Backward Compatibility**: New code handles missing columns gracefully

## Configuration

### Environment Variables for Migrations

| Variable | Default | Description |
|----------|---------|-------------|
| `MIGRATE_ON_STARTUP` | `true` | Run migrations at app startup |
| `MIGRATION_CONTRACT_PHASE` | `none` | Expected contract phase (`none`, `expand`, `contract`) |
| `MIGRATION_TIMEOUT_SECONDS` | `300` | Timeout for migration execution |

### Example: Running Without Auto-Migration

```bash
# Disable auto-migration
export MIGRATE_ON_STARTUP=false

# Run migrations separately
cargo run --bin migrate

# Then start the app
cargo run
```

## GitHub Actions Integration

### Lint on Pull Request

```yaml
# .github/workflows/migration-safety.yml
name: Migration Safety

on:
  pull_request:
    paths:
      - 'backend/migrations/**'

jobs:
  lint-migrations:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Lint migrations
        run: |
          chmod +x scripts/lint-migrations.sh
          ./scripts/lint-migrations.sh
```

### Test Old Code Against New Schema

```yaml
# .github/workflows/migration-tests.yml
name: Migration Tests

on:
  pull_request:
    paths:
      - 'backend/migrations/**'

jobs:
  test-schema-compatibility:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Apply migrations
        run: |
          cd backend
          sqlx migrate run -d sqlite::memory:
          
      - name: Test old version
        run: |
          # Build and test old version
          cargo test --test schema_compatibility
```

## Migration Naming Convention

```
001_create_anchors.sql              # Initial create (no contract phase)
002_add_trust_score.sql             # Add column (no contract phase)
061_add_new_column.sql              # Expand phase - add new column
062_remove_old_column.sql           # Contract phase - remove old column
063_rename_column.sql               # Contract phase - requires marker
```

## Common Patterns

### Pattern 1: Rename Column

**Step 1: Add new column (Expand)**

```sql
-- 061_rename_column_expand.sql
-- contract-phase: none

ALTER TABLE anchors ADD COLUMN trust_score_new REAL DEFAULT 0;
UPDATE anchors SET trust_score_new = trust_score;
```

**Step 2: Update application code to use new column**

**Step 3: Remove old column (Contract)**

```sql
-- 062_rename_column_contract.sql
-- contract-phase: after all pods use trust_score_new

ALTER TABLE anchors DROP COLUMN trust_score;
ALTER TABLE anchors RENAME COLUMN trust_score_new TO trust_score;
```

### Pattern 2: Change Column Type

**Step 1: Add new column with new type (Expand)**

```sql
-- 061_change_type_expand.sql
-- contract-phase: none

ALTER TABLE anchors ADD COLUMN score_numeric INTEGER DEFAULT 0;
UPDATE anchors SET score_numeric = CAST(trust_score AS INTEGER);
```

**Step 2: Update application code**

**Step 3: Replace old column (Contract)**

```sql
-- 062_change_type_contract.sql
-- contract-phase: after all pods use score_numeric

ALTER TABLE anchors DROP COLUMN trust_score;
ALTER TABLE anchors RENAME COLUMN score_numeric TO trust_score;
```

### Pattern 3: Split Table

**Step 1: Create new table (Expand)**

```sql
-- 061_create_audit_log_expand.sql
-- contract-phase: none

CREATE TABLE audit_log_v2 (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    event_type TEXT NOT NULL,
    details TEXT
);
```

**Step 2: Update code to write to both tables**

**Step 3: Migrate and drop old table (Contract)**

```sql
-- 062_create_audit_log_contract.sql
-- contract-phase: after all pods use audit_log_v2

-- Migrate data
INSERT INTO audit_log_v2 SELECT * FROM audit_log;

-- Drop old table
DROP TABLE audit_log;
```

## Verification Checklist

Before merging a migration PR:

- [ ] All destructive migrations have `contract-phase:` marker
- [ ] Expansion phase migrations don't require marker
- [ ] CI lint passes (`./scripts/lint-migrations.sh`)
- [ ] New schema tested with old app version
- [ ] Rolling deployment tested in staging
- [ ] Data migration tested and verified
- [ ] Rollback procedure documented

## Troubleshooting

### Migration Fails on Old Version

**Symptom**: Old pod crashes after migration with "table not found"

**Solution**: Ensure expand phase is deployed before contract phase

### Linter Reports Missing Marker

**Error**: `contains destructive statement without contract-phase marker`

**Solution**: Add `-- contract-phase: <description>` to migration header

### Downtime During Deploy

**Symptom**: Brief service interruption during deployment

**Solution**: 
1. Verify rolling deployment is enabled
2. Check `maxSurge` and `maxUnavailable` settings
3. Consider using Blue-Green deployment for destructive migrations

## Related Documentation

- [Database Schema](DATABASE_SCHEMA.md)
- [Rolling Deployments](ROLLING_DEPLOYMENTS.md)
- [SQL Style Guide](SQL_STYLE_GUIDE.md)
