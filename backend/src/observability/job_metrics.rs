use std::sync::Arc;
use std::time::{Duration, Instant};
use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram,
    HistogramOpts, Opts,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use prometheus::{IntCounterVec, HistogramVec, IntGaugeVec, Opts, HistogramOpts};

lazy_static! {
    // Job execution metrics
    pub static ref JOB_EXECUTIONS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "job_executions_total",
            "Total number of job executions"
        ),
        &["job_name", "status"]
    )
    .expect("Failed to register job_executions_total counter");

    pub static ref JOB_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "job_duration_seconds",
            "Job execution duration in seconds"
        )
        .buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0]),
        &["job_name"]
    )
    .expect("Failed to register job_duration_seconds histogram");

    pub static ref JOB_FAILURES_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "job_failures_total",
            "Total number of job failures"
        ),
        &["job_name", "error_type"]
    )
    .expect("Failed to register job_failures_total counter");

    pub static ref JOB_LAST_SUCCESS_TIMESTAMP: IntGaugeVec = IntGaugeVec::new(
        Opts::new(
            "job_last_success_timestamp",
            "Unix timestamp of last successful job execution"
        ),
        &["job_name"]
    )
    .expect("Failed to register job_last_success_timestamp gauge");

    pub static ref JOB_LAST_FAILURE_TIMESTAMP: Gauge = register_gauge!(Opts::new(
        "job_last_failure_timestamp",
        "Unix timestamp of last failed job execution"
    )
    .label_names(vec!["job_name"]))
    .expect("Failed to register job_last_failure_timestamp gauge");

    pub static ref JOB_ACTIVE_STATUS: Gauge = register_gauge!(Opts::new(
        "job_active_status",
        "Whether a job is currently active (1) or inactive (0)"
    )
    .label_names(vec!["job_name"]))
    .expect("Failed to register job_active_status gauge");

    pub static ref JOB_CONSECUTIVE_FAILURES: Gauge = register_gauge!(Opts::new(
        "job_consecutive_failures",
        "Number of consecutive failures for a job"
    )
    .label_names(vec!["job_name"]))
    .expect("Failed to register job_consecutive_failures gauge");

    // Global job registry for status tracking
    pub static ref JOB_REGISTRY: Arc<RwLock<JobRegistry>> = Arc::new(RwLock::new(JobRegistry::new()));
}

/// Job execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Running,
    Success,
    Failed(String),
    Timeout,
}

/// Job execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecution {
    pub job_name: String,
    pub status: JobStatus,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    pub duration: Option<Duration>,
    pub error: Option<String>,
}

impl JobExecution {
    pub fn new(job_name: String) -> Self {
        Self {
            job_name,
            status: JobStatus::Running,
            started_at: Instant::now(),
            completed_at: None,
            duration: None,
            error: None,
        }
    }

    pub fn complete_success(mut self) -> Self {
        self.status = JobStatus::Success;
        self.completed_at = Some(Instant::now());
        self.duration = Some(self.completed_at.unwrap() - self.started_at);
        self
    }

    pub fn complete_failure(mut self, error: String) -> Self {
        self.status = JobStatus::Failed(error.clone());
        self.completed_at = Some(Instant::now());
        self.duration = Some(self.completed_at.unwrap() - self.started_at);
        self.error = Some(error);
        self
    }

    pub fn complete_timeout(mut self) -> Self {
        self.status = JobStatus::Timeout;
        self.completed_at = Some(Instant::now());
        self.duration = Some(self.completed_at.unwrap() - self.started_at);
        self
    }
}

/// Job registry for tracking job states
#[derive(Debug, Default)]
pub struct JobRegistry {
    jobs: std::collections::HashMap<String, JobInfo>,
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub last_execution: Option<JobExecution>,
    pub consecutive_failures: u64,
    pub is_active: bool,
    pub total_executions: u64,
    pub total_failures: u64,
    pub last_success_timestamp: Option<i64>,
    pub last_failure_timestamp: Option<i64>,
}

