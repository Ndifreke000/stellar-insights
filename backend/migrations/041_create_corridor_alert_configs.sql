-- Corridor-specific alert configurations with granular thresholds
-- Migration: 041_create_corridor_alert_configs.sql

CREATE TABLE IF NOT EXISTS corridor_alert_configs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    corridor_key TEXT,
    name TEXT NOT NULL,
    success_rate_threshold REAL,
    latency_threshold_ms REAL,
    liquidity_threshold_usd REAL,
    success_rate_drop_pct REAL DEFAULT 10.0,
    latency_increase_pct REAL DEFAULT 50.0,
    liquidity_drop_pct REAL DEFAULT 30.0,
    cooldown_seconds INTEGER DEFAULT 300,
    notify_email BOOLEAN NOT NULL DEFAULT 0,
    notify_webhook BOOLEAN NOT NULL DEFAULT 0,
    notify_in_app BOOLEAN NOT NULL DEFAULT 1,
    notify_slack BOOLEAN NOT NULL DEFAULT 0,
    notify_telegram BOOLEAN NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    last_triggered_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_corridor_alert_configs_user ON corridor_alert_configs(user_id);
CREATE INDEX IF NOT EXISTS idx_corridor_alert_configs_corridor ON corridor_alert_configs(corridor_key);
CREATE INDEX IF NOT EXISTS idx_corridor_alert_configs_active ON corridor_alert_configs(is_active);
