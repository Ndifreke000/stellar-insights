-- Create payments table for storing normalized payment operations
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    transaction_hash TEXT NOT NULL,
    source_account TEXT NOT NULL,
    destination_account TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    asset_code TEXT,
    asset_issuer TEXT,
    amount REAL NOT NULL,
    created_at TEXT NOT NULL
);

-- Create ingestion_state table for tracking cursor positions
CREATE TABLE IF NOT EXISTS ingestion_state (
    task_name TEXT PRIMARY KEY,
    last_cursor TEXT NOT NULL,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create corridor_metrics table for historical tracking if needed (matches aggregates.rs)
CREATE TABLE IF NOT EXISTS corridor_metrics (
    corridor_key TEXT NOT NULL,
    asset_a_code TEXT NOT NULL,
    asset_a_issuer TEXT NOT NULL,
    asset_b_code TEXT NOT NULL,
    asset_b_issuer TEXT NOT NULL,
    date TEXT NOT NULL,
    total_transactions INTEGER DEFAULT 0,
    successful_transactions INTEGER DEFAULT 0,
    failed_transactions INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0,
    volume_usd REAL DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (corridor_key, date)
);
