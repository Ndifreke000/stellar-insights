use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use async_trait::async_trait;

use crate::observability::job_metrics::{JobRegistry, JobStatus};

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
            consecutive_failure_threshold: std::env::var("JOB_ALERTS_CONSECUTIVE_FAILURE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            success_rate_threshold: std::env::var("JOB_ALERTS_SUCCESS_RATE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(80.0),
            last_success_threshold_seconds: std::env::var("JOB_ALERTS_LAST_SUCCESS_THRESHOLD_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            min_executions_for_rate_check: std::env::var("JOB_ALERTS_MIN_EXECUTIONS_FOR_RATE_CHECK")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            alert_cooldown_seconds: std::env::var("JOB_ALERTS_COOLDOWN_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
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
            JobAlertSeverity::Info => info!(job = %alert.job_name, alert_type = ?alert.alert_type, "{}", alert.message),
            JobAlertSeverity::Warning => warn!(job = %alert.job_name, alert_type = ?alert.alert_type, "{}", alert.message),
            JobAlertSeverity::Critical => error!(job = %alert.job_name, alert_type = ?alert.alert_type, "{}", alert.message),
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

    /// Evaluate if a job should trigger an alert
    async fn evaluate_job_alert(&self, job_name: &str, job_info: &crate::observability::job_metrics::JobInfo) -> Option<JobAlert> {
        // Check cooldown period
        if let Ok(last_alerts) = self.last_alerts.try_read() {
            if let Some(last_alert_time) = last_alerts.get(job_name) {
                if last_alert_time.elapsed() < Duration::from_secs(self.config.alert_cooldown_seconds) {
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
            let success_rate = ((job_info.total_executions - job_info.total_failures) as f64 / job_info.total_executions as f64) * 100.0;
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

        // Check if job is stuck (no recent success but should be running)
        if let Some(last_success) = job_info.last_success_timestamp {
            let now = chrono::Utc::now().timestamp();
            let time_since_success = now - last_success;
            
            if time_since_success > self.config.last_success_threshold_seconds as i64 && job_info.total_failures > 0 {
                return Some(JobAlert {
                    job_name: job_name.to_string(),
                    alert_type: JobAlertType::StuckJob,
                    severity: if time_since_success > (self.config.last_success_threshold_seconds as i64 * 2) {
                        JobAlertSeverity::Critical
                    } else {
                        JobAlertSeverity::Warning
                    },
                    message: format!(
                        "Job hasn't succeeded in {} seconds (threshold: {})",
                        time_since_success, self.config.last_success_threshold_seconds
                    ),
                    timestamp: now,
                    metadata: serde_json::json!({
                        "last_success_timestamp": last_success,
                        "time_since_success_seconds": time_since_success,
                        "consecutive_failures": job_info.consecutive_failures
                    }),
                });
            }
        }

        None
    }

    /// Send alert to all handlers
    async fn send_alert(&self, alert: JobAlert) {
        debug!("Sending job alert: {:?}", alert);

        // Update last alert time
        if let Ok(mut last_alerts) = self.last_alerts.try_write() {
            last_alerts.insert(alert.job_name.clone(), Instant::now());
        }

        // Send to all handlers
        for handler in &self.handlers {
            handler.send_alert(&alert).await;
        }
    }

    /// Get alert statistics
    pub async fn get_alert_stats(&self) -> serde_json::Value {
        let last_alerts = self.last_alerts.read().await;
        let now = Instant::now();

        let mut recent_alerts = 0usize;
        let mut cooldown_alerts = 0usize;

        for (_, last_alert_time) in last_alerts.iter() {
            if last_alert_time.elapsed() < Duration::from_secs(3600) {
                recent_alerts += 1;
            }
            if last_alert_time.elapsed() < Duration::from_secs(self.config.alert_cooldown_seconds) {
                cooldown_alerts += 1;
            }
        }

        serde_json::json!({
            "config": {
                "enabled": self.config.enabled,
                "consecutive_failure_threshold": self.config.consecutive_failure_threshold,
                "success_rate_threshold": self.config.success_rate_threshold,
                "last_success_threshold_seconds": self.config.last_success_threshold_seconds,
                "min_executions_for_rate_check": self.config.min_executions_for_rate_check,
                "alert_cooldown_seconds": self.config.alert_cooldown_seconds
            },
            "stats": {
                "total_tracked_jobs": last_alerts.len(),
                "recent_alerts_last_hour": recent_alerts,
                "jobs_in_cooldown": cooldown_alerts,
                "handlers_count": self.handlers.len()
            },
            "timestamp": chrono::Utc::now().timestamp()
        })
    }
}

/// Background job for alert checking
pub struct JobAlertChecker {
    alert_manager: Arc<JobAlertManager>,
    check_interval: Duration,
}

impl JobAlertChecker {
    pub fn new(alert_manager: Arc<JobAlertManager>) -> Self {
        Self {
            alert_manager,
            check_interval: Duration::from_secs(60), // Check every minute
        }
    }

    pub async fn start(self: Arc<Self>) {
        info!("Starting job alert checker (interval: {}s)", self.check_interval.as_secs());

        let mut interval = tokio::time::interval(self.check_interval);
        interval.tick().await; // Skip first immediate tick

        loop {
            interval.tick().await;

            // Get job registry and check alerts
            if let Ok(job_registry) = crate::observability::job_metrics::JOB_REGISTRY.try_read() {
                self.alert_manager.check_job_alerts(&*job_registry).await;
            }
        }
    }
}

/// Webhook alert handler (sends alerts to webhook URL)
pub struct WebhookAlertHandler {
    webhook_url: String,
    timeout: Duration,
}

impl WebhookAlertHandler {
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url,
            timeout: Duration::from_secs(10),
        }
    }
}

impl AlertHandler for WebhookAlertHandler {
    async fn send_alert(&self, alert: &JobAlert) {
        let client = reqwest::Client::new();
        
        let payload = serde_json::json!({
            "alert": {
                "job_name": alert.job_name,
                "alert_type": format!("{:?}", alert.alert_type),
                "severity": format!("{:?}", alert.severity),
                "message": alert.message,
                "timestamp": alert.timestamp,
                "metadata": alert.metadata
            },
            "service": "stellar-insights-backend",
            "timestamp": chrono::Utc::now().timestamp()
        });

        match client
            .post(&self.webhook_url)
            .json(&payload)
            .timeout(self.timeout)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Successfully sent webhook alert for job: {}", alert.job_name);
                } else {
                    error!("Failed to send webhook alert for job {}: {}", alert.job_name, response.status());
                }
            }
            Err(e) => {
                error!("Error sending webhook alert for job {}: {}", alert.job_name, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_alert_config_default() {
        let config = JobAlertConfig::default();
        assert_eq!(config.enabled, true);
        assert_eq!(config.consecutive_failure_threshold, 3);
        assert_eq!(config.success_rate_threshold, 80.0);
        assert_eq!(config.last_success_threshold_seconds, 3600);
        assert_eq!(config.min_executions_for_rate_check, 5);
        assert_eq!(config.alert_cooldown_seconds, 1800);
    }

    #[test]
    fn test_job_alert_severity_ordering() {
        assert!(JobAlertSeverity::Critical > JobAlertSeverity::Warning);
        assert!(JobAlertSeverity::Warning > JobAlertSeverity::Info);
    }
}
