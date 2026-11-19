-- The deduplicated table keeps its rows; only the identity index and the
-- fencing table added by 056 are removed.
DROP INDEX IF EXISTS idx_contract_events_identity;
DROP TABLE IF EXISTS indexer_lease;
