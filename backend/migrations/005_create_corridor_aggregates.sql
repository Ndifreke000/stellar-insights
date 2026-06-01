-- Create corridor_metrics_hourly table for time-series aggregation
-- This table stores hourly aggregated metrics for each corridor
CREATE TABLE IF NOT EXISTS corridor_metrics_hourly (
    id TEXT PRIMARY KEY,
    corridor_key TEXT NOT NULL,
    asset_a_code TEXT NOT NULL,
    asset_a_issuer TEXT NOT NULL,
    asset_b_code TEXT NOT NULL,
    asset_b_issuer TEXT NOT NULL,
    hour_bucket TEXT NOT NULL, -- ISO 8601 timestamp truncated to hour
    total_transactions INTEGER DEFAULT 0,
    successful_transactions INTEGER DEFAULT 0,
    failed_transactions INTEGER DEFAULT 0,
    success_rate REAL DEFAULT 0,
    volume_usd REAL DEFAULT 0,
    avg_slippage_bps REAL DEFAULT 0, -- basis points
    avg_settlement_latency_ms INTEGER,
    liquidity_depth_usd REAL DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(corridor_key, hour_bucket)
);

-- Create aggregation_jobs table to track job execution
CREATE TABLE IF NOT EXISTS aggregation_jobs (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL, -- 'hourly', 'daily', etc.
    status TEXT NOT NULL, -- 'pending', 'running', 'completed', 'failed'
    start_time TEXT,
    end_time TEXT,
    last_processed_hour TEXT, -- Track which hour was last processed
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX idx_corridor_metrics_hourly_corridor ON corridor_metrics_hourly(corridor_key);
CREATE INDEX idx_corridor_metrics_hourly_hour ON corridor_metrics_hourly(hour_bucket DESC);
CREATE INDEX idx_corridor_metrics_hourly_success_rate ON corridor_metrics_hourly(success_rate DESC);
CREATE INDEX idx_aggregation_jobs_status ON aggregation_jobs(status, job_type);
CREATE INDEX idx_aggregation_jobs_created ON aggregation_jobs(created_at DESC);

-- Create view for latest corridor metrics
CREATE VIEW IF NOT EXISTS corridor_metrics_latest AS
SELECT 
    corridor_key,
    asset_a_code,
    asset_a_issuer,
    asset_b_code,
    asset_b_issuer,
    SUM(total_transactions) as total_transactions,
    SUM(successful_transactions) as successful_transactions,
    SUM(failed_transactions) as failed_transactions,
    AVG(success_rate) as avg_success_rate,
    SUM(volume_usd) as total_volume_usd,
    AVG(avg_slippage_bps) as avg_slippage_bps,
    AVG(avg_settlement_latency_ms) as avg_settlement_latency_ms,
    AVG(liquidity_depth_usd) as avg_liquidity_depth_usd,
    MAX(hour_bucket) as last_updated
FROM corridor_metrics_hourly
WHERE hour_bucket >= datetime('now', '-24 hours')
GROUP BY corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer;
