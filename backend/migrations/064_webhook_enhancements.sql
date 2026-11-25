-- Webhook enhancements migration
-- Adds: per-endpoint ordering (created_at index), auto-disable flags, secret rotation, payload versioning, delivery logs

-- Add delivery_logs table for tracking webhook deliveries
CREATE TABLE IF NOT EXISTS delivery_logs (
    id TEXT PRIMARY KEY,
    webhook_id TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event_id TEXT REFERENCES webhook_events(id) ON DELETE SET NULL,
    delivery_id TEXT NOT NULL,
    status TEXT NOT NULL,
    http_status INTEGER,
    response_body TEXT,
    error_message TEXT,
    signature TEXT,
    event_version TEXT,
    timestamp TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for delivery logs
CREATE INDEX IF NOT EXISTS idx_delivery_logs_webhook_id ON delivery_logs(webhook_id);
CREATE INDEX IF NOT EXISTS idx_delivery_logs_event_id ON delivery_logs(event_id);
CREATE INDEX IF NOT EXISTS idx_delivery_logs_timestamp ON delivery_logs(timestamp);

-- Add secret_rotation_enabled and primary/secondary secrets for rotation support
ALTER TABLE webhooks ADD COLUMN secret_rotation_enabled INTEGER DEFAULT 0;
ALTER TABLE webhooks ADD COLUMN secondary_secret TEXT;
ALTER TABLE webhooks ADD COLUMN secret_rotation_period_days INTEGER DEFAULT 30;
ALTER TABLE webhooks ADD COLUMN next_rotation_date TEXT;

-- Add payload_version to webhook_events for versioning
ALTER TABLE webhook_events ADD COLUMN event_version TEXT DEFAULT 'v1';
ALTER TABLE webhook_events ADD COLUMN signature TEXT;

-- Add disabled_reason to webhooks for auto-disable
ALTER TABLE webhooks ADD COLUMN disabled_reason TEXT;
ALTER TABLE webhooks ADD COLUMN disabled_at TEXT;
ALTER TABLE webhooks ADD COLUMN enabled_at TEXT;

-- Update existing index to use created_at for per-endpoint ordering
CREATE INDEX IF NOT EXISTS idx_webhook_events_status_created_at ON webhook_events(status, created_at);

-- Add last_success_at and consecutive_failures to webhooks for auto-disable logic
ALTER TABLE webhooks ADD COLUMN last_success_at TEXT;
ALTER TABLE webhooks ADD COLUMN consecutive_failures INTEGER DEFAULT 0;
ALTER TABLE webhooks ADD COLUMN max_consecutive_failures INTEGER DEFAULT 5;

-- Create view for webhook delivery statistics
CREATE VIEW IF NOT EXISTS webhook_delivery_stats AS
SELECT 
    w.id AS webhook_id,
    w.url,
    COUNT(CASE WHEN dl.status = 'delivered' THEN 1 END) AS delivered_count,
    COUNT(CASE WHEN dl.status = 'failed' THEN 1 END) AS failed_count,
    COUNT(CASE WHEN dl.status = 'pending' THEN 1 END) AS pending_count,
    MAX(dl.created_at) AS last_delivery_attempt
FROM webhooks w
LEFT JOIN delivery_logs dl ON w.id = dl.webhook_id
GROUP BY w.id, w.url;