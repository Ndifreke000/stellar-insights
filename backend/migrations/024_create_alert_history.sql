-- Create alert_history table
CREATE TABLE IF NOT EXISTS alert_history (
    id TEXT PRIMARY KEY,
    rule_id TEXT NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    corridor_id TEXT,
    metric_type TEXT NOT NULL,
    trigger_value REAL NOT NULL,
    threshold_value REAL NOT NULL,
    condition TEXT NOT NULL,
    message TEXT NOT NULL,
    is_read BOOLEAN NOT NULL DEFAULT 0,
    is_dismissed BOOLEAN NOT NULL DEFAULT 0,
    triggered_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_alert_history_user_id ON alert_history(user_id);
CREATE INDEX IF NOT EXISTS idx_alert_history_rule_id ON alert_history(rule_id);
CREATE INDEX IF NOT EXISTS idx_alert_history_is_read ON alert_history(is_read);
CREATE INDEX IF NOT EXISTS idx_alert_history_is_dismissed ON alert_history(is_read);
