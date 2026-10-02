-- GdprService::set_consent upserts with ON CONFLICT(user_id, consent_type),
-- which SQLite rejects unless a matching UNIQUE constraint exists. Without
-- this index every consent write failed with "ON CONFLICT clause does not
-- match any PRIMARY KEY or UNIQUE constraint".

-- Keep only the most recently updated row per (user_id, consent_type) so the
-- index can be created on databases that already hold duplicates.
DELETE FROM user_consents
WHERE rowid NOT IN (
    SELECT rowid FROM (
        SELECT rowid,
               ROW_NUMBER() OVER (
                   PARTITION BY user_id, consent_type
                   ORDER BY updated_at DESC, rowid DESC
               ) AS position
        FROM user_consents
    )
    WHERE position = 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_consents_user_type
    ON user_consents(user_id, consent_type);
