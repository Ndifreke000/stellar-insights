-- Add a network discriminator to the ledger ingestion tables.
--
-- MultiNetworkConfig / MultiNetworkRpcManager (src/multi_network.rs) support
-- running ingestion against mainnet and testnet at the same time, but the
-- ledger ingestion tables (created in 007_create_ledger_ingestion_tables.sql)
-- had no network column at all. Ingesting both networks into the same
-- database silently mixed their rows together (see issue: "Multi-network:
-- ensure testnet and mainnet data never mix").
--
-- This migration tags every ledger-ingestion row with its network so reads
-- can filter correctly, and gives each network its own ingestion cursor so
-- resuming one network's ingestion can't clobber the other's progress.
ALTER TABLE ledgers ADD COLUMN network TEXT NOT NULL DEFAULT 'mainnet';
ALTER TABLE transactions ADD COLUMN network TEXT NOT NULL DEFAULT 'mainnet';
ALTER TABLE ledger_payments ADD COLUMN network TEXT NOT NULL DEFAULT 'mainnet';
ALTER TABLE ingestion_cursor ADD COLUMN network TEXT NOT NULL DEFAULT 'mainnet';

CREATE INDEX IF NOT EXISTS idx_ledgers_network ON ledgers(network);
CREATE INDEX IF NOT EXISTS idx_transactions_network ON transactions(network);
CREATE INDEX IF NOT EXISTS idx_ledger_payments_network ON ledger_payments(network);

-- One cursor row per network; the pre-existing id=1 row keeps tracking
-- mainnet (its default) so DataIngestionService::get_ingestion_status,
-- which still reads `WHERE id = 1`, keeps working unchanged.
CREATE UNIQUE INDEX IF NOT EXISTS idx_ingestion_cursor_network ON ingestion_cursor(network);
