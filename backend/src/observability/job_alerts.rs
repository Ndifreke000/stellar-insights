use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::observability::job_metrics::JobRegistry;

/// Threshold (seconds) at which ingestion lag triggers an alert (5 minutes).
pub const INGESTION_LAG_ALERT_THRESHOLD_SECONDS: u64 = 300;

/// Job alert configuration
#[derive(Debug, Clone)]
pub struct JobAlertConfig {
    /// Enable job failure alerts
    pub enabled: bool,
    /// Number of consecutive failures before alert
    pub consecutive_failure_threshold: u64,
    /// Success rate threshold before alert (percentage)
    pub success_rate_threshold: f64,
    /// Time since last success before alert (seconds)
    pub last_success_threshold_seconds: u64,
    /// Minimum number of executions before checking success rate
    pub min_executions_for_rate_check: u64,
    /// Alert cooldown period (seconds)
    pub alert_cooldown_seconds: u64,
    /// Ingestion lag threshold before alert (seconds)
    pub ingestion_lag_threshold_seconds: u64,
}

impl Default for JobAlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            consecutive_failure_threshold: 3,
            success_rate_threshold: 80.0,
            last_success_threshold_seconds: 3600, // 1 hour
            min_executions_for_rate_check: 5,
            alert_cooldown_seconds: 1800, // 30 minutes
            ingestion_lag_threshold_seconds: INGESTION_LAG_ALERT_THRESHOLD_SECONDS,
        }
    }
}

impl JobAlertConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("JOB_ALERTS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            consecutive_failure_threshold: std::env::var(
                "JOB_ALERTS_CONSECUTIVE_FAILURE_THRESHOLD",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3),
            success_rate_threshold: std::env::var("JOB_ALERTS_SUCCESS_RATE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(80.0),
            last_success_threshold_seconds: std::env::var(
                "JOB_ALERTS_LAST_SUCCESS_THRESHOLD_SECONDS",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600),
            min_executions_for_rate_check: std::env::var(
                "JOB_ALERTS_MIN_EXECUTIONS_FOR_RATE_CHECK",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5),
            alert_cooldown_seconds: std::env::var("JOB_ALERTS_COOLDOWN_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
            ingestion_lag_threshold_seconds: std::env::var(
                "INGESTION_LAG_ALERT_THRESHOLD_SECONDS",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(INGESTION_LAG_ALERT_THRESHOLD_SECONDS),
        }
    }
}

/// Job alert information
#[derive(Debug, Clone)]
pub struct JobAlert {
    pub job_name: String,
    pub alert_type: JobAlertType,
    pub severity: JobAlertSeverity,
    pub message: String,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}

