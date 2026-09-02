-- Create verified_assets table for asset issuer verification
-- Migration: 026_create_verified_assets.sql

CREATE TABLE IF NOT EXISTS verified_assets (
    id TEXT PRIMARY KEY,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    verification_status TEXT NOT NULL CHECK (verification_status IN ('verified', 'unverified', 'suspicious')),
    reputation_score REAL NOT NULL DEFAULT 0.0,

    -- Verification sources
    stellar_expert_verified BOOLEAN DEFAULT FALSE,
    stellar_toml_verified BOOLEAN DEFAULT FALSE,
    anchor_registry_verified BOOLEAN DEFAULT FALSE,

    -- Metrics
    trustline_count INTEGER DEFAULT 0,
    transaction_count INTEGER DEFAULT 0,
    total_volume_usd REAL DEFAULT 0.0,

    -- TOML data
    toml_home_domain TEXT,
    toml_name TEXT,
    toml_description TEXT,
    toml_org_name TEXT,
    toml_org_url TEXT,
    toml_logo_url TEXT,

    -- Community reports
    suspicious_reports_count INTEGER DEFAULT 0,
    last_suspicious_report_at TEXT,

    -- Verification metadata
    last_verified_at TEXT,
    verification_notes TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(asset_code, asset_issuer)
);

CREATE INDEX IF NOT EXISTS idx_verified_assets_status ON verified_assets(verification_status);
CREATE INDEX IF NOT EXISTS idx_verified_assets_reputation ON verified_assets(reputation_score DESC);
CREATE INDEX IF NOT EXISTS idx_verified_assets_asset_code ON verified_assets(asset_code);
CREATE INDEX IF NOT EXISTS idx_verified_assets_issuer ON verified_assets(asset_issuer);
CREATE INDEX IF NOT EXISTS idx_verified_assets_updated ON verified_assets(updated_at DESC);

-- Community reports table
CREATE TABLE IF NOT EXISTS asset_verification_reports (
    id TEXT PRIMARY KEY,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    reporter_account TEXT,
    report_type TEXT NOT NULL CHECK (report_type IN ('suspicious', 'scam', 'impersonation', 'other')),
    description TEXT NOT NULL,
    evidence_url TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'reviewed', 'resolved', 'dismissed')),
    reviewed_by TEXT,
    reviewed_at TEXT,
    resolution_notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (asset_code, asset_issuer) REFERENCES verified_assets(asset_code, asset_issuer) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_reports_asset ON asset_verification_reports(asset_code, asset_issuer);
CREATE INDEX IF NOT EXISTS idx_reports_status ON asset_verification_reports(status);
CREATE INDEX IF NOT EXISTS idx_reports_created ON asset_verification_reports(created_at DESC);

-- Audit trail table
CREATE TABLE IF NOT EXISTS asset_verification_history (
    id TEXT PRIMARY KEY,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    previous_status TEXT,
    new_status TEXT NOT NULL,
    previous_reputation_score REAL,
    new_reputation_score REAL NOT NULL,
    change_reason TEXT,
    changed_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (asset_code, asset_issuer) REFERENCES verified_assets(asset_code, asset_issuer) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_verification_history_asset ON asset_verification_history(asset_code, asset_issuer);
CREATE INDEX IF NOT EXISTS idx_verification_history_created ON asset_verification_history(created_at DESC);
