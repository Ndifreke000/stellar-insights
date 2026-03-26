-- Create alert_rules table
-- Migration: 027_create_alert_rules.sql

CREATE TABLE IF NOT EXISTS alert_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    corridor_id TEXT,
    metric_type TEXT NOT NULL,
    condition TEXT NOT NULL,
    threshold REAL NOT NULL,
    notify_email BOOLEAN NOT NULL DEFAULT 0,
    notify_webhook BOOLEAN NOT NULL DEFAULT 0,
    notify_in_app BOOLEAN NOT NULL DEFAULT 1,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    snoozed_until TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_alert_rules_user_id ON alert_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_metric_type ON alert_rules(metric_type);
CREATE INDEX IF NOT EXISTS idx_alert_rules_corridor_id ON alert_rules(corridor_id);
