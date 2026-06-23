-- Existing API keys were hashed with SHA-256 which is incompatible with the
-- new Argon2id scheme. Revoke all active keys so users re-issue fresh ones
-- that will be stored with Argon2id.
UPDATE api_keys
SET status = 'revoked', revoked_at = datetime('now')
WHERE status = 'active';
