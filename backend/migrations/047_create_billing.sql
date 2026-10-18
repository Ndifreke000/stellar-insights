-- Paid API key tier, settled in USDC on Stellar.
--
-- An invoice asks the key's owner to send `amount_usdc` of USDC to the
-- treasury with `memo` as the text memo. Once a matching transaction is
-- verified on-chain the invoice is marked paid and the key's subscription is
-- extended. `transaction_hash` is unique so one payment can settle only one
-- invoice.
CREATE TABLE IF NOT EXISTS billing_invoices (
    id TEXT PRIMARY KEY NOT NULL,
    api_key_id TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    plan TEXT NOT NULL,
    amount_usdc TEXT NOT NULL,
    asset_code TEXT NOT NULL,
    asset_issuer TEXT NOT NULL,
    destination TEXT NOT NULL,
    memo TEXT NOT NULL UNIQUE,
    limit_per_minute INTEGER NOT NULL,
    period_days INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    transaction_hash TEXT UNIQUE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    paid_at TEXT,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_billing_invoices_api_key_id ON billing_invoices (api_key_id);
CREATE INDEX IF NOT EXISTS idx_billing_invoices_wallet ON billing_invoices (wallet_address);

-- The paid tier currently in force for a key. The rate limiter applies
-- `limit_per_minute` while `paid_until` is in the future.
CREATE TABLE IF NOT EXISTS api_key_subscriptions (
    api_key_id TEXT PRIMARY KEY NOT NULL,
    plan TEXT NOT NULL,
    limit_per_minute INTEGER NOT NULL,
    paid_until TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE
);
