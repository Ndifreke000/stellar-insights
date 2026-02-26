-- Create contract_events table for indexing Soroban contract events
CREATE TABLE IF NOT EXISTS contract_events (
    id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    epoch INTEGER,
    hash TEXT,
    timestamp INTEGER,
    ledger INTEGER NOT NULL,
    transaction_hash TEXT NOT NULL,
    verification_status TEXT DEFAULT 'pending',
    verified_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Add verification_status column to snapshots table if it doesn't exist
ALTER TABLE snapshots ADD COLUMN verification_status TEXT DEFAULT 'pending';
ALTER TABLE snapshots ADD COLUMN verified_at TEXT;

-- Create indexes for contract_events table
CREATE INDEX IF NOT EXISTS idx_contract_events_created_at ON contract_events(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_ledger ON contract_events(ledger DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_epoch ON contract_events(epoch DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_contract_id ON contract_events(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_events_verification_status ON contract_events(verification_status);
CREATE INDEX IF NOT EXISTS idx_contract_events_event_type ON contract_events(event_type);
CREATE INDEX IF NOT EXISTS idx_contract_events_transaction_hash ON contract_events(transaction_hash);

-- Create indexes for snapshots table verification columns
CREATE INDEX IF NOT EXISTS idx_snapshots_verification_status ON snapshots(verification_status);
CREATE INDEX IF NOT EXISTS idx_snapshots_verified_at ON snapshots(verified_at);
CREATE INDEX IF NOT EXISTS idx_snapshots_epoch ON snapshots(epoch DESC);
