-- Muxed account handling in corridor/anchor attribution.
--
-- `source_account`/`destination` on ledger_payments can be M-addresses
-- (SEP-0023 muxed accounts). Attribution to an anchor must key off the
-- underlying G-address, not the muxed address, or payments to a given
-- anchor's sub-accounts silently fail to attribute (see muxed.rs, which
-- already existed but was not used anywhere in the ingestion path).
--
-- These columns hold the base (G-address) form of source/destination,
-- resolved at ingestion time via muxed::parse_muxed_address so downstream
-- attribution/analytics can join on a single consistent account identity.
ALTER TABLE ledger_payments ADD COLUMN source_base_account TEXT;
ALTER TABLE ledger_payments ADD COLUMN destination_base_account TEXT;

CREATE INDEX IF NOT EXISTS idx_ledger_payments_source_base_account ON ledger_payments(source_base_account);
CREATE INDEX IF NOT EXISTS idx_ledger_payments_destination_base_account ON ledger_payments(destination_base_account);
