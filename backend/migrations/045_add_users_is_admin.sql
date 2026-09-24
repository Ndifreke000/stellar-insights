-- Adds a minimal admin-role flag. Until now there was no admin/role
-- concept anywhere in the schema or auth model -- admin_ip_whitelist and
-- audit_log routes could only require *authentication* (any logged-in
-- user), not real admin-only *authorization*. SQLite supports a plain
-- ADD COLUMN with a default here (unlike the snapshots.id fix, which
-- needed a full table recreation because it was changing an existing
-- column's type).
ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;
