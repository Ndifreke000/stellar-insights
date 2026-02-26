-- Migration: Add indexes for query optimization
-- Purpose: Optimize N+1 query patterns and improve JOIN performance
-- Date: 2026-02-24

-- Add composite index for batch asset fetching (required for get_assets_by_anchors)
CREATE INDEX IF NOT EXISTS idx_assets_anchor_id_code 
  ON assets(anchor_id, asset_code ASC);

-- Add index for payments table filtering by account (used in payment queries)
CREATE INDEX IF NOT EXISTS idx_payments_source_account 
  ON payments(source_account);

CREATE INDEX IF NOT EXISTS idx_payments_destination_account 
  ON payments(destination_account);

-- Add index for anchor reliability score sorting (used in list_anchors)
CREATE INDEX IF NOT EXISTS idx_anchors_reliability_score 
  ON anchors(reliability_score DESC, updated_at DESC);

-- Add composite index for timestamp filtering on payments
CREATE INDEX IF NOT EXISTS idx_payments_timestamp 
  ON payments(created_at DESC);

-- Add index for corridor queries by source/destination pair
CREATE INDEX IF NOT EXISTS idx_corridors_pair 
  ON corridors(source_code, destination_code);

-- Add index for common filtering operations
CREATE INDEX IF NOT EXISTS idx_anchors_status 
  ON anchors(status);

CREATE INDEX IF NOT EXISTS idx_assets_chain 
  ON assets(blockchain_chain);

-- Add index for dashboard queries
CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp 
  ON snapshots(snapshot_time DESC);

-- Add index for pagination optimization
CREATE INDEX IF NOT EXISTS idx_anchors_id_score 
  ON anchors(id, reliability_score);

-- Analyze tables to update statistics
-- ANALYZE anchors;
-- ANALYZE assets;
-- ANALYZE payments;
-- ANALYZE corridors;
-- ANALYZE snapshots;
