-- Token revocation table.
-- Each row represents an access token (identified by its jti claim) that has
-- been explicitly invalidated before natural expiry — e.g. after a password
-- change or key rotation.  The middleware rejects any token whose jti appears
-- here and whose expires_at has not yet passed.
--
-- Rows can be safely pruned once expires_at < strftime('%s','now').

CREATE TABLE IF NOT EXISTS token_revocations (
    jti        TEXT    NOT NULL PRIMARY KEY,
    user_id    TEXT    NOT NULL,
    revoked_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_revocations_user_id
    ON token_revocations (user_id);

CREATE INDEX IF NOT EXISTS idx_token_revocations_expires_at
    ON token_revocations (expires_at);
