//! Job persistence layer for tracking scheduled runs and ensuring exactly-once execution.
//!
//! This module provides:
//! - `job_runs` table for tracking job executions
//! - `job_run_configs` table for scheduling configuration (interval, catch-up policy)
//! - Lease-based claiming of runs to prevent duplicate execution
//! - Catch-up scheduling for missed runs based on configured policy

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use tracing::{info, warn};

use crate::jobs::config::JobRunConfig;

/// Policy for handling missed job runs when an instance comes back online
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "catch_up_policy")]
#[sqlx(rename_all = "lowercase")]
pub enum CatchUpPolicy {
    /// Schedule all missed runs
    All,
    /// Only schedule the most recent missed run
    LatestOnly,
}

impl CatchUpPolicy {
    /// Text representation stored in `job_run_configs.catch_up_policy`.
    pub fn as_str(self) -> &'static str {
        match self {
            CatchUpPolicy::All => "all",
            CatchUpPolicy::LatestOnly => "latest_only",
        }
    }

    /// Parse the stored text value; unknown values fall back to `LatestOnly`.
    pub fn parse(value: &str) -> Self {
        match value {
            "all" => CatchUpPolicy::All,
            _ => CatchUpPolicy::LatestOnly,
        }
    }
}

impl Default for CatchUpPolicy {
    fn default() -> Self {
        CatchUpPolicy::LatestOnly
    }
}

