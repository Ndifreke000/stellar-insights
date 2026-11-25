//! Admin API endpoints for job scheduling
//!
//! Provides endpoints for:
//! - Listing job runs with pagination
//! - Viewing job configuration
//! - Scheduling manual runs
//! - Viewing run details

use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::database::Database;
use crate::jobs::config::{JobConfigResponse, ScheduleRunRequest, ScheduleRunResponse};
use crate::jobs::persistence::{CatchUpPolicy, JobPersistence};

/// Job runs query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct RunsQuery {
    pub job_name: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Job runs response
#[derive(Debug, Clone, Serialize)]
pub struct RunsResponse {
    pub runs: Vec<JobRun>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Job run response (subset for API)
#[derive(Debug, Clone, Serialize)]
pub struct JobRun {
    pub id: i64,
    pub job_name: String,
    pub scheduled_for: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub status: String,
    pub attempt: i32,
    pub error: Option<String>,
    pub result_summary: Option<String>,
}

impl From<crate::jobs::persistence::JobRun> for JobRun {
    fn from(run: crate::jobs::persistence::JobRun) -> Self {
        Self {
            id: run.id,
            job_name: run.job_name,
            scheduled_for: run.scheduled_for.to_rfc3339(),
            started_at: run.started_at.map(|t| t.to_rfc3339()),
            completed_at: run.completed_at.map(|t| t.to_rfc3339()),
            status: run.status,
            attempt: run.attempt,
            error: run.error,
            result_summary: run.result_summary,
        }
    }
}

/// Register job API routes
pub fn routes(db: Database, persistence: Arc<JobPersistence>) -> axum::Router {
    axum::Router::new()
        .route("/admin/jobs/runs", axum::routing::get(list_runs))
        .route("/admin/jobs/runs/{id}", axum::routing::get(get_run))
        .route("/admin/jobs/config", axum::routing::get(list_configs))
        .route(
            "/admin/jobs/config/{job_name}",
            axum::routing::get(get_config),
        )
        .route(
            "/admin/jobs/config/{job_name}",
            axum::routing::put(update_config),
        )
        .route("/admin/jobs/schedule", axum::routing::post(schedule_run))
        .route(
            "/admin/jobs/runs/counts",
            axum::routing::get(get_run_counts),
        )
        .with_state((Arc::new(db), persistence))
}

/// List job runs
async fn list_runs(
    State((_db, persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
    Query(params): Query<RunsQuery>,
) -> Result<Json<RunsResponse>, (axum::http::StatusCode, String)> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let (runs, total) = match &params.job_name {
        Some(job_name) => {
            let runs = persistence
                .get_all_runs(job_name, limit, offset)
                .await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let total = runs.len() as i64;
            (runs, total)
        }
        None => {
            // List all jobs' runs
            let job_names = persistence
                .get_all_configs()
                .await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let mut all_runs = vec![];
            for config in job_names {
                let runs = persistence
                    .get_all_runs(&config.job_name, limit, offset)
                    .await
                    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                all_runs.extend(runs);
            }
            let total = all_runs.len() as i64;
            (all_runs, total)
        }
    };

    let page = (offset / limit) + 1;

    Ok(Json(RunsResponse {
        runs: runs.into_iter().map(JobRun::from).collect(),
        total,
        page,
        page_size: limit,
    }))
}

/// Get a specific job run
async fn get_run(
    State((db, _persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
    Path(run_id): Path<i64>,
) -> Result<Json<JobRun>, (axum::http::StatusCode, String)> {
    let run: Option<crate::jobs::persistence::JobRun> = sqlx::query_as(
        r#"
        SELECT 
            id, job_name, scheduled_for, started_at, completed_at,
            status, lease_until, attempt, error, result_summary, created_at
        FROM job_runs
        WHERE id = ?
        "#,
    )
    .bind(run_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    run.map(JobRun::from)
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Run not found".to_string(),
        ))
        .map(Json)
}

/// List all job configurations
async fn list_configs(
    State((_db, persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
) -> Result<Json<Vec<JobConfigResponse>>, (axum::http::StatusCode, String)> {
    let configs = persistence
        .get_all_configs()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        configs.into_iter().map(JobConfigResponse::from).collect(),
    ))
}

/// Get a specific job configuration
async fn get_config(
    State((_db, persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
    Path(job_name): Path<String>,
) -> Result<Json<JobConfigResponse>, (axum::http::StatusCode, String)> {
    let config = persistence
        .get_config(&job_name)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    config
        .map(JobConfigResponse::from)
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Job config not found".to_string(),
        ))
        .map(Json)
}

/// Update a job configuration
async fn update_config(
    State((db, persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
    Path(job_name): Path<String>,
    Json(payload): Json<crate::jobs::config::UpdateJobConfigRequest>,
) -> Result<Json<JobConfigResponse>, (axum::http::StatusCode, String)> {
    let config = persistence
        .get_config(&job_name)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut config = config.ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Job config not found".to_string(),
    ))?;

    if let Some(enabled) = payload.enabled {
        config.enabled = enabled;
    }
    if let Some(interval) = payload.interval_seconds {
        if interval <= 0 {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "interval_seconds must be a positive number of seconds".to_string(),
            ));
        }
        config.interval_seconds = interval;
    }
    if let Some(policy) = payload.catch_up_policy {
        if policy != "all" && policy != "latest_only" {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "catch_up_policy must be 'all' or 'latest_only'".to_string(),
            ));
        }
        config.policy = policy;
    }
    config.updated_at = Utc::now();

    // Update in database
    sqlx::query(
        r#"
        UPDATE job_run_configs
        SET enabled = ?, interval_seconds = ?, catch_up_policy = ?, updated_at = ?
        WHERE job_name = ?
        "#,
    )
    .bind(config.enabled)
    .bind(config.interval_seconds)
    .bind(&config.policy)
    .bind(config.updated_at)
    .bind(&config.job_name)
    .execute(db.pool())
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(JobConfigResponse::from(config)))
}

/// Schedule a manual run
async fn schedule_run(
    State((_db, persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
    Json(payload): Json<ScheduleRunRequest>,
) -> Result<Json<ScheduleRunResponse>, (axum::http::StatusCode, String)> {
    // For manual scheduling, we'll use the current time as the job name
    // In production, this would be parameterized
    let job_name = "manual-run".to_string();

    let run = persistence
        .get_or_create_run(&job_name, payload.scheduled_for)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ScheduleRunResponse {
        job_name,
        run_id: run.id,
        scheduled_for: run.scheduled_for,
    }))
}

/// Get run counts by status
async fn get_run_counts(
    State((_db, persistence)): State<(Arc<Database>, Arc<JobPersistence>)>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let job_names = persistence
        .get_all_configs()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut counts = serde_json::Map::new();
    for config in job_names {
        let run_counts = persistence
            .get_run_counts(&config.job_name)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        counts.insert(
            config.job_name,
            serde_json::json!({
                "completed": run_counts.completed,
                "failed": run_counts.failed,
                "running": run_counts.running,
                "pending": run_counts.pending,
                "skipped": run_counts.skipped,
                "total": run_counts.total,
            }),
        );
    }

    Ok(Json(serde_json::Value::Object(counts)))
}
