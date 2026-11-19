-- Reorg / rollback safety for aggregation (#2771).
--
-- aggregation_watermarks: per bucket kind and network, the close time of the
-- newest ledger that is at least AGGREGATION_FINALITY_DEPTH ledgers behind
-- head. Buckets ending at or before the watermark are final; newer buckets
-- are provisional and are never included in snapshots.
CREATE TABLE IF NOT EXISTS aggregation_watermarks (
    bucket_kind TEXT NOT NULL,
    network TEXT NOT NULL DEFAULT 'mainnet',
    watermark TEXT NOT NULL,
    finalized_ledger INTEGER NOT NULL,
    finality_depth INTEGER NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (bucket_kind, network)
);

-- Buckets to recompute after an upstream correction rewrote a final ledger.
CREATE TABLE IF NOT EXISTS aggregation_recompute_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_kind TEXT NOT NULL,
    bucket_start TEXT NOT NULL,
    reason TEXT NOT NULL,
    enqueued_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TEXT
);

-- At most one pending job per bucket.
CREATE UNIQUE INDEX IF NOT EXISTS idx_recompute_queue_pending
    ON aggregation_recompute_queue (bucket_kind, bucket_start)
    WHERE processed_at IS NULL;
