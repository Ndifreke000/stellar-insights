-- Trustline events for cohort tracking
CREATE TABLE IF NOT EXISTS trustline_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    event_type TEXT NOT NULL,
    balance REAL NOT NULL DEFAULT 0.0,
    recorded_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_trustline_events_asset_account 
ON trustline_events(asset_code, asset_issuer, account_id, recorded_at);

-- Trustline spam accounts
CREATE TABLE IF NOT EXISTS trustline_spam_accounts (
    account_id TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    flagged_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Unique index to prevent double claims on verification rewards per epoch and user
CREATE UNIQUE INDEX IF NOT EXISTS idx_snapshot_verifications_epoch_user 
ON snapshot_verifications(epoch, user_id);

-- Sybil clustering tables
CREATE TABLE IF NOT EXISTS verifier_account_clusters (
    user_id TEXT PRIMARY KEY,
    cluster_id TEXT NOT NULL,
    funding_account TEXT,
    risk_score REAL NOT NULL DEFAULT 0.0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_verifier_clusters_cluster 
ON verifier_account_clusters(cluster_id);

CREATE TABLE IF NOT EXISTS cluster_reward_caps (
    cluster_id TEXT PRIMARY KEY,
    daily_reward_cap INTEGER NOT NULL DEFAULT 100,
    current_day_rewards INTEGER NOT NULL DEFAULT 0,
    last_reset_date TEXT NOT NULL DEFAULT (DATE('now'))
);

CREATE TABLE IF NOT EXISTS verification_review_queue (
    id TEXT PRIMARY KEY,
    verification_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    epoch INTEGER NOT NULL,
    cluster_id TEXT,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at DATETIME,
    reviewed_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_verification_review_status 
ON verification_review_queue(status, created_at DESC);
