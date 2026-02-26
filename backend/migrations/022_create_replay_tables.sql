-- Migration: Create Replay System Tables
-- Description: Tables for contract event replay, checkpoints, and state management

-- Contract events storage
CREATE TABLE IF NOT EXISTS contract_events (
    id TEXT PRIMARY KEY,
    ledger_sequence INTEGER NOT NULL,
    transaction_hash TEXT NOT NULL,
    contract_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON-encoded event data
    timestamp TIMESTAMP NOT NULL,
    network TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(ledger_sequence, transaction_hash, event_type)
);

CREATE INDEX IF NOT EXISTS idx_contract_events_ledger ON contract_events(ledger_sequence);
CREATE INDEX IF NOT EXISTS idx_contract_events_contract ON contract_events(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_events_type ON contract_events(event_type);
CREATE INDEX IF NOT EXISTS idx_contract_events_network ON contract_events(network);
CREATE INDEX IF NOT EXISTS idx_contract_events_timestamp ON contract_events(timestamp);

-- Replay sessions
CREATE TABLE IF NOT EXISTS replay_sessions (
    session_id TEXT PRIMARY KEY,
    config TEXT NOT NULL, -- JSON-encoded ReplayConfig
    status TEXT NOT NULL, -- JSON-encoded ReplayStatus
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP,
    checkpoint TEXT, -- JSON-encoded Checkpoint (optional)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_replay_sessions_started ON replay_sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_replay_sessions_status ON replay_sessions(status);

-- Replay checkpoints
CREATE TABLE IF NOT EXISTS replay_checkpoints (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    last_ledger INTEGER NOT NULL,
    events_processed INTEGER NOT NULL DEFAULT 0,
    events_failed INTEGER NOT NULL DEFAULT 0,
    state_snapshot TEXT NOT NULL, -- JSON-encoded state
    metadata TEXT NOT NULL, -- JSON-encoded metadata
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (session_id) REFERENCES replay_sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_replay_checkpoints_session ON replay_checkpoints(session_id);
CREATE INDEX IF NOT EXISTS idx_replay_checkpoints_ledger ON replay_checkpoints(last_ledger);
CREATE INDEX IF NOT EXISTS idx_replay_checkpoints_created ON replay_checkpoints(created_at DESC);

-- Replay state snapshots
CREATE TABLE IF NOT EXISTS replay_state (
    ledger INTEGER PRIMARY KEY,
    state_json TEXT NOT NULL, -- JSON-encoded ApplicationState
    state_hash TEXT NOT NULL, -- SHA-256 hash for verification
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_replay_state_hash ON replay_state(state_hash);
CREATE INDEX IF NOT EXISTS idx_replay_state_updated ON replay_state(updated_at DESC);

-- Processed events tracking (for idempotency)
CREATE TABLE IF NOT EXISTS processed_events (
    event_id TEXT PRIMARY KEY,
    ledger_sequence INTEGER NOT NULL,
    processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_processed_events_ledger ON processed_events(ledger_sequence);
CREATE INDEX IF NOT EXISTS idx_processed_events_processed ON processed_events(processed_at);

-- Snapshots table (if not exists from previous migrations)
CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epoch INTEGER NOT NULL UNIQUE,
    hash TEXT NOT NULL,
    ledger_sequence INTEGER,
    transaction_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_snapshots_epoch ON snapshots(epoch);
CREATE INDEX IF NOT EXISTS idx_snapshots_ledger ON snapshots(ledger_sequence);

-- Add comments for documentation
-- contract_events: Stores all contract events from the blockchain for replay
-- replay_sessions: Tracks replay operations with configuration and status
-- replay_checkpoints: Saves progress checkpoints for resume capability
-- replay_state: Stores rebuilt application state at specific ledgers
-- processed_events: Tracks processed events for idempotency guarantees