impl JobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_job(&mut self, job_name: &str) {
        self.jobs.entry(job_name.to_string()).or_insert_with(|| JobInfo {
            last_execution: None,
            consecutive_failures: 0,
            is_active: false,
            total_executions: 0,
            total_failures: 0,
            last_success_timestamp: None,
            last_failure_timestamp: None,
        });
    }

    pub fn start_execution(&mut self, job_name: &str) {
        if let Some(info) = self.jobs.get_mut(job_name) {
            info.is_active = true;
            JOB_ACTIVE_STATUS.with_label_values(&[job_name]).set(1);
        }
    }

    pub fn complete_execution(&mut self, execution: JobExecution) {
        let job_name = &execution.job_name;
        
        if let Some(info) = self.jobs.get_mut(job_name) {
            info.is_active = false;
            info.last_execution = Some(execution.clone());
            info.total_executions += 1;

            let now = chrono::Utc::now().timestamp();

            match &execution.status {
                JobStatus::Success => {
                    info.consecutive_failures = 0;
                    info.last_success_timestamp = Some(now);
                    JOB_LAST_SUCCESS_TIMESTAMP.with_label_values(&[job_name]).set(now as f64);
                    JOB_CONSECUTIVE_FAILURES.with_label_values(&[job_name]).set(0);
                }
                JobStatus::Failed(error) => {
                    info.consecutive_failures += 1;
                    info.total_failures += 1;
                    info.last_failure_timestamp = Some(now);
                    JOB_LAST_FAILURE_TIMESTAMP.with_label_values(&[job_name]).set(now as f64);
                    JOB_CONSECUTIVE_FAILURES.with_label_values(&[job_name]).set(info.consecutive_failures as f64);
                }
                JobStatus::Timeout => {
                    info.consecutive_failures += 1;
                    info.total_failures += 1;
                    info.last_failure_timestamp = Some(now);
                    JOB_LAST_FAILURE_TIMESTAMP.with_label_values(&[job_name]).set(now as f64);
                    JOB_CONSECUTIVE_FAILURES.with_label_values(&[job_name]).set(info.consecutive_failures as f64);
                }
                JobStatus::Running => {} // Still running, shouldn't happen here
            }

            JOB_ACTIVE_STATUS.with_label_values(&[job_name]).set(0);
        }
    }

    pub fn get_job_info(&self, job_name: &str) -> Option<&JobInfo> {
        self.jobs.get(job_name)
    }

    pub fn get_all_jobs(&self) -> &std::collections::HashMap<String, JobInfo> {
        &self.jobs
    }
}

/// Job metrics collector for instrumenting job executions
pub struct JobMetricsCollector {
    job_name: String,
    start_time: Instant,
}

impl JobMetricsCollector {
    pub fn new(job_name: &str) -> Self {
        let collector = Self {
            job_name: job_name.to_string(),
            start_time: Instant::now(),
        };

        // Register job if not already registered
        if let Ok(mut registry) = JOB_REGISTRY.try_write() {
            registry.register_job(job_name);
            registry.start_execution(job_name);
        }

        // Log job start
        info!("Starting job execution: {}", job_name);

        collector
    }

    pub fn complete_success(self) {
        let duration = self.start_time.elapsed();
        let job_name = &self.job_name;

        // Record metrics
        JOB_EXECUTIONS_TOTAL.with_label_values(&[job_name, "success"]).inc();
        JOB_DURATION_SECONDS.with_label_values(&[job_name]).observe(duration.as_secs_f64());

        // Update registry
        if let Ok(mut registry) = JOB_REGISTRY.try_write() {
            let execution = JobExecution::new(job_name.clone()).complete_success();
            registry.complete_execution(execution);
        }

        // Log success
        info!(
            job = %job_name,
            duration_sec = duration.as_secs_f64(),
            "Job completed successfully"
        );
    }

    pub fn complete_failure(self, error: &str) {
        let duration = self.start_time.elapsed();
        let job_name = &self.job_name;
        let error_type = self.classify_error(error);

        // Record metrics
        JOB_EXECUTIONS_TOTAL.with_label_values(&[job_name, "failure"]).inc();
        JOB_FAILURES_TOTAL.with_label_values(&[job_name, &error_type]).inc();
        JOB_DURATION_SECONDS.with_label_values(&[job_name]).observe(duration.as_secs_f64());

        // Update registry
        if let Ok(mut registry) = JOB_REGISTRY.try_write() {
            let execution = JobExecution::new(job_name.clone()).complete_failure(error.to_string());
            registry.complete_execution(execution);
        }

        // Log failure
        error!(
            job = %job_name,
            duration_sec = duration.as_secs_f64(),
            error_type = %error_type,
            error = %error,
            "Job execution failed"
        );
    }

