-- Dedicated single-row table used by the enhanced health check to verify
-- the database is actually writable, not just reachable (a `SELECT 1` can
-- succeed against a read-only replica, a full disk, or a DB stuck
-- mid-recovery). See health_check_enhanced::check_database_writable_enhanced.
CREATE TABLE IF NOT EXISTS health_check_probe (
    id INTEGER PRIMARY KEY,
    checked_at TEXT NOT NULL
);
