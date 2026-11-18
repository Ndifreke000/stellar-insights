-- Per-ledger header data captured at ingestion time (#2768).
--
-- Replay needs the protocol version that was active when each ledger
-- closed (event decoding rules change across upgrades, e.g. CAP-67 in
-- protocol 23). `ContractEvent` does not carry it, so ingestion stores the
-- `ledger_version` decoded from the RPC `headerXdr` here, keyed by network.
--
-- Existing deployments: run `LedgerHeaderStore::backfill_protocol_versions`
-- (re-fetches headers from RPC for already-ingested ledgers). Ledgers outside
-- RPC retention stay missing and replay over them fails with an explicit
-- MissingLedgerHeader error rather than guessing a version.
CREATE TABLE IF NOT EXISTS ledger_headers (
    network TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    protocol_version INTEGER NOT NULL,
    hash TEXT NOT NULL,
    close_time TEXT NOT NULL,
    ingested_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (network, sequence)
);

CREATE INDEX IF NOT EXISTS idx_ledger_headers_protocol
    ON ledger_headers(network, protocol_version);
