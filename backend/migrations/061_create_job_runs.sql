-- Create catch_up_policy enum type
CREATE TYPE IF NOT EXISTS catch_up_policy AS ENUM ('all', 'latest_only');

-- Create job_run_configs table for job scheduling configuration
CREATE TABLE IF NOT EXISTS job_run_configs (
    job_name TEXT PRIMARY KEY,
    interval_seconds INTEGER NOT NULL DEFAULT 3600,
    enabled BOOLEAN NOT NULL DEFAULT true,
    catch_up_policy catch_up_policy NOT NULL DEFAULT 'latest_only',
    last_scheduled_for TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT (now() AT TIME ZONE 'utc'),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT (now() AT TIME ZONE 'utc')
);

-- Create job_runs table for tracking job executions
CREATE TABLE IF NOT EXISTS job_runs (
    id BIGSERIAL PRIMARY KEY,
    job_name TEXT NOT NULL REFERENCES job_run_configs(job_name) ON DELETE CASCADE,
    scheduled_for TIMESTAMP WITH TIME ZONE NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    status TEXT NOT NULL DEFAULT 'pending',
    -- status values: pending, running, completed, failed, skipped, catchup_scheduled
    lease_until TIMESTAMP WITH TIME ZONE,
    attempt INTEGER NOT NULL DEFAULT 1,
    error TEXT,
    result_summary TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT (now() AT TIME ZONE 'utc')
);

-- Index for quick lookups
CREATE INDEX IF NOT EXISTS idx_job_runs_job_name ON job_runs(job_name);
CREATE INDEX IF NOT EXISTS idx_job_runs_scheduled_for ON job_runs(scheduled_for);
CREATE INDEX IF NOT EXISTS idx_job_runs_status ON job_runs(status);
CREATE INDEX IF NOT EXISTS idx_job_runs_lease_until ON job_runs(lease_until) WHERE lease_until IS NOT NULL;

-- Enable row-level security
ALTER TABLE job_runs ENABLE ROW LEVEL SECURITY;
ALTER TABLE job_run_configs ENABLE ROW LEVEL SECURITY;

-- Create policies (allow all for now - can be tightened later)
CREATE POLICY IF NOT EXISTS job_runs_select ON job_runs FOR SELECT USING (true);
CREATE POLICY IF NOT EXISTS job_runs_insert ON job_runs FOR INSERT WITH CHECK (true);
CREATE POLICY IF NOT EXISTS job_runs_update ON job_runs FOR UPDATE USING (true);

CREATE POLICY IF NOT EXISTS job_run_configs_select ON job_run_configs FOR SELECT USING (true);
CREATE POLICY IF NOT EXISTS job_run_configs_insert ON job_run_configs FOR INSERT WITH CHECK (true);
CREATE POLICY IF NOT EXISTS job_run_configs_update ON job_run_configs FOR UPDATE USING (true);
