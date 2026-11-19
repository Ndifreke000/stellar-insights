-- Account lineage for merged accounts (#2773).
--
-- One row per merged account: `merged_from` was merged into `merged_into` at
-- `ledger_sequence`. Following the edges from an account gives the account
-- that survives the merges, so its payments can be attributed to that
-- account's anchor. Resolution (services/account_lineage.rs) uses a recursive
-- CTE with a depth bound and a cycle guard.
CREATE TABLE IF NOT EXISTS account_lineage (
    merged_from TEXT PRIMARY KEY,
    merged_into TEXT NOT NULL,
    ledger_sequence INTEGER NOT NULL,
    operation_id TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (merged_from <> merged_into)
);

CREATE INDEX IF NOT EXISTS idx_account_lineage_into ON account_lineage(merged_into);

-- Backfill from merges recorded before this table existed. If an account
-- shows up as a source more than once (it was recreated and merged again),
-- the latest ledger wins.
INSERT OR IGNORE INTO account_lineage (merged_from, merged_into, ledger_sequence, operation_id, created_at)
SELECT m.source_account, m.destination_account, m.ledger_sequence, m.operation_id, m.created_at
FROM account_merges m
WHERE m.source_account <> m.destination_account
  AND m.ledger_sequence = (
      SELECT MAX(m2.ledger_sequence) FROM account_merges m2
      WHERE m2.source_account = m.source_account
  );

-- Corridor metrics to recompute after a new merge changes the lineage.
-- AggregationService drains this queue.
CREATE TABLE IF NOT EXISTS lineage_recompute_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account TEXT NOT NULL,
    ledger_sequence INTEGER NOT NULL,
    requested_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at DATETIME
);

CREATE INDEX IF NOT EXISTS idx_lineage_recompute_pending
    ON lineage_recompute_queue(processed_at) WHERE processed_at IS NULL;
