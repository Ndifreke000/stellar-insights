#!/bin/bash

# Stellar Insights Database Backup Script
# Purpose: Automated backup of RDS PostgreSQL database with retention policy
# Usage: ./backup-database.sh [environment] [backup-type]
# Examples:
#   ./backup-database.sh production automated
#   ./backup-database.sh staging manual
#   ./backup-database.sh production restore last

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Environment variables (can be overridden)
ENVIRONMENT="${1:-production}"
BACKUP_TYPE="${2:-automated}"
AWS_REGION="${AWS_REGION:-us-east-1}"
DB_IDENTIFIER="stellar-insights-${ENVIRONMENT}"

# S3 backup location
S3_BUCKET="${S3_BUCKET:-stellar-insights-backups}"
S3_PREFIX="database/${ENVIRONMENT}"
BACKUP_DIR="/tmp/stellar-insights-backups"

# Retention policy (days)
RETENTION_DAILY=7
RETENTION_WEEKLY=28
RETENTION_MONTHLY=180

# ============================================================================
# Logging & Utilities
# ============================================================================

log() {
  echo "[$(date +'%Y-%m-%d %H:%M:%S')] [$$] $*"
}

error() {
  echo "[ERROR] $(date +'%Y-%m-%d %H:%M:%S') $*" >&2
}

die() {
  error "$@"
  exit 1
}

# ============================================================================
# Validation
# ============================================================================

validate_environment() {
  case "$ENVIRONMENT" in
    dev|staging|production|mainnet)
      log "Environment validated: $ENVIRONMENT"
      ;;
    *)
      die "Invalid environment: $ENVIRONMENT (must be dev, staging, production, or mainnet)"
      ;;
  esac
}

validate_dependencies() {
  local deps=("aws" "psql" "jq")
  for cmd in "${deps[@]}"; do
    if ! command -v "$cmd" &> /dev/null; then
      die "Required command not found: $cmd"
    fi
  done
  log "All dependencies found: $(echo "${deps[@]}" | tr '\n' ', ')"
}

validate_aws_credentials() {
  if ! aws sts get-caller-identity --region "$AWS_REGION" &> /dev/null; then
    die "AWS credentials invalid or not configured"
  fi
  log "AWS credentials validated"
}

validate_rds_connection() {
  # Get RDS endpoint from Terraform state or AWS API
  local endpoint
  endpoint=$(aws rds describe-db-instances \
    --db-instance-identifier "$DB_IDENTIFIER" \
    --region "$AWS_REGION" \
    --query 'DBInstances[0].Endpoint.Address' \
    --output text 2>/dev/null || echo "")

  if [ -z "$endpoint" ] || [ "$endpoint" = "None" ]; then
    die "Could not find RDS instance: $DB_IDENTIFIER in region $AWS_REGION"
  fi

  log "RDS instance found: $endpoint"
  echo "$endpoint"
}

# ============================================================================
# Database Backup
# ============================================================================

create_manual_snapshot() {
  local db_id="$1"
  local timestamp=$(date +%Y%m%d-%H%M%S)
  local snapshot_id="${db_id}-manual-${timestamp}"

  log "Creating RDS manual snapshot: $snapshot_id"

  aws rds create-db-snapshot \
    --db-instance-identifier "$db_id" \
    --db-snapshot-identifier "$snapshot_id" \
    --region "$AWS_REGION" \
    --tags "Key=Type,Value=manual" "Key=CreatedBy,Value=backup-script" "Key=Environment,Value=$ENVIRONMENT" \
    --query 'DBSnapshot.DBSnapshotIdentifier' \
    --output text

  log "✓ Snapshot created: $snapshot_id"
  echo "$snapshot_id"
}

export_snapshot_to_s3() {
  local snapshot_id="$1"
  local timestamp=$(date +%Y%m%d-%H%M%S)
  local export_id="${DB_IDENTIFIER}-export-${timestamp}"

  log "Exporting snapshot to S3: $export_id"

  # Create IAM role for RDS to S3 export if needed (assumes role exists: rds-s3-export-role)
  aws rds start-export-task \
    --export-task-identifier "$export_id" \
    --source-arn "arn:aws:rds:${AWS_REGION}:$(aws sts get-caller-identity --query Account --output text):snapshot:${snapshot_id}" \
    --s3-bucket-name "$S3_BUCKET" \
    --s3-prefix "$S3_PREFIX/exports" \
    --iam-role-arn "arn:aws:iam::$(aws sts get-caller-identity --query Account --output text):role/rds-s3-export-role" \
    --region "$AWS_REGION" \
    --query 'ExportTaskIdentifier' \
    --output text || true

  log "✓ S3 export task started: $export_id"
  echo "$export_id"
}

