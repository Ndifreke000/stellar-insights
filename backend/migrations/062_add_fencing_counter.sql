-- Monotonically increasing fencing tokens per leader role (SQLite).
-- `role` is the primary key, so no separate index is needed.
CREATE TABLE IF NOT EXISTS fence_counter (
    role TEXT PRIMARY KEY,
    counter INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
