CREATE TABLE pending_transactions (
    id TEXT PRIMARY KEY,
    source_account TEXT NOT NULL,
    xdr TEXT NOT NULL,
    required_signatures INTEGER NOT NULL,
    status TEXT NOT NULL, -- 'pending', 'ready', 'submitted'
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE transaction_signatures (
    id TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL REFERENCES pending_transactions(id) ON DELETE CASCADE,
    signer TEXT NOT NULL,
    signature TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(transaction_id, signer)
);