    pub fn complete_timeout(self) {
        let duration = self.start_time.elapsed();
        let job_name = &self.job_name;

        // Record metrics
        JOB_EXECUTIONS_TOTAL.with_label_values(&[job_name, "timeout"]).inc();
        JOB_FAILURES_TOTAL.with_label_values(&[job_name, "timeout"]).inc();
        JOB_DURATION_SECONDS.with_label_values(&[job_name]).observe(duration.as_secs_f64());

        // Update registry
        if let Ok(mut registry) = JOB_REGISTRY.try_write() {
            let execution = JobExecution::new(job_name.clone()).complete_timeout();
            registry.complete_execution(execution);
        }

        // Log timeout
        warn!(
            job = %job_name,
            duration_sec = duration.as_secs_f64(),
            "Job execution timed out"
        );
    }

    fn classify_error(&self, error: &str) -> String {
        let error_lower = error.to_lowercase();
        
        if error_lower.contains("timeout") || error_lower.contains("timed out") {
            "timeout".to_string()
        } else if error_lower.contains("connection") || error_lower.contains("network") {
            "network".to_string()
        } else if error_lower.contains("database") || error_lower.contains("sql") {
            "database".to_string()
        } else if error_lower.contains("cache") || error_lower.contains("redis") {
            "cache".to_string()
        } else if error_lower.contains("rpc") || error_lower.contains("stellar") {
            "rpc".to_string()
        } else if error_lower.contains("parse") || error_lower.contains("invalid") {
            "validation".to_string()
        } else if error_lower.contains("permission") || error_lower.contains("unauthorized") {
            "permission".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

/// Macro for easy job instrumentation
#[macro_export]
macro_rules! instrument_job {
    ($job_name:expr, $async_block:block) => {{
        let _metrics = $crate::observability::job_metrics::JobMetricsCollector::new($job_name);
        
        match async move $async_block.await {
            Ok(_) => {
                _metrics.complete_success();
                Ok(())
            }
            Err(e) => {
                _metrics.complete_failure(&e.to_string());
                Err(e)
            }
        }
    }};
}

/// Helper function to get job status summary
pub async fn get_job_status_summary() -> serde_json::Value {
    let registry = JOB_REGISTRY.read().await;
    let mut jobs = serde_json::Map::new();

    for (name, info) in registry.get_all_jobs() {
        let job_status = serde_json::json!({
            "name": name,
            "is_active": info.is_active,
            "total_executions": info.total_executions,
            "total_failures": info.total_failures,
            "consecutive_failures": info.consecutive_failures,
            "last_success_timestamp": info.last_success_timestamp,
            "last_failure_timestamp": info.last_failure_timestamp,
            "last_execution": info.last_execution.as_ref().map(|exec| {
                match &exec.status {
                    JobStatus::Success => {
                        serde_json::json!({
                            "status": "success",
                            "started_at": exec.started_at.elapsed().as_secs(),
                            "duration_ms": exec.duration.map(|d| d.as_millis()),
                            "completed_at": exec.completed_at.map(|t| t.elapsed().as_secs())
                        })
                    }
                    JobStatus::Failed(err) => {
                        serde_json::json!({
                            "status": "failed",
                            "started_at": exec.started_at.elapsed().as_secs(),
                            "duration_ms": exec.duration.map(|d| d.as_millis()),
                            "completed_at": exec.completed_at.map(|t| t.elapsed().as_secs()),
                            "error": err
                        })
                    }
                    JobStatus::Timeout => {
                        serde_json::json!({
                            "status": "timeout",
                            "started_at": exec.started_at.elapsed().as_secs(),
                            "duration_ms": exec.duration.map(|d| d.as_millis()),
                            "completed_at": exec.completed_at.map(|t| t.elapsed().as_secs())
                        })
                    }
                    JobStatus::Running => {
                        serde_json::json!({
                            "status": "running",
                            "started_at": exec.started_at.elapsed().as_secs(),
                            "duration_ms": exec.duration.map(|d| d.as_millis())
                        })
                    }
                }
            })
        });

        jobs.insert(name.clone(), job_status);
    }

    serde_json::json!({
        "jobs": jobs,
        "total_jobs": registry.get_all_jobs().len(),
        "active_jobs": registry.get_all_jobs().values().filter(|info| info.is_active).count(),
        "timestamp": chrono::Utc::now().timestamp()
    })
}