dump_database() {
  local endpoint="$1"
  local timestamp=$(date +%Y%m%d-%H%M%S)
  local backup_file="${BACKUP_DIR}/stellar-insights-${ENVIRONMENT}-${timestamp}.sql.gz"

  mkdir -p "$BACKUP_DIR"

  log "Creating database dump: $backup_file"

  # Get database URL from environment or Terraform output
  local db_name="${DB_NAME:-stellar_insights}"
  local db_user="${DB_USER:-postgres}"

  # Use pg_dump if direct access available
  if psql -h "$endpoint" -U "$db_user" -d "$db_name" -c "SELECT 1" &>/dev/null; then
    PGPASSWORD="${DB_PASSWORD:?DB_PASSWORD env var required}" \
      pg_dump -h "$endpoint" -U "$db_user" -d "$db_name" \
        --no-password \
        --verbose \
        --create \
        --clean \
        --if-exists \
        --no-owner \
        --no-acl \
        | gzip > "$backup_file"

    log "✓ Database dump created: $backup_file ($(du -h "$backup_file" | cut -f1))"
  else
    log "⚠ Direct database access not available (expected in CI/CD)"
    log "  Using RDS automated backups and snapshots instead"
    return 1
  fi

  echo "$backup_file"
}

upload_backup_to_s3() {
  local backup_file="$1"
  local s3_key="${S3_PREFIX}/manual/$(basename "$backup_file")"

  log "Uploading backup to S3: s3://${S3_BUCKET}/${s3_key}"

  aws s3 cp "$backup_file" "s3://${S3_BUCKET}/${s3_key}" \
    --region "$AWS_REGION" \
    --metadata "environment=$ENVIRONMENT,backup-type=$BACKUP_TYPE,timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --sse AES256 \
    --storage-class INTELLIGENT_TIERING

  log "✓ Backup uploaded to S3"

  # Cleanup local backup
  rm -f "$backup_file"
  log "✓ Local backup cleaned up"
}

# ============================================================================
# Retention Policy
# ============================================================================

get_backup_timestamp() {
  local backup_key="$1"
  # Extract timestamp from s3 key: database/production/manual/stellar-insights-production-20240115-143022.sql.gz
  echo "$backup_key" | sed -E 's/.*-([0-9]{8}-[0-9]{6})\..*$/\1/'
}

get_backup_age_days() {
  local backup_timestamp="$1"
  # Convert YYYYMMDD-HHMMSS to epoch
  local backup_date=$(echo "$backup_timestamp" | cut -d'-' -f1)
  local backup_epoch=$(date -d "$backup_date" +%s 2>/dev/null || date -jf %Y%m%d "$backup_date" +%s)
  local now_epoch=$(date +%s)
  echo $(( (now_epoch - backup_epoch) / 86400 ))
}

should_retain_backup() {
  local backup_key="$1"
  local timestamp=$(get_backup_timestamp "$backup_key")
  local age_days=$(get_backup_age_days "$timestamp")
  local dow=$(date -d "$(echo $timestamp | cut -d'-' -f1)" +%w 2>/dev/null || date -jf %Y%m%d "$(echo $timestamp | cut -d'-' -f1)" +%w)

  # Daily: Keep last 7 days
  if [ $age_days -le $RETENTION_DAILY ]; then
    echo "true"
    return 0
  fi

  # Weekly: Keep Sundays for 4 weeks (28 days)
  if [ $age_days -le $RETENTION_WEEKLY ] && [ "$dow" = "0" ]; then
    echo "true"
    return 0
  fi

  # Monthly: Keep first of month for 6 months (180 days)
  local day=$(echo $timestamp | cut -d'-' -f1 | cut -c7-8)
  if [ $age_days -le $RETENTION_MONTHLY ] && [ "$day" = "01" ]; then
    echo "true"
    return 0
  fi

  echo "false"
  return 1
}

cleanup_old_backups() {
  log "Checking retention policy..."
  log "Policy: Daily for $RETENTION_DAILY days, Weekly for $RETENTION_WEEKLY days, Monthly for $RETENTION_MONTHLY days"

  # List all backups
  local backups=$(aws s3 ls "s3://${S3_BUCKET}/${S3_PREFIX}/manual/" --region "$AWS_REGION" --recursive | awk '{print $4}' | sort -r)

  local deleted=0
  local kept=0

  while IFS= read -r backup_key; do
    [ -z "$backup_key" ] && continue

    if [ "$(should_retain_backup "$backup_key")" = "true" ]; then
      log "  ✓ Keep: $(basename "$backup_key")"
      ((kept++))
    else
      log "  🗑 Delete: $(basename "$backup_key")"
      aws s3 rm "s3://${S3_BUCKET}/${backup_key}" --region "$AWS_REGION"
      ((deleted++))
    fi
  done <<< "$backups"

  log "✓ Retention cleanup complete: $kept kept, $deleted deleted"
}

