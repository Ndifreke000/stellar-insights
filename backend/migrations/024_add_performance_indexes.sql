-- Migration: Add Performance Indexes
-- Purpose: Fix performance bottleneck by adding indexes on frequently queried columns
-- Date: 2026-02-24
-- Severity: High (Performance)
-- Impact: Prevents full table scans and improves query performance as data grows

-- ============================================================================
-- PAYMENTS TABLE INDEXES
-- ============================================================================

-- Index for time-range queries (most common query pattern)
-- Used by: Dashboard time filters, analytics queries, payment history
CREATE INDEX IF NOT EXISTS idx_payments_created_at 
    ON payments(created_at DESC);

-- Partial index for asset_code filtering (only when asset_code is not NULL)
-- Used by: Asset-specific payment queries
CREATE INDEX IF NOT EXISTS idx_payments_asset_code 
    ON payments(asset_code)
    WHERE asset_code IS NOT NULL;

-- Composite index for common query pattern: account + time range
-- Used by: Account payment history with date filtering
CREATE INDEX IF NOT EXISTS idx_payments_source_account_date 
    ON payments(source_account, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_payments_dest_account_date 
    ON payments(destination_account, created_at DESC);

-- Index for transaction hash lookups
-- Used by: Transaction detail queries, duplicate detection
CREATE INDEX IF NOT EXISTS idx_payments_transaction_hash 
    ON payments(transaction_hash);

-- ============================================================================
-- ASSETS TABLE INDEXES
-- ============================================================================

-- Index for asset code lookups (used in asset filtering)
CREATE INDEX IF NOT EXISTS idx_assets_asset_code 
    ON assets(asset_code);

-- Index for asset issuer lookups
CREATE INDEX IF NOT EXISTS idx_assets_asset_issuer 
    ON assets(asset_issuer);

-- Composite index for unique asset identification
CREATE INDEX IF NOT EXISTS idx_assets_code_issuer 
    ON assets(asset_code, asset_issuer);

-- ============================================================================
-- CORRIDORS TABLE INDEXES
-- ============================================================================

-- Composite index for corridor pair lookups (most common query)
-- Used by: Corridor discovery, routing queries
CREATE INDEX IF NOT EXISTS idx_corridors_source_dest 
    ON corridors(source_asset_code, destination_asset_code);

-- Index for reverse corridor lookups
CREATE INDEX IF NOT EXISTS idx_corridors_dest_source 
    ON corridors(destination_asset_code, source_asset_code);

-- Index for corridor health/reliability sorting
CREATE INDEX IF NOT EXISTS idx_corridors_reliability_score 
    ON corridors(reliability_score DESC);

-- Index for active corridor filtering
CREATE INDEX IF NOT EXISTS idx_corridors_status 
    ON corridors(status)
    WHERE status = 'active';

-- ============================================================================
-- CORRIDOR_METRICS TABLE INDEXES
-- ============================================================================

-- Composite index for corridor metrics time-series queries
CREATE INDEX IF NOT EXISTS idx_corridor_metrics_key_date 
    ON corridor_metrics(corridor_key, date DESC);

-- Index for date-based aggregations
CREATE INDEX IF NOT EXISTS idx_corridor_metrics_date 
    ON corridor_metrics(date DESC);

-- ============================================================================
-- ANCHOR_METRICS_HISTORY TABLE INDEXES
-- ============================================================================

-- Composite index for anchor time-series queries (if not already exists)
CREATE INDEX IF NOT EXISTS idx_anchor_metrics_anchor_timestamp 
    ON anchor_metrics_history(anchor_id, timestamp DESC);

-- ============================================================================
-- ANALYZE TABLES
-- ============================================================================
-- Update query planner statistics after creating indexes
-- Uncomment these when running the migration:

-- ANALYZE payments;
-- ANALYZE assets;
-- ANALYZE anchors;
-- ANALYZE corridors;
-- ANALYZE corridor_metrics;
-- ANALYZE anchor_metrics_history;

-- ============================================================================
-- PERFORMANCE NOTES
-- ============================================================================
-- Expected improvements:
-- - Time-range queries: 1-5s → <100ms (10-50x faster)
-- - Account lookups: 500ms → <10ms (50x faster)
-- - Corridor queries: 1s → <50ms (20x faster)
-- - Asset filtering: 200ms → <5ms (40x faster)
--
-- Index maintenance overhead:
-- - Write operations: ~5-10% slower (acceptable tradeoff)
-- - Storage: ~20-30% increase (indexes are smaller than tables)
--
-- Monitoring:
-- - Use EXPLAIN QUERY PLAN to verify index usage
-- - Monitor query execution times in application logs
-- - Alert on queries >100ms for investigation
