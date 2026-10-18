#!/usr/bin/env bash
#
# Restore a database backup and verify disaster recovery compliance (RTO / RPO).
#
# Usage:
#   ./scripts/restore-backup.sh [TARGET] [DEST_PATH] [OPTIONS]
#
# Arguments:
#   TARGET      Backup target to restore: 'latest' (default), an ISO-8601 timestamp,
#               or path to a local SQLite database backup file.
#   DEST_PATH   Destination path for the restored database. If omitted, restores
#               to a verified sandbox path and validates integrity without overwriting live DB.
#
# Options:
#   --force     Overwrite destination database file if it already exists.
#   --dry-run   Perform restoration and integrity checks in an isolated temp directory.
#   -h, --help  Show this help message.
#
# Environment:
#   LITESTREAM_REPLICA_URL   S3 / cloud replica URL (e.g. s3://bucket/path/payraider.db)
#   BACKUP_MAX_AGE_MINUTES   RPO target threshold in minutes (default: 60 = 1 hour)
#   RTO_MAX_SECONDS          RTO target threshold in seconds (default: 14400 = 4 hours)
#   BACKUP_VERIFY_TABLES     Space-separated table names to verify (default: "anchors corridors")
#   REPORT_DIR               Directory to write disaster recovery reports (default: ./backup-verification)
#
# Recovery Objectives:
#   - RTO (Recovery Time Objective): 4 hours
#   - RPO (Recovery Point Objective): 1 hour
#

set -euo pipefail

TARGET="latest"
DEST_PATH=""
DRY_RUN=false
FORCE=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      sed -n '2,24p' "$0"
      exit 0
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --force)
      FORCE=true
      shift
      ;;
    *)
      if [ -z "$TARGET" ] || [ "$TARGET" = "latest" ]; then
        TARGET="$1"
      elif [ -z "$DEST_PATH" ]; then
        DEST_PATH="$1"
      fi
      shift
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
REPORT_DIR="${REPORT_DIR:-$PROJECT_ROOT/backup-verification}"
RTO_MAX_SECONDS="${RTO_MAX_SECONDS:-14400}" # 4 hours
RPO_MAX_MINUTES="${BACKUP_MAX_AGE_MINUTES:-60}" # 1 hour
VERIFY_TABLES="${BACKUP_VERIFY_TABLES:-anchors corridors}"

START_EPOCH=$(date +%s)
FAILURES=()
SOURCE=""
DATA_AGE_MINUTES=0

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"; }
warn() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] [WARN] $*" >&2; }
fail() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] [FAIL] $*" >&2; FAILURES+=("$*"); }

run_sql() {
  local db_file="$1"
  local sql_query="$2"
  if command -v sqlite3 >/dev/null 2>&1; then
    sqlite3 -readonly "$db_file" "$sql_query" 2>&1
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c "import sqlite3, sys
try:
    conn = sqlite3.connect(sys.argv[1])
    cur = conn.cursor()
    cur.execute(sys.argv[2])
    for row in cur.fetchall():
        print(row[0] if len(row) == 1 else '\t'.join(str(c) for c in row))
except Exception as e:
    print(f'ERROR: {e}', file=sys.stderr)
" "$db_file" "$sql_query" 2>&1
  else
    echo "ERROR: neither sqlite3 nor python3 is installed" >&2
    return 1
  fi
}

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required" >&2; exit 2; }

WORK_DIR="$(mktemp -d)"
TEMP_RESTORE_DB="$WORK_DIR/payraider_restored.db"

cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

log "============================================================"
log " PayRaider Disaster Recovery Restoration & Verification"
log "============================================================"
log "Target:                 $TARGET"
log "RTO Target:             $((RTO_MAX_SECONDS / 3600)) hours ($RTO_MAX_SECONDS seconds)"
log "RPO Target:             $RPO_MAX_MINUTES minutes ($((RPO_MAX_MINUTES * 60)) seconds)"
log "Dry run:                $DRY_RUN"

# 1. Resolve source and restore database
log "--- Phase 1: Restoring Database ---"

if [ -f "$TARGET" ]; then
  SOURCE="file:$TARGET"
  log "Source identified as local file: $TARGET"
  cp "$TARGET" "$TEMP_RESTORE_DB"
  FILE_MTIME=$(stat -c %Y "$TARGET" 2>/dev/null || stat -f %m "$TARGET" 2>/dev/null || echo "$START_EPOCH")
  DATA_AGE_MINUTES=$(( (START_EPOCH - FILE_MTIME) / 60 ))
