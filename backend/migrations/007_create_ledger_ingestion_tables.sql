-- Ledger Ingestion Tables

CREATE TABLE IF NOT EXISTS ingestion_cursor (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    last_ledger_sequence INTEGER NOT NULL,
    cursor TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ledgers (
    sequence INTEGER PRIMARY KEY,
    hash TEXT NOT NULL,
    close_time TEXT NOT NULL,
    transaction_count INTEGER DEFAULT 0,
    operation_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transactions (
    hash TEXT PRIMARY KEY,
    ledger_sequence INTEGER NOT NULL REFERENCES ledgers(sequence),
    source_account TEXT,
    fee INTEGER,
    operation_count INTEGER,
    successful INTEGER,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ledger_payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ledger_sequence INTEGER NOT NULL REFERENCES ledgers(sequence),
    transaction_hash TEXT NOT NULL, -- references transactions(hash) but we might ingest payments without full tx indexing if optimized
    operation_type TEXT,
    source_account TEXT,
    destination TEXT,
    asset_code TEXT,
    asset_issuer TEXT,
    amount TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_ledger_payments_account ON ledger_payments(source_account);
CREATE INDEX IF NOT EXISTS idx_ledger_payments_destination ON ledger_payments(destination);
