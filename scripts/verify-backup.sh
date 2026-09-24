#!/usr/bin/env bash
#
# Verify that a database backup can actually be restored and is internally consistent.
#
# Usage:
#   ./scripts/verify-backup.sh latest                      # latest Litestream replica state
#   ./scripts/verify-backup.sh 2026-09-01T12:00:00Z        # point-in-time recovery
#   ./scripts/verify-backup.sh ./backups/payraider_X.db    # a local backup.rs snapshot
#
# Environment:
#   LITESTREAM_REPLICA_URL  Replica to restore from, e.g.
#                           s3://payraider-db-backups-<account>/<environment>/payraider.db
#                           (required unless verifying a local file)
#   BACKUP_MAX_AGE_MINUTES  Fail if the replica's newest WAL is older than this (default 60)
#   BACKUP_VERIFY_TABLES    Space-separated tables that must exist and be non-empty
#                           (default: "anchors corridors")
#   BACKUP_REPORT_DIR       Where to write the JSON report (default ./backup-verification)
#   BACKUP_METRICS_FILE     Optional node_exporter textfile-collector output (.prom)
#   BACKUP_ALERT_WEBHOOK_URL Optional Slack-compatible webhook, notified on failure
#   KEEP_RESTORED_DB=true   Keep the restored file (path is printed) instead of deleting it
#
# Checks: restore succeeds, replica freshness, PRAGMA integrity_check,
# PRAGMA foreign_key_check, applied sqlx migrations, required tables non-empty.
# Exits non-zero if any check fails. See docs/backup-system.md.

set -uo pipefail

TARGET="${1:-latest}"
MAX_AGE_MINUTES="${BACKUP_MAX_AGE_MINUTES:-60}"
VERIFY_TABLES="${BACKUP_VERIFY_TABLES:-anchors corridors}"
REPORT_DIR="${BACKUP_REPORT_DIR:-./backup-verification}"
WORK_DIR="$(mktemp -d)"
RESTORED_DB="$WORK_DIR/restored.db"
START_EPOCH=$(date +%s)
FAILURES=()
SOURCE=""
RESTORED_THROUGH=""

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"; }
fail() { log "CHECK FAILED: $*"; FAILURES+=("$*"); }

