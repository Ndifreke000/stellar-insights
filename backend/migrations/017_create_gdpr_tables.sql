-- GDPR Compliance Tables
-- Migration: 017_create_gdpr_tables.sql

-- User consent tracking table
CREATE TABLE IF NOT EXISTS user_consents (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    consent_type TEXT NOT NULL,
    consent_given BOOLEAN NOT NULL DEFAULT FALSE,
    consent_version TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    granted_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_user_consents_user_id ON user_consents(user_id);
CREATE INDEX IF NOT EXISTS idx_user_consents_type ON user_consents(consent_type);

-- Data export requests table (Right to Access/Portability)
CREATE TABLE IF NOT EXISTS data_export_requests (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    requested_data_types TEXT NOT NULL,
    export_format TEXT NOT NULL DEFAULT 'json',
    requested_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    expires_at TEXT,
    download_token TEXT UNIQUE,
    file_path TEXT,
    error_message TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_data_export_requests_user_id ON data_export_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_data_export_requests_status ON data_export_requests(status);
CREATE INDEX IF NOT EXISTS idx_data_export_requests_download_token ON data_export_requests(download_token);

-- Data deletion requests table (Right to be Forgotten)
CREATE TABLE IF NOT EXISTS data_deletion_requests (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reason TEXT,
    delete_all_data BOOLEAN NOT NULL DEFAULT TRUE,
    data_types_to_delete TEXT,
    requested_at TEXT NOT NULL DEFAULT (datetime('now')),
    scheduled_deletion_at TEXT,
    completed_at TEXT,
    cancelled_at TEXT,
    error_message TEXT,
    confirmation_token TEXT UNIQUE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_data_deletion_requests_user_id ON data_deletion_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_data_deletion_requests_status ON data_deletion_requests(status);
CREATE INDEX IF NOT EXISTS idx_data_deletion_requests_confirmation_token ON data_deletion_requests(confirmation_token);

-- Consent history/audit log
CREATE TABLE IF NOT EXISTS consent_audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    consent_type TEXT NOT NULL,
    action TEXT NOT NULL,
    old_value BOOLEAN,
    new_value BOOLEAN,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_consent_audit_log_user_id ON consent_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_consent_audit_log_created_at ON consent_audit_log(created_at);

-- Data processing activities log
CREATE TABLE IF NOT EXISTS data_processing_log (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    activity_type TEXT NOT NULL,
    data_category TEXT NOT NULL,
    purpose TEXT,
    legal_basis TEXT,
    processed_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_data_processing_log_user_id ON data_processing_log(user_id);
CREATE INDEX IF NOT EXISTS idx_data_processing_log_processed_at ON data_processing_log(processed_at);