/// Types of job alerts
#[derive(Debug, Clone)]
pub enum JobAlertType {
    ConsecutiveFailures,
    LowSuccessRate,
    StuckJob,
    LongRunningJob,
    JobTimeout,
    UnexpectedError,
    IngestionLag,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobAlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert handler trait
#[async_trait]
pub trait AlertHandler: Send + Sync {
    async fn send_alert(&self, alert: &JobAlert);
}

/// Console alert handler (logs to console)
pub struct ConsoleAlertHandler;

#[async_trait]
impl AlertHandler for ConsoleAlertHandler {
    async fn send_alert(&self, alert: &JobAlert) {
        match alert.severity {
            JobAlertSeverity::Info => {
                info!(job = %alert.job_name, alert_type = ?alert.alert_type, "{}", alert.message)
            }
            JobAlertSeverity::Warning => {
                warn!(job = %alert.job_name, alert_type = ?alert.alert_type, "{}", alert.message)
            }
            JobAlertSeverity::Critical => {
                error!(job = %alert.job_name, alert_type = ?alert.alert_type, "{}", alert.message)
            }
        }
    }
}

/// Job alert manager
pub struct JobAlertManager {
    config: JobAlertConfig,
    handlers: Vec<Box<dyn AlertHandler>>,
    last_alerts: Arc<RwLock<std::collections::HashMap<String, Instant>>>,
}

impl JobAlertManager {
    pub fn new(config: JobAlertConfig) -> Self {
        Self {
            config,
            handlers: vec![Box::new(ConsoleAlertHandler)],
            last_alerts: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_handler(mut self, handler: Box<dyn AlertHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Check for job alerts based on current job status
    pub async fn check_job_alerts(&self, job_registry: &JobRegistry) {
        if !self.config.enabled {
            debug!("Job alerts disabled, skipping alert checks");
            return;
        }

        for (job_name, job_info) in job_registry.get_all_jobs() {
            if let Some(alert) = self.evaluate_job_alert(job_name, job_info).await {
                self.send_alert(alert).await;
            }
        }
    }

    /// Check ingestion lag against the configured threshold and alert if exceeded.
    ///
    /// `lag_seconds` is the number of seconds the ingestion cursor is behind the
    /// network head (as exposed by the `ingestion_lag_seconds` gauge).
    pub async fn check_ingestion_lag(&self, lag_seconds: u64) {
        if !self.config.enabled {
            debug!("Job alerts disabled, skipping ingestion lag check");
            return;
        }

        if lag_seconds <= self.config.ingestion_lag_threshold_seconds {
            return;
        }

        let alert = JobAlert {
            job_name: "ingestion".to_string(),
            alert_type: JobAlertType::IngestionLag,
            severity: if lag_seconds >= self.config.ingestion_lag_threshold_seconds * 2 {
                JobAlertSeverity::Critical
            } else {
                JobAlertSeverity::Warning
            },
            message: format!(
                "Ingestion is {}s behind network head (threshold: {}s)",
                lag_seconds, self.config.ingestion_lag_threshold_seconds
            ),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: serde_json::json!({
                "lag_seconds": lag_seconds,
                "threshold_seconds": self.config.ingestion_lag_threshold_seconds
            }),
        };

        self.send_alert(alert).await;
    }

    /// Evaluate if a job should trigger an alert
    async fn evaluate_job_alert(
        &self,
        job_name: &str,
        job_info: &crate::observability::job_metrics::JobInfo,
    ) -> Option<JobAlert> {
        // Check cooldown period
        if let Ok(last_alerts) = self.last_alerts.try_read() {
            if let Some(last_alert_time) = last_alerts.get(job_name) {
                if last_alert_time.elapsed()
                    < Duration::from_secs(self.config.alert_cooldown_seconds)
                {
                    debug!("Job {} is in alert cooldown period", job_name);
                    return None;
                }
            }
        }

        // Check consecutive failures
        if job_info.consecutive_failures >= self.config.consecutive_failure_threshold {
            return Some(JobAlert {
                job_name: job_name.to_string(),
                alert_type: JobAlertType::ConsecutiveFailures,
                severity: if job_info.consecutive_failures >= 5 {
                    JobAlertSeverity::Critical
                } else {
                    JobAlertSeverity::Warning
                },
                message: format!(
                    "Job has {} consecutive failures (threshold: {})",
                    job_info.consecutive_failures, self.config.consecutive_failure_threshold
                ),
                timestamp: chrono::Utc::now().timestamp(),
                metadata: serde_json::json!({
                    "consecutive_failures": job_info.consecutive_failures,
                    "total_failures": job_info.total_failures,
                    "total_executions": job_info.total_executions
                }),
            });
        }

        // Check success rate
        if job_info.total_executions >= self.config.min_executions_for_rate_check {
            let success_rate = ((job_info.total_executions - job_info.total_failures) as f64
                / job_info.total_executions as f64)
                * 100.0;
            if success_rate < self.config.success_rate_threshold {
                return Some(JobAlert {
                    job_name: job_name.to_string(),
                    alert_type: JobAlertType::LowSuccessRate,
                    severity: if success_rate < 50.0 {
                        JobAlertSeverity::Critical
                    } else {
                        JobAlertSeverity::Warning
                    },
                    message: format!(
                        "Job success rate is {:.1}% (threshold: {:.1}%)",
                        success_rate, self.config.success_rate_threshold
                    ),
                    timestamp: chrono::Utc::now().timestamp(),
                    metadata: serde_json::json!({
                        "success_rate": success_rate,
                        "total_executions": job_info.total_executions,
                        "total_failures": job_info.total_failures,
                        "consecutive_failures": job_info.consecutive_failures
                    }),
                });
            }
        }

        None
    }

    /// Send an alert through all registered handlers
    async fn send_alert(&self, alert: JobAlert) {
        for handler in &self.handlers {
            handler.send_alert(&alert).await;
        }

        // Record alert time for cooldown
        if let Ok(mut last_alerts) = self.last_alerts.try_write() {
            last_alerts.insert(alert.job_name.clone(), Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ingestion_lag_below_threshold_does_not_alert() {
        let manager = JobAlertManager::new(JobAlertConfig::default());
        // Should not panic / should be a no-op below threshold.
        manager.check_ingestion_lag(INGESTION_LAG_ALERT_THRESHOLD_SECONDS).await;
    }

    #[tokio::test]
    async fn ingestion_lag_above_threshold_alerts() {
        let manager = JobAlertManager::new(JobAlertConfig::default());
        manager
            .check_ingestion_lag(INGESTION_LAG_ALERT_THRESHOLD_SECONDS + 1)
            .await;
    }

    #[test]
    fn default_threshold_is_five_minutes() {
        assert_eq!(JobAlertConfig::default().ingestion_lag_threshold_seconds, 300);
    }
}