elif [ "$TARGET" = "latest" ] && [ -z "${LITESTREAM_REPLICA_URL:-}" ]; then
  # Local fallback if Litestream is not configured
  LOCAL_BACKUP=""
  if compgen -G "$PROJECT_ROOT/backups/*.db" > /dev/null; then
    LOCAL_BACKUP=$(ls -t "$PROJECT_ROOT/backups/"*.db | head -n 1)
  elif [ -f "$PROJECT_ROOT/backend/payraider.db" ]; then
    LOCAL_BACKUP="$PROJECT_ROOT/backend/payraider.db"
  elif [ -f "$PROJECT_ROOT/payraider.db" ]; then
    LOCAL_BACKUP="$PROJECT_ROOT/payraider.db"
  fi

  if [ -n "$LOCAL_BACKUP" ] && [ -f "$LOCAL_BACKUP" ]; then
    SOURCE="local-backup:$LOCAL_BACKUP"
    log "No LITESTREAM_REPLICA_URL set; utilizing latest local snapshot: $LOCAL_BACKUP"
    cp "$LOCAL_BACKUP" "$TEMP_RESTORE_DB"
    FILE_MTIME=$(stat -c %Y "$LOCAL_BACKUP" 2>/dev/null || stat -f %m "$LOCAL_BACKUP" 2>/dev/null || echo "$START_EPOCH")
    DATA_AGE_MINUTES=$(( (START_EPOCH - FILE_MTIME) / 60 ))
  else
    fail "No Litestream replica configured and no local database found to restore from"
  fi
else
  # Restore via Litestream
  if ! command -v litestream >/dev/null 2>&1; then
    fail "litestream command not found in PATH"
  elif [ -z "${LITESTREAM_REPLICA_URL:-}" ]; then
    fail "LITESTREAM_REPLICA_URL is required when restoring from cloud replica"
  else
    SOURCE="$LITESTREAM_REPLICA_URL"
    if [ "$TARGET" = "latest" ]; then
      log "Checking Litestream generations at $LITESTREAM_REPLICA_URL"
      LAST_WAL=$(litestream generations "$LITESTREAM_REPLICA_URL" 2>/dev/null | awk 'NR > 1 { print $NF }' | sort | tail -n 1 || true)
      if [ -n "$LAST_WAL" ]; then
        WAL_EPOCH=$(date -u -d "$LAST_WAL" +%s 2>/dev/null || echo "$START_EPOCH")
        DATA_AGE_MINUTES=$(( (START_EPOCH - WAL_EPOCH) / 60 ))
        log "Latest replica WAL timestamp: $LAST_WAL (${DATA_AGE_MINUTES}m ago)"
      fi
      log "Restoring latest state from $LITESTREAM_REPLICA_URL..."
      if ! litestream restore -o "$TEMP_RESTORE_DB" "$LITESTREAM_REPLICA_URL"; then
        fail "litestream restore failed"
      fi
    else
      log "Restoring point-in-time ($TARGET) from $LITESTREAM_REPLICA_URL..."
      if ! litestream restore -o "$TEMP_RESTORE_DB" -timestamp "$TARGET" "$LITESTREAM_REPLICA_URL"; then
        fail "litestream point-in-time restore failed"
      fi
    fi
  fi
fi

# 2. Check restored file exists and non-empty
if [ ! -s "$TEMP_RESTORE_DB" ]; then
  fail "Restored database file is empty or missing"
fi

# 3. Phase 2: Integrity & Verification
log "--- Phase 2: Verifying Data Integrity ---"

