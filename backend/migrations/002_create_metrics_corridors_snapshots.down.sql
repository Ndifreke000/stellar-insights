-- Rollback: Drop indexes
DROP INDEX IF EXISTS idx_snapshots_verified_at;
DROP INDEX IF EXISTS idx_snapshots_verification_status;
DROP INDEX IF EXISTS idx_snapshots_ledger;
DROP INDEX IF EXISTS idx_snapshots_epoch_desc;
DROP INDEX IF EXISTS idx_snapshots_snapshot_time;
DROP INDEX IF EXISTS idx_snapshots_timestamp;
DROP INDEX IF EXISTS idx_snapshots_entity;
DROP INDEX IF EXISTS idx_metrics_timestamp;
DROP INDEX IF EXISTS idx_metrics_entity;
DROP INDEX IF EXISTS idx_corridors_reliability;

-- Rollback: Drop tables
DROP TABLE IF EXISTS snapshots;
DROP TABLE IF EXISTS metrics;
DROP TABLE IF EXISTS corridors;
