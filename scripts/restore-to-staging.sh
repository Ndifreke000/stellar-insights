#!/usr/bin/env bash
#
# Restore a production backup into the staging Kubernetes deployment.
#
# Usage:
#   ./scripts/restore-to-staging.sh latest --yes
#   ./scripts/restore-to-staging.sh 2026-09-01T12:00:00Z --yes   # point in time
#
# Steps:
#   1. Restores LITESTREAM_REPLICA_URL (production) and verifies it with verify-backup.sh
#   2. Scales the staging backend to 0
#   3. Copies the restored database onto the staging PVC (removing stale WAL/SHM files
#      and Litestream's local state, so staging's sidecar starts a fresh generation)
#   4. Scales the backend back up, waits for the rollout and checks /health
#
# DESTRUCTIVE for staging data. Requires --yes. Needs kubectl access to the staging
# namespace plus everything verify-backup.sh needs (litestream, sqlite3, jq).
#
# Environment (defaults match k8s/overlays/staging):
#   LITESTREAM_REPLICA_URL  Production replica, e.g. s3://payraider-db-backups-<account>/production/payraider.db
#   STAGING_NAMESPACE       default: payraider-staging
#   STAGING_DEPLOYMENT      default: staging-payraider-backend
#   STAGING_PVC             default: staging-payraider-backend-data
#   STAGING_DB_FILE         default: payraider.db (on the PVC, mounted at /data)

set -euo pipefail

TARGET="latest"
CONFIRMED="no"
for arg in "$@"; do
  case "$arg" in
    --yes|-y) CONFIRMED="yes" ;;
    -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
    *) TARGET="$arg" ;;
  esac
done

NAMESPACE="${STAGING_NAMESPACE:-payraider-staging}"
DEPLOYMENT="${STAGING_DEPLOYMENT:-staging-payraider-backend}"
PVC="${STAGING_PVC:-staging-payraider-backend-data}"
DB_FILE="${STAGING_DB_FILE:-payraider.db}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HELPER_POD="db-restore-$(date +%s)"

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*"; }
die() { log "ERROR: $*" >&2; exit 1; }

[ "$CONFIRMED" = "yes" ] || die "This replaces the staging database. Re-run with --yes to confirm."
[[ "$NAMESPACE" == *staging* ]] || die "Refusing to restore into non-staging namespace '$NAMESPACE'"
command -v kubectl >/dev/null 2>&1 || die "kubectl is required"

WORK_DIR="$(mktemp -d)"
REPLICAS=""
cleanup() {
  kubectl -n "$NAMESPACE" delete pod "$HELPER_POD" --ignore-not-found --wait=false >/dev/null 2>&1 || true
  # Never leave staging scaled to zero if something failed part-way.
  if [ -n "$REPLICAS" ]; then
    kubectl -n "$NAMESPACE" scale "deployment/$DEPLOYMENT" --replicas="$REPLICAS" >/dev/null 2>&1 || true
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

# 1. Restore and verify
log "Restoring and verifying production backup ($TARGET)"
KEEP_RESTORED_DB=true BACKUP_REPORT_DIR="$WORK_DIR/report" \
  "$SCRIPT_DIR/verify-backup.sh" "$TARGET" | tee "$WORK_DIR/verify.log"
RESTORED_DB=$(sed -n 's/.*Restored database kept at //p' "$WORK_DIR/verify.log" | tail -n 1)
[ -f "$RESTORED_DB" ] || die "Could not locate restored database"
trap 'rm -rf "$(dirname "$RESTORED_DB")"; cleanup' EXIT

# 2. Stop the staging backend (single SQLite writer)
REPLICAS=$(kubectl -n "$NAMESPACE" get "deployment/$DEPLOYMENT" -o jsonpath='{.spec.replicas}')
[ "${REPLICAS:-0}" -ge 1 ] || REPLICAS=1
log "Scaling $DEPLOYMENT to 0 (was $REPLICAS)"
kubectl -n "$NAMESPACE" scale "deployment/$DEPLOYMENT" --replicas=0
kubectl -n "$NAMESPACE" wait --for=delete pod -l "app=payraider,component=backend" --timeout=180s 2>/dev/null \
  || kubectl -n "$NAMESPACE" rollout status "deployment/$DEPLOYMENT" --timeout=180s

# 3. Copy the database onto the PVC through a short-lived helper pod
#    (same UID/fsGroup as the backend container in k8s/backend/deployment.yaml)
log "Starting helper pod $HELPER_POD with PVC $PVC"
kubectl -n "$NAMESPACE" run "$HELPER_POD" --image=busybox:1.36 --restart=Never --overrides="$(cat <<JSON
{
  "spec": {
    "securityContext": {"runAsUser": 1000, "fsGroup": 1001, "runAsNonRoot": true},
    "containers": [{
      "name": "restore",
      "image": "busybox:1.36",
      "command": ["sleep", "900"],
      "securityContext": {"allowPrivilegeEscalation": false, "capabilities": {"drop": ["ALL"]}},
      "volumeMounts": [{"name": "data", "mountPath": "/data"}]
    }],
    "volumes": [{"name": "data", "persistentVolumeClaim": {"claimName": "$PVC"}}]
  }
}
JSON
)"
kubectl -n "$NAMESPACE" wait --for=condition=Ready "pod/$HELPER_POD" --timeout=180s

log "Copying restored database to /data/$DB_FILE"
kubectl -n "$NAMESPACE" cp "$RESTORED_DB" "$HELPER_POD:/data/$DB_FILE.restore"
kubectl -n "$NAMESPACE" exec "$HELPER_POD" -- sh -c "
  set -e
  cd /data
  [ -f '$DB_FILE' ] && cp '$DB_FILE' '$DB_FILE.pre-restore'
  rm -f '$DB_FILE-wal' '$DB_FILE-shm'
  rm -rf '.$DB_FILE-litestream'
  mv '$DB_FILE.restore' '$DB_FILE'
  ls -l /data
"
kubectl -n "$NAMESPACE" delete pod "$HELPER_POD" --wait=true

# 4. Bring staging back and smoke-check it
log "Scaling $DEPLOYMENT back to $REPLICAS"
kubectl -n "$NAMESPACE" scale "deployment/$DEPLOYMENT" --replicas="$REPLICAS"
REPLICAS=""
kubectl -n "$NAMESPACE" rollout status "deployment/$DEPLOYMENT" --timeout=300s

POD=$(kubectl -n "$NAMESPACE" get pods -l "app=payraider,component=backend" \
  -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || true)
if [ -n "$POD" ]; then
  kubectl -n "$NAMESPACE" exec "$POD" -c backend -- curl -fsS http://127.0.0.1:8080/health >/dev/null \
    && log "Staging /health OK" \
    || die "Staging backend is not healthy after restore (previous DB kept at /data/$DB_FILE.pre-restore)"
fi

log "Staging restore complete (previous database kept at /data/$DB_FILE.pre-restore)"
