-- Create liquidity_pools table for storing pool metadata and computed metrics
CREATE TABLE IF NOT EXISTS liquidity_pools (
    pool_id TEXT PRIMARY KEY,
    pool_type TEXT NOT NULL DEFAULT 'constant_product',
    fee_bp INTEGER NOT NULL DEFAULT 30,  -- fee in basis points (e.g., 30 = 0.30%)
    total_trustlines INTEGER NOT NULL DEFAULT 0,
    total_shares TEXT NOT NULL DEFAULT '0',
    reserve_a_asset_code TEXT NOT NULL,
    reserve_a_asset_issuer TEXT,
    reserve_a_amount REAL NOT NULL DEFAULT 0.0,
    reserve_b_asset_code TEXT NOT NULL,
    reserve_b_asset_issuer TEXT,
    reserve_b_amount REAL NOT NULL DEFAULT 0.0,
    -- Computed metrics
    total_value_usd REAL NOT NULL DEFAULT 0.0,
    volume_24h_usd REAL NOT NULL DEFAULT 0.0,
    fees_earned_24h_usd REAL NOT NULL DEFAULT 0.0,
    apy REAL NOT NULL DEFAULT 0.0,
    impermanent_loss_pct REAL NOT NULL DEFAULT 0.0,
    trade_count_24h INTEGER NOT NULL DEFAULT 0,
    last_synced_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create liquidity_pool_snapshots table for historical time-series data
CREATE TABLE IF NOT EXISTS liquidity_pool_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pool_id TEXT NOT NULL,
    reserve_a_amount REAL NOT NULL,
    reserve_b_amount REAL NOT NULL,
    total_value_usd REAL NOT NULL DEFAULT 0.0,
    volume_usd REAL NOT NULL DEFAULT 0.0,
    fees_usd REAL NOT NULL DEFAULT 0.0,
    apy REAL NOT NULL DEFAULT 0.0,
    impermanent_loss_pct REAL NOT NULL DEFAULT 0.0,
    trade_count INTEGER NOT NULL DEFAULT 0,
    snapshot_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pool_id) REFERENCES liquidity_pools(pool_id)
);

-- Indexes for liquidity_pools
CREATE INDEX IF NOT EXISTS idx_lp_apy ON liquidity_pools(apy DESC);
CREATE INDEX IF NOT EXISTS idx_lp_volume ON liquidity_pools(volume_24h_usd DESC);
CREATE INDEX IF NOT EXISTS idx_lp_total_value ON liquidity_pools(total_value_usd DESC);
CREATE INDEX IF NOT EXISTS idx_lp_updated ON liquidity_pools(updated_at);

-- Indexes for liquidity_pool_snapshots
CREATE INDEX IF NOT EXISTS idx_lps_pool_id ON liquidity_pool_snapshots(pool_id);
CREATE INDEX IF NOT EXISTS idx_lps_snapshot_at ON liquidity_pool_snapshots(snapshot_at);
CREATE INDEX IF NOT EXISTS idx_lps_pool_time ON liquidity_pool_snapshots(pool_id, snapshot_at);
