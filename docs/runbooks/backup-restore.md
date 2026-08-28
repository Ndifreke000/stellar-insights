# Runbook: Database Backup & Restore

Covers the `BackupManager` subsystem (`backend/src/backup.rs`) — how backups are
produced, how to verify one, and how to restore after data loss.

Related issue: [#1857](https://github.com/Ndifreke000/stellar-insights/issues/1857).

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `BACKUP_ENABLED` | `false` | Master switch for the scheduler |
| `BACKUP_DB_PATH` | derived from `DATABASE_URL` | Live SQLite file to copy |
| `BACKUP_DIR` | `./backups` | Destination directory |
| `BACKUP_RETENTION_DAYS` | `30` | Age after which backups are pruned |
| `BACKUP_SCHEDULE_HOUR_UTC` | `2` | Hour (UTC) the daily backup runs |

Backups are written as `stellar_insights_<YYYYMMDD_HHMMSS>.db`, each with a
`.sha256` sidecar used for later integrity verification.

## Confirming backups are actually running

The code path existing is not evidence that backups happen. Check all three:

1. **Logs** — look for `Database backup created` at the configured hour:
   ```bash
   kubectl logs deploy/stellar-insights-backend --since=24h | grep "Database backup created"
   ```
2. **Metrics** — `backup_size_bytes` should be non-zero and refreshed daily;
   `backup_verification_failure_total` should be flat.
3. **Filesystem** — a file dated within the last 24h in `BACKUP_DIR`.

If any of the three is missing, treat it as a failed backup, not a reporting gap.

## Restore procedure

1. **Stop the backend** so nothing writes to the database mid-restore.
   ```bash
   kubectl scale deploy/stellar-insights-backend --replicas=0
   ```
2. **Pick the newest good backup.**
   ```bash
   ls -lt "$BACKUP_DIR"/stellar_insights_*.db | head
   ```
3. **Verify it before trusting it** — compare against the sidecar checksum:
   ```bash
   sha256sum -c "$BACKUP_DIR/stellar_insights_<TIMESTAMP>.db.sha256"
   ```
4. **Move the damaged database aside** (never delete it — it may still be needed
   for forensics or partial recovery).
   ```bash
   mv "$BACKUP_DB_PATH" "$BACKUP_DB_PATH.corrupt.$(date +%s)"
   ```
5. **Restore.**
   ```bash
   cp "$BACKUP_DIR/stellar_insights_<TIMESTAMP>.db" "$BACKUP_DB_PATH"
   ```
6. **Check integrity of the restored file.**
   ```bash
   sqlite3 "$BACKUP_DB_PATH" "PRAGMA integrity_check;"   # expect: ok
   ```
7. **Bring the backend back up** and confirm `/health` is green.
   ```bash
   kubectl scale deploy/stellar-insights-backend --replicas=1
   ```

## Data loss window

Backups are daily, so a restore can lose up to 24h of writes. Note the backup
timestamp in the incident record so the gap is explicit, and re-ingest from
Horizon for the affected window where possible.

## Automated coverage

`backend/tests/backup_restore_test.rs` exercises this runbook's core path:
backup → destroy live DB → restore → assert data integrity, plus checksum
verification of a corrupted backup. Run it with:

```bash
cargo test --test backup_restore_test
```
