-- Durable audit trail for transaction signatures (#issue: KMS signer).
--
-- Every attempt to sign a Soroban transaction — whether it succeeded, was
-- rejected by the envelope policy before the signer was called, or failed
-- inside the KMS — gets one row here, in addition to the always-on
-- `signature_audit` tracing events.
--
-- `tx_hash` is the hex-encoded 32-byte Stellar transaction signature base
-- (SHA256 of the TransactionSignaturePayload XDR), not the transaction
-- hash. `signer_backend`/`signer_key_id` identify which TransactionSigner
-- backend and key produced (or refused) the signature.
CREATE TABLE IF NOT EXISTS transaction_signature_audit (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    signer_backend TEXT NOT NULL,
    signer_key_id TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    contract_id TEXT NOT NULL,
    function_name TEXT NOT NULL,
    epoch INTEGER,
    status TEXT NOT NULL,
    error TEXT,
    latency_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_signature_audit_created
    ON transaction_signature_audit(created_at);

CREATE INDEX IF NOT EXISTS idx_signature_audit_tx_hash
    ON transaction_signature_audit(tx_hash);
