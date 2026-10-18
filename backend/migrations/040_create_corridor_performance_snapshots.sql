-- Corridor performance snapshots for tracking metric changes over time
-- Migration: 040_create_corridor_performance_snapshots.sql

CREATE TABLE IF NOT EXISTS corridor_performance_snapshots (
    id TEXT PRIMARY KEY,
    corridor_key TEXT NOT NULL,
    source_asset_code TEXT NOT NULL,
    source_asset_issuer TEXT NOT NULL,
    destination_asset_code TEXT NOT NULL,
    destination_asset_issuer TEXT NOT NULL,
    success_rate REAL NOT NULL,
    avg_settlement_latency_ms REAL NOT NULL,
    liquidity_depth_usd REAL NOT NULL,
    volume_usd REAL NOT NULL,
    total_transactions INTEGER NOT NULL DEFAULT 0,
    successful_transactions INTEGER NOT NULL DEFAULT 0,
    failed_transactions INTEGER NOT NULL DEFAULT 0,
    snapshot_time TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_corridor_snapshots_key ON corridor_performance_snapshots(corridor_key);
CREATE INDEX IF NOT EXISTS idx_corridor_snapshots_time ON corridor_performance_snapshots(snapshot_time DESC);
CREATE INDEX IF NOT EXISTS idx_corridor_snapshots_key_time ON corridor_performance_snapshots(corridor_key, snapshot_time DESC);
