-- Push notification device tokens
-- Migration: 051_create_device_tokens.sql

CREATE TABLE IF NOT EXISTS device_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    account TEXT NOT NULL,
    device_token TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL DEFAULT 'fcm',
    device_label TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_device_tokens_account ON device_tokens(account);
CREATE INDEX IF NOT EXISTS idx_device_tokens_token ON device_tokens(device_token);
