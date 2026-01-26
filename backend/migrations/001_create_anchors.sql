-- Create anchors table for tracking anchor performance metrics
CREATE TABLE IF NOT EXISTS anchors (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    stellar_account TEXT NOT NULL UNIQUE,
    home_domain TEXT,
    total_transactions INTEGER DEFAULT 0,
    successful_transactions INTEGER DEFAULT 0,
    failed_transactions INTEGER DEFAULT 0,
    total_volume_usd REAL DEFAULT 0,
    avg_settlement_time_ms INTEGER DEFAULT 0,
    reliability_score REAL DEFAULT 0,
    status TEXT DEFAULT 'green',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create assets table for tracking issued assets per anchor
CREATE TABLE IF NOT EXISTS assets (
    id TEXT PRIMARY KEY,
    anchor_id TEXT NOT NULL REFERENCES anchors(id) ON DELETE CASCADE,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    total_supply REAL,
    num_holders INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(asset_code, asset_issuer)
);

-- Create anchor_metrics_history table for time-series data
CREATE TABLE IF NOT EXISTS anchor_metrics_history (
    id TEXT PRIMARY KEY,
    anchor_id TEXT NOT NULL REFERENCES anchors(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    success_rate REAL NOT NULL,
    failure_rate REAL NOT NULL,
    reliability_score REAL NOT NULL,
    total_transactions INTEGER NOT NULL,
    successful_transactions INTEGER NOT NULL,
    failed_transactions INTEGER NOT NULL,
    avg_settlement_time_ms INTEGER,
    volume_usd REAL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for better query performance
CREATE INDEX idx_anchors_reliability ON anchors(reliability_score DESC);
CREATE INDEX idx_anchors_status ON anchors(status);
CREATE INDEX idx_anchors_stellar_account ON anchors(stellar_account);
CREATE INDEX idx_assets_anchor ON assets(anchor_id);
CREATE INDEX idx_anchor_metrics_anchor_time ON anchor_metrics_history(anchor_id, timestamp DESC);
CREATE INDEX idx_anchor_metrics_timestamp ON anchor_metrics_history(timestamp DESC);
