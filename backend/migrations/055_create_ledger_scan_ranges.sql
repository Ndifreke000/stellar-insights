-- Ledger scan coverage, tracked separately from contract events (#2765).
--
-- Gap detection used to derive coverage from `SELECT DISTINCT ledger FROM
-- contract_events`, so every ledger that emitted no event for the watched
-- contracts was reported as a gap and re-fetched by the backfill job. The
-- indexer now records every successfully scanned batch here as an inclusive
-- [start_ledger, end_ledger] range, merging adjacent/overlapping ranges, and
-- gap detection is a range difference over this table.
CREATE TABLE IF NOT EXISTS ledger_scan_ranges (
    network TEXT NOT NULL,
    start_ledger INTEGER NOT NULL,
    end_ledger INTEGER NOT NULL,
    scanned_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (network, start_ledger),
    CHECK (end_ledger >= start_ledger)
);

CREATE INDEX IF NOT EXISTS idx_ledger_scan_ranges_end
    ON ledger_scan_ranges(network, end_ledger);

-- One-off backfill for existing deployments: every ledger already present in
-- the ledger-ingestion history (`ledgers`, tagged by network since 052) was
-- scanned. Collapse consecutive sequences into ranges (gaps-and-islands).
INSERT OR IGNORE INTO ledger_scan_ranges (network, start_ledger, end_ledger, scanned_at)
SELECT network, MIN(sequence), MAX(sequence), datetime('now')
FROM (
    SELECT network,
           sequence,
           sequence - ROW_NUMBER() OVER (PARTITION BY network ORDER BY sequence) AS grp
    FROM ledgers
)
GROUP BY network, grp;
