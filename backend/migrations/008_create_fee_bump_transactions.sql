-- Create fee_bump_transactions table
CREATE TABLE IF NOT EXISTS fee_bump_transactions (
    transaction_hash TEXT PRIMARY KEY,
    ledger_sequence INTEGER NOT NULL,
    fee_source TEXT NOT NULL,
    fee_charged INTEGER NOT NULL,
    max_fee INTEGER NOT NULL,
    inner_transaction_hash TEXT NOT NULL,
    inner_max_fee INTEGER NOT NULL,
    signatures_count INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (ledger_sequence) REFERENCES ledgers(sequence)
);

-- Create index on ledger_sequence for faster lookups
CREATE INDEX IF NOT EXISTS idx_fee_bump_ledger_sequence ON fee_bump_transactions(ledger_sequence);

-- Create index on fee_source for analytics
CREATE INDEX IF NOT EXISTS idx_fee_bump_fee_source ON fee_bump_transactions(fee_source);

-- Create index on created_at for time-based queries
CREATE INDEX IF NOT EXISTS idx_fee_bump_created_at ON fee_bump_transactions(created_at);
