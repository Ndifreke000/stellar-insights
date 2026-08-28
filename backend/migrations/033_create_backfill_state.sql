CREATE TABLE IF NOT EXISTS backfill_state (
    id TEXT PRIMARY KEY,
    current_ledger INTEGER NOT NULL,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
