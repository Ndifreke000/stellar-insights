-- Ledger Ingestion Tables

CREATE TABLE IF NOT EXISTS ingestion_cursor (
    id SERIAL PRIMARY KEY,
    last_ledger_sequence BIGINT NOT NULL,
    cursor TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ledgers (
    sequence BIGINT PRIMARY KEY,
    hash TEXT NOT NULL,
    close_time TIMESTAMP WITH TIME ZONE NOT NULL,
    transaction_count INTEGER DEFAULT 0,
    operation_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transactions (
    hash TEXT PRIMARY KEY,
    ledger_sequence BIGINT NOT NULL REFERENCES ledgers(sequence),
    source_account TEXT,
    fee BIGINT,
    operation_count INTEGER,
    successful BOOLEAN,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ledger_payments (
    id SERIAL PRIMARY KEY,
    ledger_sequence BIGINT NOT NULL REFERENCES ledgers(sequence),
    transaction_hash TEXT NOT NULL, -- references transactions(hash) but we might ingest payments without full tx indexing if optimized
    operation_type TEXT,
    source_account TEXT,
    destination TEXT,
    asset_code TEXT,
    asset_issuer TEXT,
    amount TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_ledger_payments_account ON ledger_payments(source_account);
CREATE INDEX idx_ledger_payments_destination ON ledger_payments(destination);
