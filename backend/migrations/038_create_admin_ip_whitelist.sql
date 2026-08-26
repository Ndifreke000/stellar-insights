-- Migration: Create configurable admin IP whitelist table
CREATE TABLE IF NOT EXISTS admin_ip_whitelist (
    id TEXT PRIMARY KEY,
    ip_or_cidr TEXT NOT NULL UNIQUE,
    description TEXT,
    added_by_user_id TEXT,
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (added_by_user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_admin_ip_whitelist_ip ON admin_ip_whitelist(ip_or_cidr);

-- Fail-closed default: if table is empty, deny all admin access (enforced in middleware)