/// Represents a scheduled job run
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobRun {
    pub id: i64,
    pub job_name: String,
    pub scheduled_for: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub lease_until: Option<DateTime<Utc>>,
    pub attempt: i32,
    pub error: Option<String>,
    pub result_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl JobRun {
    /// Create a new pending job run
    pub fn new_pending(job_name: &str, scheduled_for: DateTime<Utc>) -> Self {
        Self {
            id: 0,
            job_name: job_name.to_string(),
            scheduled_for,
            started_at: None,
            completed_at: None,
            status: "pending".to_string(),
            lease_until: None,
            attempt: 1,
            error: None,
            result_summary: None,
            created_at: Utc::now(),
        }
    }

    /// Mark the run as started
    pub fn start(&mut self) {
        self.status = "running".to_string();
        self.started_at = Some(Utc::now());
        // Set lease to 75% of the way to next scheduled time (max 30 min)
        let default_lease = Duration::minutes(30);
        self.lease_until = Some(Utc::now() + default_lease);
    }

    /// Mark the run as completed
    pub fn complete(&mut self) {
        self.status = "completed".to_string();
        self.completed_at = Some(Utc::now());
        self.lease_until = None;
    }

    /// Mark the run as failed
    pub fn fail(&mut self, error: &str) {
        self.status = "failed".to_string();
        self.completed_at = Some(Utc::now());
        self.error = Some(error.to_string());
        self.lease_until = None;
    }

    /// Mark the run as skipped (e.g., no catch-up policy)
    pub fn skip(&mut self) {
        self.status = "skipped".to_string();
        self.completed_at = Some(Utc::now());
        self.lease_until = None;
    }
}

/// Job persistence layer for tracking scheduled runs
pub struct JobPersistence {
    pool: SqlitePool,
}

impl JobPersistence {
    /// Create a new JobPersistence instance
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize job run configs for all jobs if they don't exist
    pub async fn initialize_job_configs(&self) -> Result<()> {
        let jobs = vec![
            ("corridor-refresh", 300, CatchUpPolicy::LatestOnly),
            ("asset-revalidation", 86400, CatchUpPolicy::LatestOnly),
            ("backup", 86400, CatchUpPolicy::LatestOnly),
            ("backfill", 3600, CatchUpPolicy::All),
            ("anchor-refresh", 600, CatchUpPolicy::LatestOnly),
        ];

        for (job_name, interval_seconds, catch_up_policy) in jobs {
            sqlx::query(
                r#"
                INSERT INTO job_run_configs (job_name, interval_seconds, enabled, catch_up_policy)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(job_name) DO NOTHING
                "#,
            )
            .bind(job_name)
            .bind(interval_seconds)
            .bind(true)
            .bind(catch_up_policy.as_str())
            .execute(&self.pool)
            .await
            .with_context(|| format!("Failed to initialize job config for '{}'", job_name))?;
        }

        Ok(())
    }

    /// Get or create a job run for the given scheduled time
    pub async fn get_or_create_run(
        &self,
        job_name: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<JobRun> {
        // Try to find an existing pending run for this schedule
        let existing: Option<JobRun> = sqlx::query_as(
            r#"
            SELECT 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            FROM job_runs
            WHERE job_name = ? AND scheduled_for = ?
            AND status IN ('pending', 'running')
            LIMIT 1
            "#,
        )
        .bind(job_name)
        .bind(scheduled_for)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(run) = existing {
            return Ok(run);
        }

        // Create a new pending run
        let result = sqlx::query(
            r#"
            INSERT INTO job_runs (job_name, scheduled_for, status)
            VALUES (?, ?, 'pending')
            RETURNING 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            "#,
        )
        .bind(job_name)
        .bind(scheduled_for)
        .fetch_one(&self.pool)
        .await?;

        Ok(JobRun {
            id: result.try_get("id")?,
            job_name: job_name.to_string(),
            scheduled_for,
            started_at: None,
            completed_at: None,
            status: "pending".to_string(),
            lease_until: None,
            attempt: 1,
            error: None,
            result_summary: None,
            created_at: Utc::now(),
        })
    }

    /// Claim a pending run for execution
    /// Returns the run if it was claimed, or None if another instance has it
    pub async fn claim_run(&self, job_name: &str, max_attempts: i32) -> Result<Option<JobRun>> {
        let now = Utc::now();
        let lease_duration = Duration::minutes(30);

        // Find a pending or failed run to claim
        let run: Option<JobRun> = sqlx::query_as(
            r#"
            UPDATE job_runs
            SET 
                status = 'running',
                started_at = ?, 
                lease_until = ?,
                attempt = COALESCE(attempt, 1) + 1
            WHERE id = (
                SELECT id FROM job_runs
                WHERE job_name = ?
                AND status IN ('pending', 'failed')
                AND attempt < ?
                ORDER BY scheduled_for ASC
                LIMIT 1
            )
            RETURNING 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            "#,
        )
        .bind(now)
        .bind(now + lease_duration)
        .bind(job_name)
        .bind(max_attempts)
        .fetch_optional(&self.pool)
        .await?;

        Ok(run)
    }

    /// Check for expired leases and return runs that can be retried
    pub async fn get_expired_leases(&self) -> Result<Vec<JobRun>> {
        let now = Utc::now();

        let runs: Vec<JobRun> = sqlx::query_as(
            r#"
            SELECT 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            FROM job_runs
            WHERE status = 'running'
            AND lease_until < ?
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        Ok(runs)
    }

    /// Record the completion of a job run
    pub async fn complete_run(&self, run_id: i64, result_summary: Option<&str>) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE job_runs
            SET 
                status = 'completed',
                completed_at = ?,
                lease_until = NULL,
                result_summary = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(result_summary)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a failed job run
    pub async fn fail_run(&self, run_id: i64, error: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE job_runs
            SET 
                status = 'failed',
                completed_at = ?,
                lease_until = NULL,
                error = ?
            WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(error)
        .bind(run_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get runs for a specific job within a time range
    pub async fn get_runs_in_range(
        &self,
        job_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<JobRun>> {
        let runs: Vec<JobRun> = sqlx::query_as(
            r#"
            SELECT 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            FROM job_runs
            WHERE job_name = ?
            AND scheduled_for >= ?
            AND scheduled_for <= ?
            ORDER BY scheduled_for DESC
            "#,
        )
        .bind(job_name)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(runs)
    }

    /// Get the most recent run for a job
    pub async fn get_latest_run(&self, job_name: &str) -> Result<Option<JobRun>> {
        let run: Option<JobRun> = sqlx::query_as(
            r#"
            SELECT 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            FROM job_runs
            WHERE job_name = ?
            ORDER BY scheduled_for DESC
            LIMIT 1
            "#,
        )
        .bind(job_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(run)
    }

    /// Get all runs for a job, paginated
    pub async fn get_all_runs(
        &self,
        job_name: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<JobRun>> {
        let runs: Vec<JobRun> = sqlx::query_as(
            r#"
            SELECT 
                id, job_name, scheduled_for, started_at, completed_at,
                status, lease_until, attempt, error, result_summary, created_at
            FROM job_runs
            WHERE job_name = ?
            ORDER BY scheduled_for DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(job_name)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(runs)
    }

    /// Get counts of runs by status
    pub async fn get_run_counts(&self, job_name: &str) -> Result<JobRunCounts> {
        let counts: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT 
                COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) as completed,
                COALESCE(SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END), 0) as failed,
                COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0) as running,
                COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
                COALESCE(SUM(CASE WHEN status = 'skipped' THEN 1 ELSE 0 END), 0) as skipped,
                COUNT(*) as total
            FROM job_runs
            WHERE job_name = ?
            "#,
        )
        .bind(job_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(JobRunCounts {
            completed: counts.0,
            failed: counts.1,
            running: counts.2,
            pending: counts.3,
            skipped: counts.4,
            total: counts.5,
        })
    }

    /// Find missed runs based on catch-up policy and schedule them
    pub async fn find_missed_runs(
        &self,
        job_name: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>> {
        let config: Option<JobRunConfig> = sqlx::query_as(
            r#"
            SELECT job_name, interval_seconds, enabled, catch_up_policy, last_scheduled_for,
                   created_at, updated_at
            FROM job_run_configs
            WHERE job_name = ?
            "#,
        )
        .bind(job_name)
        .fetch_optional(&self.pool)
        .await?;

        let config = match config {
            Some(c) => c,
            None => return Ok(vec![]),
        };

        if !config.enabled {
            return Ok(vec![]);
        }

        let last_scheduled = config
            .last_scheduled_for
            .unwrap_or_else(|| now - Duration::days(365));
        let policy = CatchUpPolicy::parse(&config.policy);

        // Calculate how many runs were missed
        let duration_since_last = now - last_scheduled;
        let missed_count = if config.interval_seconds > 0 {
            duration_since_last.num_seconds() / config.interval_seconds
        } else {
            0
        };

        if missed_count <= 0 {
            return Ok(vec![]);
        }

        let mut missed_schedules = vec![];
        match policy {
            CatchUpPolicy::All => {
                // Schedule all missed runs
                for i in 1..=missed_count {
                    let scheduled_for = last_scheduled
                        + Duration::seconds(config.interval_seconds.saturating_mul(i));
                    if scheduled_for <= now {
                        missed_schedules.push(scheduled_for);
                    }
                }
            }
            CatchUpPolicy::LatestOnly => {
                // Only schedule the most recent missed run
                let scheduled_for = last_scheduled
                    + Duration::seconds(config.interval_seconds.saturating_mul(missed_count));
                if scheduled_for <= now {
                    missed_schedules.push(scheduled_for);
                }
            }
        }

        Ok(missed_schedules)
    }

    /// Update the last scheduled time for a job
    pub async fn update_last_scheduled(
        &self,
        job_name: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE job_run_configs
            SET 
                last_scheduled_for = ?,
                updated_at = ?
            WHERE job_name = ?
            "#,
        )
        .bind(scheduled_for)
        .bind(Utc::now())
        .bind(job_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all job run configurations
    pub async fn get_all_configs(&self) -> Result<Vec<JobRunConfig>> {
        let configs: Vec<JobRunConfig> = sqlx::query_as(
            r#"
            SELECT job_name, interval_seconds, enabled, catch_up_policy, last_scheduled_for,
                   created_at, updated_at
            FROM job_run_configs
            ORDER BY job_name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(configs)
    }

    /// Get configuration for a specific job
    pub async fn get_config(&self, job_name: &str) -> Result<Option<JobRunConfig>> {
        let config: Option<JobRunConfig> = sqlx::query_as(
            r#"
            SELECT job_name, interval_seconds, enabled, catch_up_policy, last_scheduled_for,
                   created_at, updated_at
            FROM job_run_configs
            WHERE job_name = ?
            "#,
        )
        .bind(job_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(config)
    }
}

/// Counts of job runs by status
#[derive(Debug, Clone, Default)]
pub struct JobRunCounts {
    pub completed: i64,
    pub failed: i64,
    pub running: i64,
    pub pending: i64,
    pub skipped: i64,
    pub total: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_run_lifecycle() {
        let mut run = JobRun::new_pending("test-job", Utc::now());

        assert_eq!(run.status, "pending");
        assert!(run.started_at.is_none());
        assert!(run.completed_at.is_none());

        run.start();
        assert_eq!(run.status, "running");
        assert!(run.started_at.is_some());
        assert!(run.completed_at.is_none());
        assert!(run.lease_until.is_some());

        run.complete();
        assert_eq!(run.status, "completed");
        assert!(run.completed_at.is_some());
        assert!(run.lease_until.is_none());
    }
}
