-- Claimable balances for escrow, scheduled payments, and airdrops
CREATE TABLE IF NOT EXISTS claimable_balances (
    id TEXT PRIMARY KEY,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT,
    amount TEXT NOT NULL,
    sponsor TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT,
    claimed BOOLEAN NOT NULL DEFAULT 0,
    claimed_at TEXT,
    claimed_by TEXT,
    last_modified_ledger INTEGER,
    paging_token TEXT
);

-- Claimants for each claimable balance
CREATE TABLE IF NOT EXISTS claimable_balance_claimants (
    balance_id TEXT NOT NULL,
    destination TEXT NOT NULL,
    predicate TEXT NOT NULL,
    PRIMARY KEY (balance_id, destination),
    FOREIGN KEY (balance_id) REFERENCES claimable_balances(id)
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_claimable_balances_expires_at ON claimable_balances(expires_at);
CREATE INDEX IF NOT EXISTS idx_claimable_balances_claimed ON claimable_balances(claimed);
CREATE INDEX IF NOT EXISTS idx_claimable_balances_asset ON claimable_balances(asset_code, asset_issuer);
CREATE INDEX IF NOT EXISTS idx_claimable_balances_sponsor ON claimable_balances(sponsor);
CREATE INDEX IF NOT EXISTS idx_claimable_balance_claimants_balance ON claimable_balance_claimants(balance_id);
