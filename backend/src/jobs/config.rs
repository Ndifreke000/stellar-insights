//! Job configuration types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Configuration for a job run
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct JobRunConfig {
    pub job_name: String,
    pub interval_seconds: i64,
    pub enabled: bool,
    #[sqlx(rename = "catch_up_policy")]
    pub policy: String, // Will be parsed as enum in persistence module
    pub last_scheduled_for: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl JobRunConfig {
    /// Create a new job configuration
    pub fn new(job_name: &str, interval_seconds: i64, enabled: bool) -> Self {
        Self {
            job_name: job_name.to_string(),
            interval_seconds,
            enabled,
            policy: "latest_only".to_string(), // Default
            last_scheduled_for: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Request body for updating job configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateJobConfigRequest {
    pub enabled: Option<bool>,
    pub interval_seconds: Option<i64>,
    pub catch_up_policy: Option<String>,
}

/// Response for job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfigResponse {
    pub job_name: String,
    pub interval_seconds: i64,
    pub enabled: bool,
    pub catch_up_policy: String,
    pub last_scheduled_for: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<JobRunConfig> for JobConfigResponse {
    fn from(config: JobRunConfig) -> Self {
        Self {
            job_name: config.job_name,
            interval_seconds: config.interval_seconds,
            enabled: config.enabled,
            catch_up_policy: config.policy,
            last_scheduled_for: config.last_scheduled_for,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

/// Request body for scheduling a run
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScheduleRunRequest {
    pub scheduled_for: DateTime<Utc>,
}

/// Response for scheduling a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRunResponse {
    pub job_name: String,
    pub run_id: i64,
    pub scheduled_for: DateTime<Utc>,
}

/// Response for claiming a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRunResponse {
    pub run_id: i64,
    pub job_name: String,
    pub scheduled_for: DateTime<Utc>,
    pub attempt: i32,
}

/// Response for completing a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRunResponse {
    pub run_id: i64,
    pub status: String,
    pub completed_at: DateTime<Utc>,
}

/// Response for failing a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailRunResponse {
    pub run_id: i64,
    pub status: String,
    pub completed_at: DateTime<Utc>,
    pub error: Option<String>,
}
