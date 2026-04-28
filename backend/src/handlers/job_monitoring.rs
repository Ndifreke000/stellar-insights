use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::database::Database;
use crate::error::{ApiError, ApiResult};
use crate::observability::job_metrics::get_job_status_summary;

/// Query parameters for job monitoring endpoints
#[derive(Debug, Deserialize)]
pub struct JobMonitoringQuery {
    /// Filter by specific job name
    pub job: Option<String>,
    /// Include detailed execution history
    pub include_history: Option<bool>,
    /// Limit number of historical records to return
    pub history_limit: Option<usize>,
}

/// Detailed job status response
#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub jobs: HashMap<String, JobStatusDetail>,
    pub summary: JobSummary,
    pub timestamp: i64,
}

/// Individual job status details
#[derive(Debug, Serialize)]
pub struct JobStatusDetail {
    pub name: String,
    pub is_active: bool,
    pub total_executions: u64,
    pub total_failures: u64,
    pub consecutive_failures: u64,
    pub success_rate: f64,
    pub last_success_timestamp: Option<i64>,
    pub last_failure_timestamp: Option<i64>,
    pub last_execution: Option<LastExecutionDetail>,
    pub health_status: JobHealthStatus,
}

/// Last execution details
#[derive(Debug, Serialize)]
pub struct LastExecutionDetail {
    pub status: String,
    pub started_at: u64,
    pub duration_ms: Option<u64>,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
}

/// Job health status
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobHealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Overall job summary
#[derive(Debug, Serialize)]
pub struct JobSummary {
    pub total_jobs: usize,
    pub active_jobs: usize,
    pub healthy_jobs: usize,
    pub warning_jobs: usize,
    pub critical_jobs: usize,
    pub total_executions: u64,
    pub total_failures: u64,
    pub overall_success_rate: f64,
}

/// Get comprehensive job status
pub async fn get_job_status(
    State(_db): State<Arc<Database>>,
    Query(query): Query<JobMonitoringQuery>,
) -> ApiResult<Json<JobStatusResponse>> {
    info!("Fetching job status with query: {:?}", query);

    let status_summary = get_job_status_summary().await;
    let summary_data = status_summary.as_object().unwrap();

    let jobs_data = summary_data.get("jobs").unwrap().as_object().unwrap();
    let mut jobs = HashMap::new();

    let mut total_executions = 0u64;
    let mut total_failures = 0u64;
    let mut healthy_count = 0usize;
    let mut warning_count = 0usize;
    let mut critical_count = 0usize;

    for (name, job_data) in jobs_data {
        // Filter by specific job if requested
        if let Some(ref filter_job) = query.job {
            if name != filter_job {
                continue;
            }
        }

        let job_obj = job_data.as_object().unwrap();
        let is_active = job_obj.get("is_active").unwrap().as_bool().unwrap();
        let total_executions_job = job_obj.get("total_executions").unwrap().as_u64().unwrap();
        let total_failures_job = job_obj.get("total_failures").unwrap().as_u64().unwrap();
        let consecutive_failures = job_obj.get("consecutive_failures").unwrap().as_u64().unwrap();
        let last_success_timestamp = job_obj.get("last_success_timestamp").unwrap().as_i64();
        let last_failure_timestamp = job_obj.get("last_failure_timestamp").unwrap().as_i64();

        total_executions += total_executions_job;
        total_failures += total_failures_job;

        let success_rate = if total_executions_job > 0 {
            ((total_executions_job - total_failures_job) as f64 / total_executions_job as f64) * 100.0
        } else {
            0.0
        };

        let health_status = determine_health_status(
            is_active,
            consecutive_failures,
            success_rate,
            last_success_timestamp,
            last_failure_timestamp,
        );

        match health_status {
            JobHealthStatus::Healthy => healthy_count += 1,
            JobHealthStatus::Warning => warning_count += 1,
            JobHealthStatus::Critical => critical_count += 1,
            JobHealthStatus::Unknown => {}
        }

        let last_execution = job_obj.get("last_execution").and_then(|exec| {
            exec.as_object().map(|exec_obj| LastExecutionDetail {
                status: exec_obj.get("status").unwrap().as_str().unwrap().to_string(),
                started_at: exec_obj.get("started_at").unwrap().as_u64().unwrap(),
                duration_ms: exec_obj.get("duration_ms").and_then(|v| v.as_u64()),
                completed_at: exec_obj.get("completed_at").and_then(|v| v.as_u64()),
                error: exec_obj.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
            })
        });

        let job_detail = JobStatusDetail {
            name: name.clone(),
            is_active,
            total_executions: total_executions_job,
            total_failures: total_failures_job,
            consecutive_failures,
            success_rate,
            last_success_timestamp,
            last_failure_timestamp,
            last_execution,
            health_status,
        };

        jobs.insert(name, job_detail);
    }

    let overall_success_rate = if total_executions > 0 {
        ((total_executions - total_failures) as f64 / total_executions as f64) * 100.0
    } else {
        0.0
    };

    let summary = JobSummary {
        total_jobs: jobs.len(),
        active_jobs: jobs.values().filter(|j| j.is_active).count(),
        healthy_jobs: healthy_count,
        warning_jobs: warning_count,
        critical_jobs: critical_count,
        total_executions,
        total_failures,
        overall_success_rate,
    };

    let response = JobStatusResponse {
        jobs,
        summary,
        timestamp: summary_data.get("timestamp").unwrap().as_i64().unwrap(),
    };

    Ok(Json(response))
}

