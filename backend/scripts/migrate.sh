#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP_DIR="${SCRIPT_DIR}/../backups"

mkdir -p "${BACKUP_DIR}"

log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*"
}

error() {
    echo "[ERROR] $*" >&2
    exit 1
}

success() {
    echo "[SUCCESS] $*"
}

# Extracts the file path from a sqlite:// or sqlite: DATABASE_URL.
# The backend is SQLite-only -- see docs/adr/0001-sqlite-vs-postgres.md.
sqlite_db_path() {
    local url="${DATABASE_URL}"
    url="${url#sqlite://}"
    url="${url#sqlite:}"
    echo "${url%%\?*}"
}

backup_database() {
    local backup_file="${BACKUP_DIR}/backup_$(date +%Y%m%d_%H%M%S).db"
    local db_path
    db_path="$(sqlite_db_path)"

    log "Creating database backup: ${backup_file}"

    if ! sqlite3 "${db_path}" ".backup '${backup_file}'"; then
        error "Failed to create database backup"
    fi

    log "Backup created successfully"
    echo "${backup_file}"
}

run_migration() {
    local dry_run="${1:-false}"
    
    log "Running migrations (dry_run=${dry_run})"
    
    if [[ "${dry_run}" == "true" ]]; then
        log "DRY RUN MODE - No changes will be applied"
        sqlx migrate run --database-url "${DATABASE_URL}" --dry-run || error "Migration dry-run failed"
    else
        sqlx migrate run --database-url "${DATABASE_URL}" || error "Migration failed"
    fi
    
    success "Migrations completed"
}

rollback_migration() {
    local backup_file="$1"
    local db_path
    db_path="$(sqlite_db_path)"

    log "Rolling back database from backup: ${backup_file}"

    if [[ ! -f "${backup_file}" ]]; then
        error "Backup file not found: ${backup_file}"
    fi

    log "Replacing ${db_path} with ${backup_file}"
    cp "${db_path}" "${db_path}.pre-rollback.$(date +%s)" 2>/dev/null || true
    cp "${backup_file}" "${db_path}" || error "Failed to restore from backup"

    success "Database rolled back successfully"
}

check_status() {
    log "Checking migration status"
    sqlx migrate info --database-url "${DATABASE_URL}" || error "Failed to check migration status"
}

main() {
    local command="${1:-help}"
    
    case "${command}" in
        migrate)
            backup_database
            run_migration "false"
            check_status
            ;;
        migrate-dry-run)
            run_migration "true"
            ;;
        rollback)
            local backup_file="${2:-}"
            if [[ -z "${backup_file}" ]]; then
                error "Usage: $0 rollback <backup_file>"
            fi
            rollback_migration "${backup_file}"
            ;;
        status)
            check_status
            ;;
        list-backups)
            log "Available backups:"
            ls -lh "${BACKUP_DIR}"/backup_*.sql 2>/dev/null || log "No backups found"
            ;;
        help)
            cat << EOF
Database Migration Management

Usage: $0 <command> [options]

Commands:
    migrate              Run pending migrations with automatic backup
    migrate-dry-run      Preview migrations without applying changes
    rollback <file>      Rollback database from backup file
    status               Check current migration status
    list-backups         List available backup files
    help                 Show this help message

Environment Variables:
    DATABASE_URL         SQLite database URL (required, sqlite://... -- see
                          docs/adr/0001-sqlite-vs-postgres.md)

Examples:
    $0 migrate
    $0 migrate-dry-run
    $0 rollback backups/backup_20240424_183000.db
    $0 status

EOF
            ;;
        *)
            error "Unknown command: ${command}. Use '$0 help' for usage."
            ;;
    esac
}

if [[ -z "${DATABASE_URL:-}" ]]; then
    error "DATABASE_URL environment variable not set"
fi

main "$@"
