#!/bin/bash
# Analyzes SQLite index usage: runs ANALYZE to populate sqlite_stat1 (row-count
# estimates per index), then EXPLAIN QUERY PLAN for a set of representative
# queries against the app's hot tables, flagging any that fall back to a full
# table SCAN instead of using an index.
#
# Usage: ./analyze_indexes.sh [database_path]

set -e

DB_PATH="${1:-.payraider.db}"

if ! command -v sqlite3 &> /dev/null; then
    echo "✗ sqlite3 is not installed. Please install it first."
    exit 1
fi

if [ ! -f "$DB_PATH" ]; then
    echo "✗ Database file not found: $DB_PATH"
    exit 1
fi

echo "ℹ Running ANALYZE to refresh index statistics..."
sqlite3 "$DB_PATH" "ANALYZE;"
echo ""

echo "ℹ All indexes and their estimated selectivity (lower stat = more selective):"
echo ""
sqlite3 -header -column "$DB_PATH" \
  "SELECT m.name AS table_name, s.idx AS index_name, s.stat
   FROM sqlite_stat1 s
   JOIN sqlite_master m ON m.name = s.tbl
   ORDER BY m.name, s.idx;"
echo ""

echo "ℹ Indexes defined but never populated in sqlite_stat1 (never analyzed / unused so far):"
sqlite3 "$DB_PATH" \
  "SELECT name FROM sqlite_master
   WHERE type = 'index' AND name NOT LIKE 'sqlite_autoindex%'
     AND name NOT IN (SELECT idx FROM sqlite_stat1 WHERE idx IS NOT NULL)
   ORDER BY name;"
echo ""

echo "ℹ EXPLAIN QUERY PLAN for representative hot-path queries:"
echo "  (a plan step containing 'SCAN' without 'USING INDEX' means a full table scan)"
echo ""

# Add representative queries for your busiest endpoints here as they change.
QUERIES=(
  "SELECT * FROM corridors WHERE source_asset_code = 'USDC' ORDER BY reliability_score DESC LIMIT 20"
  "SELECT * FROM anchors WHERE status = 'active' ORDER BY reliability_score DESC LIMIT 20"
  "SELECT * FROM anchor_metrics_history WHERE anchor_id = 'anchor-1' ORDER BY timestamp DESC LIMIT 50"
  "SELECT * FROM metrics WHERE entity_id = 'corridor-1' ORDER BY timestamp DESC LIMIT 50"
)

for q in "${QUERIES[@]}"; do
  echo "-- $q"
  sqlite3 "$DB_PATH" "EXPLAIN QUERY PLAN $q;"
  echo ""
done

echo "✓ Index analysis complete."
