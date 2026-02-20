-- Trustline stats to track asset adoption
CREATE TABLE IF NOT EXISTS trustline_stats (
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    total_trustlines INTEGER NOT NULL DEFAULT 0,
    authorized_trustlines INTEGER NOT NULL DEFAULT 0,
    unauthorized_trustlines INTEGER NOT NULL DEFAULT 0,
    total_supply REAL NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (asset_code, asset_issuer)
);

-- Historical snapshots for charting trustline growth over time
CREATE TABLE IF NOT EXISTS trustline_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    total_trustlines INTEGER NOT NULL,
    authorized_trustlines INTEGER NOT NULL,
    unauthorized_trustlines INTEGER NOT NULL,
    total_supply REAL NOT NULL,
    snapshot_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for efficient chronological and per-asset queries
CREATE INDEX IF NOT EXISTS idx_trustline_snapshots_asset_time 
ON trustline_snapshots(asset_code, asset_issuer, snapshot_at DESC);
