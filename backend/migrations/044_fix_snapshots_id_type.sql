-- Fix snapshots.id: it was declared INTEGER PRIMARY KEY AUTOINCREMENT, but
-- every write path (database.rs, db/metrics.rs) generates a UUID string
-- (Uuid::new_v4().to_string()) and binds it as the id -- which SQLite
-- rejects with a datatype mismatch, since INTEGER PRIMARY KEY is a strict
-- rowid alias. snapshot_verifications.snapshot_id (migration 013) already
-- declares itself TEXT NOT NULL referencing snapshots(id), confirming TEXT
-- was always the intended type here; the original CREATE TABLE was wrong.
--
-- SQLite has no ALTER COLUMN, so this recreates the table with the correct
-- type and copies existing rows across.

CREATE TABLE snapshots_new (
    id TEXT PRIMARY KEY,
    entity_id TEXT,
    entity_type TEXT,
    data TEXT,
    hash TEXT,
    epoch INTEGER UNIQUE,
    ledger_sequence INTEGER,
    transaction_hash TEXT,
    snapshot_time TIMESTAMP,
    verification_status TEXT DEFAULT 'pending',
    verified_at TEXT,
    timestamp TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO snapshots_new
SELECT CAST(id AS TEXT), entity_id, entity_type, data, hash, epoch,
       ledger_sequence, transaction_hash, snapshot_time,
       verification_status, verified_at, timestamp, created_at
FROM snapshots;

DROP TABLE snapshots;
ALTER TABLE snapshots_new RENAME TO snapshots;

CREATE INDEX idx_snapshots_entity ON snapshots(entity_id, entity_type);
CREATE INDEX idx_snapshots_timestamp ON snapshots(timestamp DESC);
CREATE INDEX idx_snapshots_snapshot_time ON snapshots(snapshot_time DESC);
CREATE INDEX idx_snapshots_epoch_desc ON snapshots(epoch DESC);
CREATE INDEX idx_snapshots_ledger ON snapshots(ledger_sequence);
CREATE INDEX idx_snapshots_verification_status ON snapshots(verification_status);
CREATE INDEX idx_snapshots_verified_at ON snapshots(verified_at);
