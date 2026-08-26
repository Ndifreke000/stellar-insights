-- Migration: Enhance admin_audit_log with session/device/IP context and event type
-- Add new columns for richer audit trail
ALTER TABLE admin_audit_log ADD COLUMN session_id TEXT;
ALTER TABLE admin_audit_log ADD COLUMN device_user_agent TEXT;
ALTER TABLE admin_audit_log ADD COLUMN ip_address TEXT;
ALTER TABLE admin_audit_log ADD COLUMN event_type VARCHAR(50);

-- Create indexes for new columns
CREATE INDEX IF NOT EXISTS idx_admin_audit_session_id ON admin_audit_log(session_id);
CREATE INDEX IF NOT EXISTS idx_admin_audit_ip_address ON admin_audit_log(ip_address);
CREATE INDEX IF NOT EXISTS idx_admin_audit_event_type ON admin_audit_log(event_type);