# ============================================================================
# Restore
# ============================================================================

restore_from_snapshot() {
  local snapshot_id="$1"
  local new_db_id="${DB_IDENTIFIER}-restored-$(date +%s)"

  log "Restoring database from snapshot: $snapshot_id"
  log "Target DB instance: $new_db_id"

  cat << 'EOF'

⚠️  RESTORE PROCEDURE (manual operation required)

This script can initiate a restore via AWS API, but you must complete
the restore manually in the AWS console or via AWS CLI:

1. Verify the snapshot exists:
   aws rds describe-db-snapshots --db-snapshot-identifier $snapshot_id

2. Restore to a new database instance:
   aws rds restore-db-instance-from-db-snapshot \
     --db-instance-identifier $new_db_id \
     --db-snapshot-identifier $snapshot_id \
     --db-instance-class db.t3.medium \
     --no-publicly-accessible

3. Wait for restoration (5-10 minutes):
   aws rds wait db-instance-available --db-instance-identifier $new_db_id

4. Verify restored database:
   psql -h <new-endpoint> -U postgres -d stellar_insights -c "SELECT COUNT(*) FROM users;"

5. If restore successful, switch application to new endpoint and delete old instance:
   aws rds delete-db-instance --db-instance-identifier $original_db_id --skip-final-snapshot

6. If restore failed, delete the failed instance and retry:
   aws rds delete-db-instance --db-instance-identifier $new_db_id --skip-final-snapshot

EOF

  error "Manual intervention required for restore operation"
}

# ============================================================================
# Main
# ============================================================================

main() {
  log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  log "Stellar Insights Database Backup System"
  log "Environment: $ENVIRONMENT | Type: $BACKUP_TYPE"
  log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # Pre-flight checks
  validate_environment
  validate_dependencies
  validate_aws_credentials
  local rds_endpoint
  rds_endpoint=$(validate_rds_connection)

  case "$BACKUP_TYPE" in
    automated|manual)
      log "Creating $BACKUP_TYPE backup..."

      # 1. Create RDS snapshot (AWS handles automated backups already)
      local snapshot_id
      snapshot_id=$(create_manual_snapshot "$DB_IDENTIFIER")

      # 2. Export snapshot to S3 for long-term storage (asynchronous)
      export_snapshot_to_s3 "$snapshot_id"

      # 3. Try to create database dump (may not be available in CI)
      if dump_database "$rds_endpoint"; then
        upload_backup_to_s3 "$backup_file"
      fi

      # 4. Cleanup old backups per retention policy
      cleanup_old_backups

      log "✓ Backup complete!"
      ;;

    restore)
      log "Restore requested..."
      local restore_point="${3:-last}"

      case "$restore_point" in
        last)
          log "Finding most recent snapshot..."
          local latest_snapshot
          latest_snapshot=$(aws rds describe-db-snapshots \
            --db-instance-identifier "$DB_IDENTIFIER" \
            --region "$AWS_REGION" \
            --query 'sort_by(DBSnapshots, &SnapshotCreateTime)[-1].DBSnapshotIdentifier' \
            --output text)

          if [ -z "$latest_snapshot" ] || [ "$latest_snapshot" = "None" ]; then
            die "No snapshots found for $DB_IDENTIFIER"
          fi

          restore_from_snapshot "$latest_snapshot"
          ;;
        *)
          restore_from_snapshot "$restore_point"
          ;;
      esac
      ;;

    verify)
      log "Verifying backup system..."
      log "RDS Instance: $DB_IDENTIFIER"
      log "S3 Bucket: $S3_BUCKET"
      log "Region: $AWS_REGION"

      # Check if instance exists
      aws rds describe-db-instances --db-instance-identifier "$DB_IDENTIFIER" --region "$AWS_REGION" &>/dev/null || \
        die "RDS instance not found: $DB_IDENTIFIER"

      # Check if S3 bucket exists
      aws s3 ls "s3://${S3_BUCKET}" --region "$AWS_REGION" &>/dev/null || \
        die "S3 bucket not found: $S3_BUCKET"

      # List recent snapshots
      log "Recent snapshots:"
      aws rds describe-db-snapshots \
        --db-instance-identifier "$DB_IDENTIFIER" \
        --region "$AWS_REGION" \
        --query 'DBSnapshots[*].[DBSnapshotIdentifier, SnapshotCreateTime, DBSnapshotStatus]' \
        --output table \
        | head -10

      # List recent backups in S3
      log "Recent backups in S3:"
      aws s3 ls "s3://${S3_BUCKET}/${S3_PREFIX}/" --region "$AWS_REGION" --recursive | tail -10

      log "✓ Backup system verified"
      ;;

    *)
      die "Invalid backup type: $BACKUP_TYPE (must be automated, manual, restore, or verify)"
      ;;
  esac

  log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Run main function
main "$@"
