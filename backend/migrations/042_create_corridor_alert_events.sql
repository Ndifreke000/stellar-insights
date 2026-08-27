-- Corridor performance alert events log
-- Migration: 042_create_corridor_alert_events.sql

CREATE TABLE IF NOT EXISTS corridor_alert_events (
    id TEXT PRIMARY KEY,
    config_id TEXT NOT NULL REFERENCES corridor_alert_configs(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    corridor_key TEXT NOT NULL,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'warning',
    message TEXT NOT NULL,
    old_value REAL,
    new_value REAL,
    threshold_value REAL,
    acknowledged BOOLEAN NOT NULL DEFAULT 0,
    acknowledged_at TEXT,
    triggered_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_corridor_alert_events_user ON corridor_alert_events(user_id);
CREATE INDEX IF NOT EXISTS idx_corridor_alert_events_corridor ON corridor_alert_events(corridor_key);
CREATE INDEX IF NOT EXISTS idx_corridor_alert_events_type ON corridor_alert_events(alert_type);
CREATE INDEX IF NOT EXISTS idx_corridor_alert_events_triggered ON corridor_alert_events(triggered_at DESC);
CREATE INDEX IF NOT EXISTS idx_corridor_alert_events_unack ON corridor_alert_events(user_id, acknowledged);