/// Get simple health check for jobs
pub async fn get_job_health(
    State(_db): State<Arc<Database>>,
) -> ApiResult<Json<serde_json::Value>> {
    let status_summary = get_job_status_summary().await;
    let summary_data = status_summary.as_object().unwrap();
    let jobs_data = summary_data.get("jobs").unwrap().as_object().unwrap();

    let mut healthy_count = 0usize;
    let mut warning_count = 0usize;
    let mut critical_count = 0usize;

    for (_, job_data) in jobs_data {
        let job_obj = job_data.as_object().unwrap();
        let is_active = job_obj.get("is_active").unwrap().as_bool().unwrap();
        let total_executions = job_obj.get("total_executions").unwrap().as_u64().unwrap();
        let total_failures = job_obj.get("total_failures").unwrap().as_u64().unwrap();
        let consecutive_failures = job_obj.get("consecutive_failures").unwrap().as_u64().unwrap();
        let last_success_timestamp = job_obj.get("last_success_timestamp").unwrap().as_i64();
        let last_failure_timestamp = job_obj.get("last_failure_timestamp").unwrap().as_i64();

        let success_rate = if total_executions > 0 {
            ((total_executions - total_failures) as f64 / total_executions as f64) * 100.0
        } else {
            0.0
        };

        let health_status = determine_health_status(
            is_active,
            consecutive_failures,
            success_rate,
            last_success_timestamp,
            last_failure_timestamp,
        );

        match health_status {
            JobHealthStatus::Healthy => healthy_count += 1,
            JobHealthStatus::Warning => warning_count += 1,
            JobHealthStatus::Critical => critical_count += 1,
            JobHealthStatus::Unknown => {}
        }
    }

    let total_jobs = healthy_count + warning_count + critical_count;
    let overall_status = if critical_count > 0 {
        "critical"
    } else if warning_count > 0 {
        "warning"
    } else if total_jobs == 0 {
        "unknown"
    } else {
        "healthy"
    };

    let response = serde_json::json!({
        "status": overall_status,
        "total_jobs": total_jobs,
        "healthy_jobs": healthy_count,
        "warning_jobs": warning_count,
        "critical_jobs": critical_count,
        "timestamp": chrono::Utc::now().timestamp()
    });

    Ok(Json(response))
}

/// Get job metrics for Prometheus
pub async fn get_job_metrics() -> impl IntoResponse {
    use prometheus::{Encoder, TextEncoder};
    
    let registry = &crate::observability::metrics::REGISTRY;
    let metric_families = registry.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            buffer,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("content-type", "text/plain")],
            format!("Failed to encode metrics: {}", e).into_bytes(),
        ),
    }
}

/// Determine job health status based on various factors
fn determine_health_status(
    is_active: bool,
    consecutive_failures: u64,
    success_rate: f64,
    last_success: Option<i64>,
    last_failure: Option<i64>,
) -> JobHealthStatus {
    let now = chrono::Utc::now().timestamp();

    // If job has never run, consider it unknown
    if last_success.is_none() && last_failure.is_none() {
        return JobHealthStatus::Unknown;
    }

    // Critical conditions
    if consecutive_failures >= 5 {
        return JobHealthStatus::Critical;
    }

    if success_rate < 50.0 && consecutive_failures >= 3 {
        return JobHealthStatus::Critical;
    }

    // Check if job hasn't succeeded in a long time (24 hours)
    if let Some(last_success) = last_success {
        if now - last_success > 86400 && consecutive_failures > 0 {
            return JobHealthStatus::Critical;
        }
    } else if consecutive_failures > 0 {
        // Never succeeded but has failures
        return JobHealthStatus::Critical;
    }

    // Warning conditions
    if consecutive_failures >= 3 {
        return JobHealthStatus::Warning;
    }

    if success_rate < 80.0 {
        return JobHealthStatus::Warning;
    }

    // Check if job hasn't succeeded in a moderate time (6 hours)
    if let Some(last_success) = last_success {
        if now - last_success > 21600 && consecutive_failures > 0 {
            return JobHealthStatus::Warning;
        }
    }

    // Check if last failure was recent (1 hour) and job is supposed to be running
    if is_active {
        if let Some(last_failure) = last_failure {
            if now - last_failure < 3600 && consecutive_failures > 0 {
                return JobHealthStatus::Warning;
            }
        }
    }

    JobHealthStatus::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_health_status() {
        // Test critical conditions
        assert_eq!(
            determine_health_status(true, 5, 90.0, Some(100), Some(110)),
            JobHealthStatus::Critical
        );

        assert_eq!(
            determine_health_status(true, 3, 40.0, Some(100), Some(110)),
            JobHealthStatus::Critical
        );

        // Test warning conditions
        assert_eq!(
            determine_health_status(true, 3, 90.0, Some(100), Some(110)),
            JobHealthStatus::Warning
        );

        assert_eq!(
            determine_health_status(true, 1, 70.0, Some(100), Some(110)),
            JobHealthStatus::Warning
        );

        // Test healthy conditions
        assert_eq!(
            determine_health_status(true, 0, 95.0, Some(100), None),
            JobHealthStatus::Healthy
        );

        assert_eq!(
            determine_health_status(false, 1, 90.0, Some(100), Some(110)),
            JobHealthStatus::Healthy
        );

        // Test unknown conditions
        assert_eq!(
            determine_health_status(false, 0, 0.0, None, None),
            JobHealthStatus::Unknown
        );
    }
}
