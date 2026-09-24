-- Migration: Create tables for TOTP-based 2FA with backup codes
CREATE TABLE IF NOT EXISTS user_2fa_secrets (
    user_id TEXT PRIMARY KEY,
    encrypted_secret TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    enrolled_at TIMESTAMP,
    backup_codes_generated_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS user_2fa_backup_codes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    hashed_code TEXT NOT NULL UNIQUE,
    used_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES user_2fa_secrets(user_id)
);

CREATE INDEX IF NOT EXISTS idx_2fa_backup_user_id ON user_2fa_backup_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_2fa_backup_used ON user_2fa_backup_codes(used_at);

-- Add 2fa_verified flag to track session's 2FA verification state (in-memory, not persisted)
-- Sessions table tracks this via audit log and session state logic
