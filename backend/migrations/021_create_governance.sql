-- Governance proposals table
CREATE TABLE IF NOT EXISTS governance_proposals (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    proposal_type TEXT DEFAULT 'contract_upgrade',
    target_contract TEXT,
    new_wasm_hash TEXT,
    status TEXT DEFAULT 'draft',
    created_by TEXT NOT NULL,
    on_chain_id INTEGER,
    voting_ends_at DATETIME,
    finalized_at DATETIME,
    executed_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Governance votes table
CREATE TABLE IF NOT EXISTS governance_votes (
    id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL,
    voter_address TEXT NOT NULL,
    choice TEXT NOT NULL,
    tx_hash TEXT,
    voted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (proposal_id) REFERENCES governance_proposals(id),
    UNIQUE(proposal_id, voter_address)
);

-- Governance comments table
CREATE TABLE IF NOT EXISTS governance_comments (
    id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL,
    author_address TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (proposal_id) REFERENCES governance_proposals(id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_governance_proposals_status ON governance_proposals(status);
CREATE INDEX IF NOT EXISTS idx_governance_proposals_created_by ON governance_proposals(created_by);
CREATE INDEX IF NOT EXISTS idx_governance_votes_proposal_id ON governance_votes(proposal_id);
CREATE INDEX IF NOT EXISTS idx_governance_votes_voter_address ON governance_votes(voter_address);
CREATE INDEX IF NOT EXISTS idx_governance_comments_proposal_id ON governance_comments(proposal_id);
