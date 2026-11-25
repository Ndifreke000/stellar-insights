-- Job scheduling configuration and run tracking (SQLite).
--
-- catch_up_policy is stored as text ('all' | 'latest_only'); SQLite has no
-- enum types, so the allowed values are enforced with a CHECK constraint.

CREATE TABLE IF NOT EXISTS job_run_configs (
    job_name TEXT PRIMARY KEY,
    interval_seconds INTEGER NOT NULL DEFAULT 3600,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    catch_up_policy TEXT NOT NULL DEFAULT 'latest_only'
        CHECK (catch_up_policy IN ('all', 'latest_only')),
    last_scheduled_for TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Tracks individual job executions
CREATE TABLE IF NOT EXISTS job_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_name TEXT NOT NULL REFERENCES job_run_configs(job_name) ON DELETE CASCADE,
    scheduled_for TIMESTAMP NOT NULL,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    status TEXT NOT NULL DEFAULT 'pending',
    -- status values: pending, running, completed, failed, skipped, catchup_scheduled
    lease_until TIMESTAMP,
    attempt INTEGER NOT NULL DEFAULT 1,
    error TEXT,
    result_summary TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_job_runs_job_name ON job_runs(job_name);
CREATE INDEX IF NOT EXISTS idx_job_runs_scheduled_for ON job_runs(scheduled_for);
CREATE INDEX IF NOT EXISTS idx_job_runs_status ON job_runs(status);
CREATE INDEX IF NOT EXISTS idx_job_runs_lease_until ON job_runs(lease_until) WHERE lease_until IS NOT NULL;
