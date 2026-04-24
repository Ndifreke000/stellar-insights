-- Rollback: Drop indexes
DROP INDEX IF EXISTS idx_anchor_metrics_timestamp;
DROP INDEX IF EXISTS idx_anchor_metrics_anchor_time;
DROP INDEX IF EXISTS idx_assets_anchor;
DROP INDEX IF EXISTS idx_anchors_stellar_account;
DROP INDEX IF EXISTS idx_anchors_status;
DROP INDEX IF EXISTS idx_anchors_reliability;

-- Rollback: Drop tables
DROP TABLE IF EXISTS anchor_metrics_history;
DROP TABLE IF EXISTS assets;
DROP TABLE IF EXISTS anchors;
