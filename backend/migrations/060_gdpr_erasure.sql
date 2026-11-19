-- Migration: GDPR erasure jobs, per-store steps, certificates, and per-user
-- data keys used for crypto-shredding of backups.
--
-- user_data_keys holds a per-user data encryption key (DEK) wrapped by the
-- deployment KEK (ENCRYPTION_KEY). PII columns encrypted under the DEK become
-- unrecoverable everywhere — including inside backup snapshots — once the row
-- is destroyed, which is what makes backup erasure cryptographic rather than
-- dependent on rewriting archive files.

CREATE TABLE IF NOT EXISTS user_data_keys (
    user_id TEXT PRIMARY KEY NOT NULL,
    wrapped_dek TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    destroyed_at TEXT
);

-- One row per erasure attempt. A deletion request may produce several jobs if
-- an earlier attempt failed partway; the newest job is authoritative.
CREATE TABLE IF NOT EXISTS erasure_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    deletion_request_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    started_at TEXT NOT NULL,
    completed_at TEXT,
    certificate_json TEXT,
    error TEXT,
    FOREIGN KEY (deletion_request_id) REFERENCES data_deletion_requests(id)
);

CREATE INDEX IF NOT EXISTS idx_erasure_jobs_request ON erasure_jobs(deletion_request_id);
CREATE INDEX IF NOT EXISTS idx_erasure_jobs_user ON erasure_jobs(user_id);
CREATE INDEX IF NOT EXISTS idx_erasure_jobs_status ON erasure_jobs(status);

-- Per-store outcome for one erasure job. Every personal-data store gets a row
-- so partial failures are visible and retryable.
CREATE TABLE IF NOT EXISTS erasure_steps (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL,
    store TEXT NOT NULL,
    status TEXT NOT NULL,
    rows_affected INTEGER NOT NULL DEFAULT 0,
    detail TEXT,
    error TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (job_id) REFERENCES erasure_jobs(id)
);

CREATE INDEX IF NOT EXISTS idx_erasure_steps_job ON erasure_steps(job_id);
