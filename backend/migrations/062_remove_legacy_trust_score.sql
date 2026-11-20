-- Remove legacy_trust_score column (contract phase)
-- contract-phase: after all pods are upgraded to v1.2.0
--
-- This migration removes the legacy_trust_score column which is no longer
-- used after v1.2.0. All pods must be running v1.2.0 or later before running
-- this migration.
--
-- Run this migration ONLY after:
-- 1. All pods are running v1.2.0 or later
-- 2. The application code no longer reads legacy_trust_score
-- 3. Data has been migrated to trust_score if needed

-- Drop the index first
DROP INDEX IF EXISTS idx_anchors_legacy_trust_score;

-- Drop the column
ALTER TABLE anchors DROP COLUMN legacy_trust_score;
