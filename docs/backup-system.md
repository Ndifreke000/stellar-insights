# Backup System

This document describes the database backup system for Stellar Insights and provides
step-by-step restore procedures referenced by the backup-verification CI workflow.

---

## Overview

Stellar Insights uses a scheduled SQLite backup system implemented in
`backend/src/backup.rs`. Every night at a configurable UTC hour the backend copies
the live database file to a local backup directory and prunes backups older than the
configured retention window.

```
backend/
├── src/backup.rs              # BackupManager implementation
└── scripts/backup/
    ├── full_backup.sh         # wal-g full-backup helper (PostgreSQL variant)
    └── incremental_backup.sh  # wal-g incremental helper (PostgreSQL variant)
```

---

## Configuration

Set the following environment variables (see `backend/.env.example`):

| Variable | Default | Description |
|---|---|---|
| `BACKUP_ENABLED` | `false` | Set to `true` to activate the scheduler |
| `BACKUP_DB_PATH` | derived from `DATABASE_URL` | Path to the SQLite database file |
| `BACKUP_DIR` | `./backups` | Directory where backup files are written |
| `BACKUP_RETENTION_DAYS` | `30` | Delete backups older than this many days |
| `BACKUP_SCHEDULE_HOUR_UTC` | `2` | UTC hour at which the nightly backup runs |

Example `.env` snippet:

```bash
BACKUP_ENABLED=true
BACKUP_DB_PATH=./stellar_insights.db
BACKUP_DIR=./backups
BACKUP_RETENTION_DAYS=30
BACKUP_SCHEDULE_HOUR_UTC=2
```

---

## How Backups Are Created

1. The `BackupManager::spawn_scheduler` task wakes up at the next occurrence of
   `BACKUP_SCHEDULE_HOUR_UTC`.
2. It calls `create_backup`, which copies the live database file to
   `<BACKUP_DIR>/stellar_insights_<YYYYMMDD_HHMMSS>.db`.
3. It calls `cleanup_old_backups`, which removes any file in `BACKUP_DIR` whose
   modification time is older than `BACKUP_RETENTION_DAYS` days.

---

## Restore Procedures

### 1. Identify the target backup

List available backups, sorted newest-first:

```bash
ls -lt ./backups/stellar_insights_*.db
```

Choose the most recent file before the incident (or `latest` if you want the newest):

```bash
TARGET=./backups/stellar_insights_<YYYYMMDD_HHMMSS>.db
```

### 2. Stop the backend

```bash
# Docker-based deployment
docker compose stop backend

# Systemd-based deployment
sudo systemctl stop stellar-insights-backend
```

### 3. Replace the live database

```bash
# Back up the current (potentially corrupt) database first
cp ./stellar_insights.db ./stellar_insights.db.broken

# Restore from backup
cp "$TARGET" ./stellar_insights.db
```

### 4. Verify the restored database

```bash
sqlite3 ./stellar_insights.db "PRAGMA integrity_check;"
# Expected output: ok

sqlite3 ./stellar_insights.db "SELECT COUNT(*) FROM payments;"
```

If `PRAGMA integrity_check` returns anything other than `ok`, try the next-oldest
backup and repeat.

### 5. Restart the backend

```bash
# Docker-based deployment
docker compose start backend

# Systemd-based deployment
sudo systemctl start stellar-insights-backend
```

### 6. Confirm the service is healthy

```bash
curl -s http://localhost:8080/api/anchors | jq '.data | length'
```

---

## Restore Verification (CI)

The backup-verification workflow (`.github/workflows/`) performs an automated
restore test on every scheduled backup:

1. Downloads the latest backup artifact from the workflow run.
2. Runs `sqlite3 <backup_file> "PRAGMA integrity_check;"`.
3. Runs a row-count sanity check against key tables (`payments`, `corridors`,
   `anchors`).
4. Fails the job and posts a GitHub issue if any check does not pass.

If the CI job fails with **"Restore verification of the production Litestream
replica failed"**, follow the manual restore procedure above and then re-run the
workflow to confirm the fix.

---

## Retention Policy

Backups older than `BACKUP_RETENTION_DAYS` days are automatically deleted by
`BackupManager::cleanup_old_backups`. To manually remove all backups older than
7 days:

```bash
find ./backups -name "stellar_insights_*.db" -mtime +7 -delete
```

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| No backup files in `BACKUP_DIR` | `BACKUP_ENABLED=false` or wrong `BACKUP_DIR` | Check environment variables |
| Backup scheduler never fires | Wrong `BACKUP_SCHEDULE_HOUR_UTC` | Set the correct UTC hour and restart |
| `PRAGMA integrity_check` fails | Incomplete copy (in-flight write during backup) | Restore next-oldest backup; consider enabling WAL mode |
| CI reports "replica failed" | Missing or corrupt backup artifact | Follow the manual restore procedure above |

---

## Related Files

- `backend/src/backup.rs` — `BackupConfig`, `BackupManager`
- `backend/.env.example` — All configurable variables with defaults
- `backend/scripts/backup/full_backup.sh` — wal-g full-backup script
- `backend/scripts/backup/incremental_backup.sh` — wal-g incremental script
