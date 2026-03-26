-- Migration: Create vault_audit_log table for Vault secret access tracking
-- This table logs all Vault operations for security and compliance

CREATE TABLE IF NOT EXISTS vault_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    operation TEXT NOT NULL,
    resource TEXT NOT NULL,
    user_id TEXT,
    status TEXT NOT NULL,
    details TEXT, -- JSON stored as TEXT for SQLite compatibility
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_vault_audit_timestamp ON vault_audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_vault_audit_operation ON vault_audit_log(operation);
CREATE INDEX IF NOT EXISTS idx_vault_audit_resource ON vault_audit_log(resource);
CREATE INDEX IF NOT EXISTS idx_vault_audit_user_id ON vault_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_vault_audit_status ON vault_audit_log(status);

-- Summary view for audit log reporting
CREATE VIEW IF NOT EXISTS vault_audit_summary AS
SELECT
    DATE(timestamp) AS date,
    operation,
    COUNT(*) AS total_operations,
    COUNT(CASE WHEN status = 'success' THEN 1 END) AS successful,
    COUNT(CASE WHEN status = 'failure' THEN 1 END) AS failed
FROM vault_audit_log
GROUP BY DATE(timestamp), operation
ORDER BY date DESC, operation;
