-- Migration: Add missing indexes
-- Purpose: Cover tables that had no indexes after audit of migrations 001-028
-- Date: 2026-04-24

-- pending_transactions: queried by status (polling for ready/submitted) and source_account
CREATE INDEX IF NOT EXISTS idx_pending_transactions_status
    ON pending_transactions(status);

CREATE INDEX IF NOT EXISTS idx_pending_transactions_source_account
    ON pending_transactions(source_account);

CREATE INDEX IF NOT EXISTS idx_pending_transactions_created_at
    ON pending_transactions(created_at DESC);

-- transaction_signatures: queried by transaction_id (FK lookups) and signer
CREATE INDEX IF NOT EXISTS idx_transaction_signatures_transaction_id
    ON transaction_signatures(transaction_id);

CREATE INDEX IF NOT EXISTS idx_transaction_signatures_signer
    ON transaction_signatures(signer);

-- api_usage_stats: composite index for the most common analytics query (endpoint + time window)
CREATE INDEX IF NOT EXISTS idx_api_usage_endpoint_timestamp
    ON api_usage_stats(endpoint, timestamp DESC);

-- Update query planner statistics
ANALYZE pending_transactions;
ANALYZE transaction_signatures;
ANALYZE api_usage_stats;
