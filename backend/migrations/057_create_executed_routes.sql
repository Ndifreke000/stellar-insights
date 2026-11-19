-- Executed path-payment routes for corridor route analysis (#2774).
CREATE TABLE IF NOT EXISTS executed_routes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL UNIQUE,
    transaction_hash TEXT NOT NULL,
    ledger INTEGER,
    operation_type TEXT NOT NULL,
    corridor_key TEXT NOT NULL,
    source_asset TEXT NOT NULL,
    destination_asset TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '[]',
    hop_count INTEGER NOT NULL,
    source_amount REAL,
    destination_amount REAL,
    quoted_amount REAL,
    hop_amounts TEXT NOT NULL DEFAULT '[]',
    successful INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_executed_routes_corridor ON executed_routes(corridor_key, id DESC);
