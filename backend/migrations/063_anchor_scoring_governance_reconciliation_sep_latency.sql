-- Tables for Anchor health scoring, governance reconciliation, and SEP settlement latency (#2814, #2816, #2822)

-- 1. Anchor health scores with transparent versioned methodology (#2816)
CREATE TABLE IF NOT EXISTS anchor_health_scores (
    id TEXT PRIMARY KEY,
    anchor_id TEXT NOT NULL,
    methodology_version TEXT NOT NULL,
    composite_score REAL NOT NULL,
    availability_score REAL NOT NULL,
    success_rate_score REAL NOT NULL,
    latency_score REAL NOT NULL,
    toml_validity_score REAL NOT NULL,
    asset_verification_score REAL NOT NULL,
    status TEXT NOT NULL,
    details_json TEXT NOT NULL,
    computed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anchor_id) REFERENCES anchors(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_anchor_health_scores_anchor 
ON anchor_health_scores(anchor_id, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_anchor_health_scores_version 
ON anchor_health_scores(methodology_version);

-- 2. Governance chain events and reconciliation discrepancies (#2822)
CREATE TABLE IF NOT EXISTS governance_chain_events (
    id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    voter_address TEXT,
    choice TEXT,
    weight INTEGER NOT NULL DEFAULT 1,
    ledger_sequence INTEGER NOT NULL,
    tx_hash TEXT,
    recorded_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (proposal_id) REFERENCES governance_proposals(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_governance_chain_events_proposal 
ON governance_chain_events(proposal_id, recorded_at ASC);

CREATE TABLE IF NOT EXISTS governance_reconciliation_discrepancies (
    id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL,
    db_votes_for INTEGER NOT NULL,
    chain_votes_for INTEGER NOT NULL,
    db_votes_against INTEGER NOT NULL,
    chain_votes_against INTEGER NOT NULL,
    db_votes_abstain INTEGER NOT NULL DEFAULT 0,
    chain_votes_abstain INTEGER NOT NULL DEFAULT 0,
    discrepancy_type TEXT NOT NULL,
    details TEXT,
    detected_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    resolved BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (proposal_id) REFERENCES governance_proposals(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_gov_reconciliation_status 
ON governance_reconciliation_discrepancies(resolved, detected_at DESC);

-- 3. End-to-end SEP-24/31 stage latencies (#2814)
CREATE TABLE IF NOT EXISTS anchor_stage_latencies (
    id TEXT PRIMARY KEY,
    anchor_id TEXT NOT NULL,
    transaction_id TEXT NOT NULL,
    sep_standard TEXT NOT NULL,
    user_transfer_ms INTEGER,
    on_chain_ms INTEGER,
    anchor_processing_ms INTEGER,
    external_payout_ms INTEGER,
    total_latency_ms INTEGER NOT NULL,
    status TEXT NOT NULL,
    recorded_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (anchor_id) REFERENCES anchors(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_anchor_stage_latencies_anchor 
ON anchor_stage_latencies(anchor_id, recorded_at DESC);
