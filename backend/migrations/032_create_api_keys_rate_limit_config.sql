CREATE TABLE IF NOT EXISTS api_keys_rate_limit_config (
    api_key_id TEXT PRIMARY KEY NOT NULL,
    limit_per_minute INTEGER NOT NULL DEFAULT 60,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_api_keys_rate_limit_config_api_key_id
    ON api_keys_rate_limit_config (api_key_id);