cleanup() {
  if [ "${KEEP_RESTORED_DB:-false}" = "true" ] && [ -f "$RESTORED_DB" ]; then
    log "Restored database kept at $RESTORED_DB"
  else
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

for cmd in sqlite3 jq sha256sum; do
  command -v "$cmd" >/dev/null 2>&1 || { log "ERROR: $cmd is required"; exit 2; }
done

write_metrics() {
  local success="$1" duration="$2"
  [ -n "${BACKUP_METRICS_FILE:-}" ] || return 0
  mkdir -p "$(dirname "$BACKUP_METRICS_FILE")"
  {
    echo "# HELP backup_restore_verification_success 1 if the last restore verification passed"
    echo "# TYPE backup_restore_verification_success gauge"
    echo "backup_restore_verification_success $success"
    echo "# HELP backup_restore_verification_duration_seconds Duration of the last restore verification"
    echo "# TYPE backup_restore_verification_duration_seconds gauge"
    echo "backup_restore_verification_duration_seconds $duration"
    if [ "$success" = "1" ]; then
      echo "# HELP backup_restore_verification_last_success_timestamp_seconds Unix time of the last passing restore verification"
      echo "# TYPE backup_restore_verification_last_success_timestamp_seconds gauge"
      echo "backup_restore_verification_last_success_timestamp_seconds $(date +%s)"
    fi
  } > "$BACKUP_METRICS_FILE.tmp" && mv "$BACKUP_METRICS_FILE.tmp" "$BACKUP_METRICS_FILE"
}

notify_failure() {
  [ -n "${BACKUP_ALERT_WEBHOOK_URL:-}" ] || return 0
  command -v curl >/dev/null 2>&1 || return 0
  local text
  text=$(printf ':rotating_light: *Backup verification FAILED* (%s, target %s)\n%s' \
    "$SOURCE" "$TARGET" "$(printf ' - %s\n' "${FAILURES[@]}")")
  curl -fsS -m 10 -H 'Content-Type: application/json' \
    -d "$(jq -n --arg text "$text" '{text: $text}')" \
    "$BACKUP_ALERT_WEBHOOK_URL" >/dev/null || log "WARN: failed to send webhook alert"
}

finish() {
  local duration=$(( $(date +%s) - START_EPOCH ))
  local status="success"
  [ ${#FAILURES[@]} -eq 0 ] || status="failure"

  local checksum="" size=0
  if [ -f "$RESTORED_DB" ]; then
    checksum=$(sha256sum "$RESTORED_DB" | cut -d' ' -f1)
    size=$(stat -c %s "$RESTORED_DB")
  fi

  mkdir -p "$REPORT_DIR"
  local report="$REPORT_DIR/verify-$(date -u +%Y%m%dT%H%M%SZ).json"
  jq -n \
    --arg status "$status" \
    --arg source "$SOURCE" \
    --arg target "$TARGET" \
    --arg restored_through "$RESTORED_THROUGH" \
    --arg sha256 "$checksum" \
    --argjson size "$size" \
    --argjson duration "$duration" \
    --argjson failures "$(printf '%s\n' "${FAILURES[@]}" | jq -R . | jq -s 'map(select(length > 0))')" \
    '{status: $status, source: $source, target: $target, restored_through: $restored_through,
      sha256: $sha256, size_bytes: $size, verified_at: (now | todate),
      duration_seconds: $duration, failures: $failures}' > "$report"
  cp "$report" "$REPORT_DIR/latest.json"
  log "Report written to $report"

  if [ "$status" = "success" ]; then
    write_metrics 1 "$duration"
    log "Backup verification PASSED in ${duration}s"
    exit 0
  fi
  write_metrics 0 "$duration"
  notify_failure
  log "Backup verification FAILED"
  exit 1
}

# 1. Restore
if [ -f "$TARGET" ]; then
  SOURCE="file:$TARGET"
  log "Verifying local snapshot $TARGET"
  cp "$TARGET" "$RESTORED_DB"
else
  command -v litestream >/dev/null 2>&1 || { log "ERROR: litestream is required"; exit 2; }
  [ -n "${LITESTREAM_REPLICA_URL:-}" ] || { log "ERROR: LITESTREAM_REPLICA_URL is not set"; exit 2; }
  SOURCE="$LITESTREAM_REPLICA_URL"

  # Freshness: the newest WAL end time across generations (only for the latest state).
  if [ "$TARGET" = "latest" ]; then
    LAST_WAL=$(litestream generations "$LITESTREAM_REPLICA_URL" 2>/dev/null \
      | awk 'NR > 1 { print $NF }' | sort | tail -n 1)
    if [ -z "$LAST_WAL" ]; then
      fail "No Litestream generations found at $LITESTREAM_REPLICA_URL"
      finish
    fi
    AGE_MINUTES=$(( ( $(date +%s) - $(date -u -d "$LAST_WAL" +%s) ) / 60 ))
    RESTORED_THROUGH="$LAST_WAL"
    log "Replica last updated $LAST_WAL (${AGE_MINUTES}m ago)"
    if [ "$AGE_MINUTES" -gt "$MAX_AGE_MINUTES" ]; then
      fail "Replica is stale: last WAL ${AGE_MINUTES}m ago (max ${MAX_AGE_MINUTES}m)"
    fi
    log "Restoring latest state from $LITESTREAM_REPLICA_URL"
    litestream restore -o "$RESTORED_DB" "$LITESTREAM_REPLICA_URL" \
      || { fail "litestream restore failed"; finish; }
  else
    RESTORED_THROUGH="$TARGET"
    log "Restoring point in time $TARGET from $LITESTREAM_REPLICA_URL"
    litestream restore -o "$RESTORED_DB" -timestamp "$TARGET" "$LITESTREAM_REPLICA_URL" \
      || { fail "litestream point-in-time restore to $TARGET failed"; finish; }
  fi
fi

[ -s "$RESTORED_DB" ] || { fail "Restored database is missing or empty"; finish; }

# 2. Page-level / structural integrity
INTEGRITY=$(sqlite3 -readonly "$RESTORED_DB" "PRAGMA integrity_check;" 2>&1)
if [ "$INTEGRITY" = "ok" ]; then
  log "integrity_check: ok"
else
  fail "integrity_check: $(echo "$INTEGRITY" | head -n 5 | tr '\n' ' ')"
fi

# 3. Referential integrity
FK_VIOLATIONS=$(sqlite3 -readonly "$RESTORED_DB" "PRAGMA foreign_key_check;" 2>&1 | wc -l)
if [ "$FK_VIOLATIONS" -eq 0 ]; then
  log "foreign_key_check: ok"
else
  fail "foreign_key_check: $FK_VIOLATIONS violating rows"
fi

# 4. Schema: migrations applied and none failed
MIGRATIONS=$(sqlite3 -readonly "$RESTORED_DB" "SELECT count(*) FROM _sqlx_migrations WHERE success = 1;" 2>/dev/null)
FAILED_MIGRATIONS=$(sqlite3 -readonly "$RESTORED_DB" "SELECT count(*) FROM _sqlx_migrations WHERE success = 0;" 2>/dev/null)
if [ -z "$MIGRATIONS" ] || [ "$MIGRATIONS" -eq 0 ]; then
  fail "No applied migrations found in _sqlx_migrations"
elif [ "${FAILED_MIGRATIONS:-0}" -gt 0 ]; then
  fail "$FAILED_MIGRATIONS failed migrations recorded in _sqlx_migrations"
else
  log "migrations: $MIGRATIONS applied"
fi

# 5. Required data present
for table in $VERIFY_TABLES; do
  ROWS=$(sqlite3 -readonly "$RESTORED_DB" "SELECT count(*) FROM \"$table\";" 2>/dev/null)
  if [ -z "$ROWS" ]; then
    fail "Table $table is missing or unreadable"
  elif [ "$ROWS" -eq 0 ]; then
    fail "Table $table is empty"
  else
    log "$table: $ROWS rows"
  fi
done

finish
