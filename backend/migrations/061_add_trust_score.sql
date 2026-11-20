-- Add trust_score column to anchors (expand phase)
-- contract-phase: none (expansion is always safe for rolling deployments)
--
-- This migration adds a new column to the anchors table. The old version
-- will continue to work because SQLite allows additional columns that
-- aren't selected.
--
-- Run this migration BEFORE deploying code that reads the new column.
-- Both old and new pods can run simultaneously after this migration.

-- Add trust_score column with default value
ALTER TABLE anchors ADD COLUMN trust_score REAL DEFAULT 0.0;

-- Create index for better query performance
CREATE INDEX IF NOT EXISTS idx_anchors_trust_score ON anchors(trust_score);
