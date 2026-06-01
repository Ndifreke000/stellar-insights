-- Account merge tracking for Stellar account lifecycle analytics
CREATE TABLE IF NOT EXISTS account_merges (
    operation_id TEXT PRIMARY KEY,
    transaction_hash TEXT NOT NULL,
    ledger_sequence INTEGER NOT NULL,
    source_account TEXT NOT NULL,
    destination_account TEXT NOT NULL,
    merged_balance REAL NOT NULL DEFAULT 0.0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (ledger_sequence) REFERENCES ledgers(sequence)
);

CREATE INDEX IF NOT EXISTS idx_account_merges_ledger ON account_merges(ledger_sequence);
CREATE INDEX IF NOT EXISTS idx_account_merges_source ON account_merges(source_account);
CREATE INDEX IF NOT EXISTS idx_account_merges_destination ON account_merges(destination_account);
CREATE INDEX IF NOT EXISTS idx_account_merges_created_at ON account_merges(created_at DESC);