if [ ${#FAILURES[@]} -eq 0 ]; then
  # Structural integrity
  INTEGRITY=$(run_sql "$TEMP_RESTORE_DB" "PRAGMA integrity_check;")
  if [ "$INTEGRITY" = "ok" ]; then
    log "PRAGMA integrity_check: OK"
  else
    fail "PRAGMA integrity_check failed: $INTEGRITY"
  fi

  # Referential integrity
  FK_OUTPUT=$(run_sql "$TEMP_RESTORE_DB" "PRAGMA foreign_key_check;")
  FK_ERRS=0
  if [ -n "$FK_OUTPUT" ]; then
    FK_ERRS=$(echo "$FK_OUTPUT" | wc -l)
  fi
  if [ "$FK_ERRS" -eq 0 ]; then
    log "PRAGMA foreign_key_check: OK (0 violations)"
  else
    fail "PRAGMA foreign_key_check: $FK_ERRS violations"
  fi

  # Migration verification
  HAS_MIG=$(run_sql "$TEMP_RESTORE_DB" "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations';" || echo "0")
  if [ "$HAS_MIG" = "1" ]; then
    MIGRATIONS_COUNT=$(run_sql "$TEMP_RESTORE_DB" "SELECT count(*) FROM _sqlx_migrations WHERE success = 1;" || echo "0")
  else
    MIGRATIONS_COUNT="N/A (standard schema)"
  fi
  log "Applied migrations found: $MIGRATIONS_COUNT"

  # Table existence and row checks
  for table in $VERIFY_TABLES; do
    EXISTS=$(run_sql "$TEMP_RESTORE_DB" "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='$table';" || echo "0")
    if [ "$EXISTS" -gt 0 ]; then
      ROWS=$(run_sql "$TEMP_RESTORE_DB" "SELECT count(*) FROM \"$table\";" || echo "0")
      log "Table '$table' verified: $ROWS rows"
    else
      warn "Table '$table' does not exist in restored schema"
    fi
  done
fi

# 4. Phase 3: Copy to Destination if requested and not dry run
RESTORED_DESTINATION=""
if [ -n "$DEST_PATH" ] && [ "$DRY_RUN" = "false" ] && [ ${#FAILURES[@]} -eq 0 ]; then
  if [ -f "$DEST_PATH" ] && [ "$FORCE" != "true" ]; then
    fail "Destination file '$DEST_PATH' already exists. Use --force to overwrite."
  else
    mkdir -p "$(dirname "$DEST_PATH")"
    cp "$TEMP_RESTORE_DB" "$DEST_PATH"
    RESTORED_DESTINATION="$DEST_PATH"
    log "Restored database written to $DEST_PATH"
  fi
fi

# 5. Phase 4: Metrics, RTO / RPO Evaluation
END_EPOCH=$(date +%s)
DURATION_SECONDS=$((END_EPOCH - START_EPOCH))

log "--- Phase 3: Disaster Recovery Evaluation ---"
log "Restoration duration:    ${DURATION_SECONDS}s"
if [ "$DURATION_SECONDS" -le "$RTO_MAX_SECONDS" ]; then
  log "RTO Evaluation:          PASSED (${DURATION_SECONDS}s <= ${RTO_MAX_SECONDS}s)"
  RTO_STATUS="PASSED"
else
  fail "RTO threshold exceeded: ${DURATION_SECONDS}s > ${RTO_MAX_SECONDS}s"
  RTO_STATUS="FAILED"
fi

if [ "$DATA_AGE_MINUTES" -le "$RPO_MAX_MINUTES" ]; then
  log "RPO Evaluation:          PASSED (${DATA_AGE_MINUTES}m <= ${RPO_MAX_MINUTES}m)"
  RPO_STATUS="PASSED"
else
  warn "RPO threshold exceeded: data age ${DATA_AGE_MINUTES}m > ${RPO_MAX_MINUTES}m"
  RPO_STATUS="EXCEEDED"
fi

STATUS="SUCCESS"
if [ ${#FAILURES[@]} -gt 0 ]; then
  STATUS="FAILURE"
fi

# Generate JSON Report
mkdir -p "$REPORT_DIR"
REPORT_FILE="$REPORT_DIR/restore-$(date -u +%Y%m%dT%H%M%SZ).json"
CHECKSUM=""
FILE_SIZE=0
if [ -s "$TEMP_RESTORE_DB" ]; then
  CHECKSUM=$(sha256sum "$TEMP_RESTORE_DB" | cut -d' ' -f1)
  FILE_SIZE=$(stat -c %s "$TEMP_RESTORE_DB" 2>/dev/null || stat -f %z "$TEMP_RESTORE_DB" 2>/dev/null || echo 0)
fi

jq -n \
  --arg status "$STATUS" \
  --arg source "$SOURCE" \
  --arg target "$TARGET" \
  --arg destination "${RESTORED_DESTINATION:-sandbox}" \
  --arg sha256 "$CHECKSUM" \
  --argjson size "$FILE_SIZE" \
  --argjson duration "$DURATION_SECONDS" \
  --arg rto_status "$RTO_STATUS" \
  --arg rpo_status "$RPO_STATUS" \
  --argjson data_age_minutes "$DATA_AGE_MINUTES" \
  --argjson failures "$(printf '%s\n' "${FAILURES[@]}" | jq -R . | jq -s 'map(select(length > 0))')" \
  '{
    status: $status,
    source: $source,
    target: $target,
    destination: $destination,
    sha256: $sha256,
    size_bytes: $size,
    restored_at: (now | todate),
    duration_seconds: $duration,
    rto: {
      status: $rto_status,
      duration_seconds: $duration,
      target_seconds: 14400
    },
    rpo: {
      status: $rpo_status,
      age_minutes: $data_age_minutes,
      target_minutes: 60
    },
    failures: $failures
  }' > "$REPORT_FILE"
cp "$REPORT_FILE" "$REPORT_DIR/restore-latest.json"

log "Report written to $REPORT_FILE"
log "============================================================"
if [ "$STATUS" = "SUCCESS" ]; then
  log "DR RESTORATION & VERIFICATION COMPLETED SUCCESSFULLY"
  log "============================================================"
  exit 0
else
  log "DR RESTORATION & VERIFICATION ENCOUNTERED FAILURES"
  log "============================================================"
  exit 1
fi
