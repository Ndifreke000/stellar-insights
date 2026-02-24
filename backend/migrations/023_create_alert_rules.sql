-- Create alert_rules table
CREATE TABLE IF NOT EXISTS alert_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    corridor_id TEXT, -- NULL means it applies globally or is not corridor-specific
    metric_type TEXT NOT NULL, -- e.g., 'success_rate', 'latency', 'liquidity'
    condition TEXT NOT NULL, -- e.g., 'above', 'below', 'equals'
    threshold REAL NOT NULL,
    notify_email BOOLEAN NOT NULL DEFAULT 0,
    notify_webhook BOOLEAN NOT NULL DEFAULT 0,
    notify_in_app BOOLEAN NOT NULL DEFAULT 1,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    snoozed_until DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_alert_rules_user_id ON alert_rules(user_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_metric_type ON alert_rules(metric_type);
CREATE INDEX IF NOT EXISTS idx_alert_rules_corridor_id ON alert_rules(corridor_id);
