-- Fee-bumped transactions should be attributed to the inner (originating) source,
-- not the outer fee-bump account that merely paid the fee. Add a dedicated column
-- so callers that need "who actually initiated this transaction" (e.g. success-rate
-- and corridor attribution) don't have to reuse fee_source, which only tracks the
-- fee payer and can legitimately differ from the transaction's real originator.
ALTER TABLE fee_bump_transactions ADD COLUMN inner_source TEXT NOT NULL DEFAULT '';

-- Create index on inner_source for attribution lookups/analytics
CREATE INDEX IF NOT EXISTS idx_fee_bump_inner_source ON fee_bump_transactions(inner_source);
