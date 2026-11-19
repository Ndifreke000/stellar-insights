-- Make contract event indexing idempotent under concurrent workers (#2769).
--
-- 1. Rebuild contract_events with an identity key of
--    (network, ledger, transaction_hash, op_index, event_index). The old
--    UNIQUE(ledger, transaction_hash, event_type) constraint from 022 would
--    silently drop distinct events of the same type emitted by one tx once
--    writes switch to ON CONFLICT DO NOTHING, so it is removed. SQLite can't
--    drop a table constraint in place, hence the rebuild.
-- 2. Deduplicate rows that the previous INSERT OR REPLACE path (plus random
--    backfill ids) could have produced, keeping the earliest row.
-- 3. Add indexer_lease for leader fencing tokens.

CREATE TABLE contract_events_new (
    id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    epoch INTEGER,
    hash TEXT,
    timestamp INTEGER,
    timestamp_dt TIMESTAMP,
    ledger INTEGER NOT NULL,
    transaction_hash TEXT NOT NULL,
    data TEXT,
    network TEXT,
    verification_status TEXT DEFAULT 'pending',
    verified_at TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    op_index INTEGER NOT NULL DEFAULT 0,
    event_index INTEGER NOT NULL DEFAULT 0
);

-- Existing rows carry no op/event index, so number duplicates within a
-- (network, ledger, tx, event_type) group 0..n to keep distinct events and
-- only collapse exact retries (same contract/type/epoch/hash).
INSERT INTO contract_events_new (
    id, contract_id, event_type, epoch, hash, timestamp, timestamp_dt, ledger,
    transaction_hash, data, network, verification_status, verified_at,
    created_at, op_index, event_index
)
SELECT id, contract_id, event_type, epoch, hash, timestamp, timestamp_dt, ledger,
       transaction_hash, data, network, verification_status, verified_at,
       created_at, 0,
       (SELECT COUNT(*) FROM contract_events o
         WHERE IFNULL(o.network, '') = IFNULL(c.network, '')
           AND o.ledger = c.ledger
           AND o.transaction_hash = c.transaction_hash
           AND o.rowid < c.rowid)
FROM contract_events c
WHERE c.rowid IN (
    SELECT MIN(rowid) FROM contract_events
    GROUP BY IFNULL(network, ''), ledger, transaction_hash, contract_id,
             event_type, IFNULL(epoch, -1), IFNULL(hash, '')
);

DROP TABLE contract_events;
ALTER TABLE contract_events_new RENAME TO contract_events;

CREATE UNIQUE INDEX IF NOT EXISTS idx_contract_events_identity
    ON contract_events(IFNULL(network, ''), ledger, transaction_hash, op_index, event_index);
CREATE INDEX IF NOT EXISTS idx_contract_events_ledger ON contract_events(ledger DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_contract ON contract_events(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_events_type ON contract_events(event_type);
CREATE INDEX IF NOT EXISTS idx_contract_events_epoch ON contract_events(epoch DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_verification_status ON contract_events(verification_status);
CREATE INDEX IF NOT EXISTS idx_contract_events_timestamp ON contract_events(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_created_at ON contract_events(created_at DESC);

CREATE TABLE IF NOT EXISTS indexer_lease (
    role TEXT PRIMARY KEY,
    token INTEGER NOT NULL,
    holder TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
