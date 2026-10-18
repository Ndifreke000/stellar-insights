# Database Seeding Guide

## Overview

The seeding system provides realistic development data for all major entities in the database. It is designed to be **idempotent** — running it multiple times will not create duplicate records.

## Files

| File                                                 | Purpose                             |
| ---------------------------------------------------- | ----------------------------------- |
| `backend/seed_data.sh`                               | Main seeding script (direct SQLite) |
| `backend/migrations/030_comprehensive_seed_data.sql` | SQL seed data for all entities      |

---

## Quick Start

```bash
# Run migrations first (if not already done)
sqlx migrate run

# Seed the database
./seed_data.sh .payraider.db

# Reset and re-seed
./seed_data.sh .payraider.db --reset
```

---

## What Gets Seeded

### Anchors (7 records)

| Name                           | Status    | Reliability | Volume (USD) |
| ------------------------------ | --------- | ----------- | ------------ |
| Circle                         | 🟢 green  | 99.3%       | $15,500,000  |
| Stellar Development Foundation | 🟢 green  | 99.5%       | $50,000,000  |
| MoneyGram Access               | 🟢 green  | 98.9%       | $9,800,000   |
| AnchorUSD                      | 🟡 yellow | 95.0%       | $7,500,000   |
| Vibrant                        | 🟡 yellow | 94.8%       | $4,200,000   |
| TestNet Anchor                 | 🔴 red    | 80.0%       | $800,000     |
| Legacy Bridge                  | 🔴 red    | 80.0%       | $1,200,000   |

### Assets (9 records)

| Code  | Anchor    | Supply         | Holders   |
| ----- | --------- | -------------- | --------- |
| USDC  | Circle    | 5,000,000,000  | 250,000   |
| EURC  | Circle    | 2,000,000,000  | 80,000    |
| GBPD  | Circle    | 1,500,000,000  | 60,000    |
| MGI   | MoneyGram | 800,000,000    | 120,000   |
| MGUSD | MoneyGram | 600,000,000    | 90,000    |
| XLM   | Stellar   | 50,000,000,000 | 5,000,000 |
| AUSD  | AnchorUSD | 300,000,000    | 50,000    |
| VELO  | Vibrant   | 1,000,000,000  | 150,000   |
| TEST  | TestNet   | 100,000,000    | 5,000     |

### Corridors (8 records)

| Source      | Destination           | Reliability | Status   |
| ----------- | --------------------- | ----------- | -------- |
| USDC → EURC | Circle → Circle       | 99.2%       | active   |
| USDC → GBPD | Circle → Circle       | 98.8%       | active   |
| USDC → MGI  | Circle → MoneyGram    | 97.5%       | active   |
| XLM → USDC  | Stellar → Circle      | 99.5%       | active   |
| XLM → MGI   | Stellar → MoneyGram   | 98.2%       | active   |
| MGI → MGUSD | MoneyGram → MoneyGram | 96.5%       | active   |
| VELO → USDC | Vibrant → Circle      | 94.2%       | active   |
| TEST → USDC | TestNet → Circle      | 75.0%       | degraded |

### Metrics History (14 records)

Time-series performance data for each anchor over the last 30 days.

### Users (4 records)

| Username  | Role                 | Password   |
| --------- | -------------------- | ---------- |
| admin     | Administrator        | `password` |
| analyst   | Read-only analyst    | `password` |
| developer | Read/write developer | `password` |
| viewer    | Read-only viewer     | `password` |

> ⚠️ These are development-only credentials. Never use in production.

### API Keys (5 records)

| Name            | Scopes             | Status           |
| --------------- | ------------------ | ---------------- |
| Admin Key       | read, write, admin | active           |
| Analyst Key     | read               | active           |
| Development Key | read, write        | active           |
| Expired Key     | read               | active (expired) |
| Revoked Key     | read, write        | revoked          |

### Governance (4 proposals, 11 votes, 5 comments)

| Proposal                | Status   |
| ----------------------- | -------- |
| Upgrade Core Contract   | active   |
| Increase Fee Threshold  | active   |
| Add New Anchor          | passed   |
| Deprecate Legacy Bridge | rejected |

---

## Idempotency

All inserts use `INSERT OR IGNORE` so re-running the script is safe. Existing records are never overwritten.

To force a full reset:

```bash
./seed_data.sh .payraider.db --reset
```

This deletes all seed records (identified by their prefixed IDs) and re-inserts them.

---

## Verification Queries

```bash
# Count all seeded anchors
sqlite3 .payraider.db "SELECT COUNT(*) FROM anchors WHERE id LIKE 'anchor-%';"

# View anchors by status
sqlite3 .payraider.db "SELECT name, status, reliability_score FROM anchors WHERE id LIKE 'anchor-%' ORDER BY reliability_score DESC;"

# View corridors
sqlite3 .payraider.db "SELECT source_asset_code, destination_asset_code, reliability_score, status FROM corridors WHERE id LIKE 'corridor-%';"

# View recent metrics history
sqlite3 .payraider.db "SELECT anchor_id, timestamp, success_rate FROM anchor_metrics_history WHERE anchor_id LIKE 'anchor-%' ORDER BY timestamp DESC LIMIT 10;"

# View governance proposals
sqlite3 .payraider.db "SELECT title, status FROM governance_proposals WHERE created_by LIKE 'GPROPOSER%';"
```

---

## CI Integration

To run seeding in CI, add this step after migrations:

```yaml
- name: Seed database
  run: ./backend/seed_data.sh ${{ env.DB_PATH }}
```

---

## Adding New Seed Data

1. Add `INSERT OR IGNORE` statements to `backend/migrations/030_comprehensive_seed_data.sql`
2. Use a consistent ID prefix (e.g. `anchor-`, `corridor-`, `asset-`) so the `--reset` flag can clean them up
3. Update this document with the new records
4. Test idempotency by running the script twice
