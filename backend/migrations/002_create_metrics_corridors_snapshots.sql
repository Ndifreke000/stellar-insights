-- Create corridors table for tracking asset corridors
CREATE TABLE IF NOT EXISTS corridors (
    id TEXT PRIMARY KEY,
    source_asset_code TEXT NOT NULL,
    source_asset_issuer TEXT NOT NULL,
    destination_asset_code TEXT NOT NULL,
    destination_asset_issuer TEXT NOT NULL,
    reliability_score REAL DEFAULT 0,
    status TEXT DEFAULT 'active',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
);

-- Create metrics table for generic metric tracking
CREATE TABLE IF NOT EXISTS metrics (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    value REAL NOT NULL,
    entity_id TEXT, -- Can be anchor_id or corridor_id
    entity_type TEXT, -- 'anchor' or 'corridor'
    timestamp TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create snapshots table for historical state snapshots
CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON blob of the state
    hash TEXT, -- SHA-256 hash of the snapshot
    epoch INTEGER, -- Epoch identifier for historical tracking
    timestamp TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX idx_corridors_reliability ON corridors(reliability_score DESC);
CREATE INDEX idx_metrics_entity ON metrics(entity_id, entity_type);
CREATE INDEX idx_metrics_timestamp ON metrics(timestamp DESC);
CREATE INDEX idx_snapshots_entity ON snapshots(entity_id, entity_type);
CREATE INDEX idx_snapshots_timestamp ON snapshots(timestamp DESC);
CREATE INDEX idx_snapshots_epoch ON snapshots(epoch DESC);
