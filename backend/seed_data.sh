#!/bin/bash

# Comprehensive database seeding script for development
# Seeds realistic test data for all major entities
# Usage: ./seed_data.sh [database_path] [--reset]

set -e

DB_PATH="${1:-.payraider.db}"
RESET_FLAG="${2:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

# Check if sqlite3 is installed
if ! command -v sqlite3 &> /dev/null; then
    log_error "sqlite3 is not installed. Please install it first."
    exit 1
fi

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    log_error "Database file not found: $DB_PATH"
    log_info "Run migrations first: sqlx migrate run"
    exit 1
fi

log_info "Starting database seeding..."
echo ""

# Optional: Reset database (delete existing seed data)
if [ "$RESET_FLAG" = "--reset" ]; then
    log_warning "Resetting seed data..."
    sqlite3 "$DB_PATH" << EOF
DELETE FROM governance_comments WHERE proposal_id IN (SELECT id FROM governance_proposals WHERE created_by LIKE 'GPROPOSER%');
DELETE FROM governance_votes WHERE proposal_id IN (SELECT id FROM governance_proposals WHERE created_by LIKE 'GPROPOSER%');
DELETE FROM governance_proposals WHERE created_by LIKE 'GPROPOSER%';
DELETE FROM api_keys WHERE wallet_address LIKE 'G%' AND (name LIKE '%Key%' OR name LIKE '%Test%');
DELETE FROM users WHERE username IN ('admin', 'analyst', 'developer', 'viewer');
DELETE FROM metrics WHERE entity_id LIKE 'metric-%' OR entity_id LIKE 'corridor-%' OR entity_id LIKE 'anchor-%';
DELETE FROM anchor_metrics_history WHERE anchor_id LIKE 'anchor-%';
DELETE FROM corridors WHERE id LIKE 'corridor-%';
DELETE FROM assets WHERE id LIKE 'asset-%';
DELETE FROM anchors WHERE id LIKE 'anchor-%';
EOF
    log_success "Seed data reset"
    echo ""
fi

# Load comprehensive seed data
log_info "Loading comprehensive seed data..."
sqlite3 "$DB_PATH" < "$(dirname "$0")/migrations/030_comprehensive_seed_data.sql"
log_success "Seed data loaded"
echo ""

# Verify data was inserted
log_info "Verifying seeded data..."
echo ""

ANCHOR_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM anchors WHERE id LIKE 'anchor-%';")
log_success "Anchors: $ANCHOR_COUNT"

ASSET_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM assets WHERE id LIKE 'asset-%';")
log_success "Assets: $ASSET_COUNT"

CORRIDOR_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM corridors WHERE id LIKE 'corridor-%';")
log_success "Corridors: $CORRIDOR_COUNT"

METRIC_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM metrics WHERE entity_id LIKE 'metric-%';")
log_success "Metrics: $METRIC_COUNT"

HISTORY_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM anchor_metrics_history WHERE anchor_id LIKE 'anchor-%';")
log_success "Metrics History: $HISTORY_COUNT"

USER_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM users WHERE username IN ('admin', 'analyst', 'developer', 'viewer');")
log_success "Test Users: $USER_COUNT"

API_KEY_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM api_keys WHERE name LIKE '%Key%';")
log_success "API Keys: $API_KEY_COUNT"

PROPOSAL_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM governance_proposals WHERE created_by LIKE 'GPROPOSER%';")
log_success "Governance Proposals: $PROPOSAL_COUNT"

VOTE_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM governance_votes WHERE id LIKE 'vote-%';")
log_success "Governance Votes: $VOTE_COUNT"

echo ""
log_success "Database seeding completed successfully!"
echo ""

# Display sample queries
log_info "Sample queries to explore the data:"
echo ""
echo "  # View all anchors"
echo "  sqlite3 $DB_PATH \"SELECT id, name, status, reliability_score FROM anchors WHERE id LIKE 'anchor-%' ORDER BY reliability_score DESC;\""
echo ""
echo "  # View corridors by reliability"
echo "  sqlite3 $DB_PATH \"SELECT source_asset_code, destination_asset_code, reliability_score, status FROM corridors WHERE id LIKE 'corridor-%' ORDER BY reliability_score DESC;\""
echo ""
echo "  # View recent metrics"
echo "  sqlite3 $DB_PATH \"SELECT anchor_id, timestamp, success_rate, reliability_score FROM anchor_metrics_history WHERE anchor_id LIKE 'anchor-%' ORDER BY timestamp DESC LIMIT 10;\""
echo ""
echo "  # View governance proposals"
echo "  sqlite3 $DB_PATH \"SELECT id, title, status, created_at FROM governance_proposals WHERE created_by LIKE 'GPROPOSER%';\""
echo ""
echo "  # View test users"
echo "  sqlite3 $DB_PATH \"SELECT username FROM users WHERE username IN ('admin', 'analyst', 'developer', 'viewer');\""
echo ""

log_info "Test user credentials (password: password):"
echo "  - admin"
echo "  - analyst"
echo "  - developer"
echo "  - viewer"
echo ""

log_info "To reset seed data later, run:"
echo "  ./seed_data.sh $DB_PATH --reset"
echo ""
