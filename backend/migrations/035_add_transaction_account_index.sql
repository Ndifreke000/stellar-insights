-- Migration: Add composite index on transactions(account_id, created_at)
-- Purpose: Eliminate full sequential scans when querying transactions by account
--          at mainnet scale (millions of rows). Fixes issue #1623.
-- Date: 2026-06-29

CREATE INDEX IF NOT EXISTS idx_transactions_account_created
    ON transactions(account_id, created_at DESC);

ANALYZE transactions;
