-- Asset Impersonation Detection tables
-- Adds canonical issuer allowlist, impersonation reports, and audit logs

-- Canonical issuer allowlist per asset code
CREATE TABLE IF NOT EXISTS canonical_issuers (
    id TEXT PRIMARY KEY,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'admin',  -- 'admin', 'stellar_expert', 'anchor_directory'
    is_verified INTEGER NOT NULL DEFAULT 0,
    verified_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(asset_code, asset_issuer)
);

-- Asset impersonation reports
CREATE TABLE IF NOT EXISTS asset_impersonation_reports (
    id TEXT PRIMARY KEY,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    report_type TEXT NOT NULL,  -- same_code_different_issuer, lookalike_domain, homoglyph_domain, punycode_domain
    risk_level TEXT NOT NULL,   -- low, medium, high, critical
    details TEXT NOT NULL,
    evidence TEXT,              -- JSON array of evidence strings
    canonical_issuers TEXT,     -- JSON array of canonical issuer accounts
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, reviewed, dismissed
    reviewed_by TEXT,
    reviewed_at TEXT,
    resolution_notes TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for impersonation reports
CREATE INDEX IF NOT EXISTS idx_impersonation_reports_asset_code ON asset_impersonation_reports(asset_code);
CREATE INDEX IF NOT EXISTS idx_impersonation_reports_asset_issuer ON asset_impersonation_reports(asset_issuer);
CREATE INDEX IF NOT EXISTS idx_impersonation_reports_status ON asset_impersonation_reports(status);
CREATE INDEX IF NOT EXISTS idx_impersonation_reports_risk_level ON asset_impersonation_reports(risk_level);

-- Audit log for admin overrides
CREATE TABLE IF NOT EXISTS asset_verification_audit_log (
    id TEXT PRIMARY KEY,
    action TEXT NOT NULL,       -- add_canonical_issuer, remove_canonical_issuer, override_risk
    asset_code TEXT,
    asset_issuer TEXT,
    actor_account TEXT,
    details TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for audit log
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON asset_verification_audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_asset_code ON asset_verification_audit_log(asset_code);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor ON asset_verification_audit_log(actor_account);