-- Migration: Create Request Signing Nonce Table
-- Description: Track nonces for request signing replay protection

CREATE TABLE IF NOT EXISTS request_signing_nonces (
    nonce TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_request_signing_nonces_client_id ON request_signing_nonces(client_id);
CREATE INDEX IF NOT EXISTS idx_request_signing_nonces_expires_at ON request_signing_nonces(expires_at);
