-- Migration: Create vault_audit_log table for Vault secret access tracking
-- This table logs all Vault operations for security and compliance

CREATE TABLE IF NOT EXISTS vault_audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    operation VARCHAR(50) NOT NULL,
    resource VARCHAR(255) NOT NULL,
    user_id TEXT,
    status VARCHAR(20) NOT NULL,
    details JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_vault_audit_timestamp ON vault_audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_vault_audit_operation ON vault_audit_log(operation);
CREATE INDEX IF NOT EXISTS idx_vault_audit_resource ON vault_audit_log(resource);
CREATE INDEX IF NOT EXISTS idx_vault_audit_user_id ON vault_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_vault_audit_status ON vault_audit_log(status);

-- Create view for audit log summary
DROP VIEW IF EXISTS vault_audit_summary;
CREATE VIEW vault_audit_summary AS
SELECT 
    DATE(timestamp) as date,
    operation,
    COUNT(*) as total_operations,
    COUNT(CASE WHEN status = 'success' THEN 1 END) as successful,
    COUNT(CASE WHEN status = 'failure' THEN 1 END) as failed
FROM vault_audit_log
GROUP BY DATE(timestamp), operation
ORDER BY date DESC, operation;
