//! Alert Service for Contract Event Monitoring
//!
//! Sends alerts when verification failures or anomalies are detected.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert types for contract events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    VerificationFailed {
        epoch: u64,
        expected_hash: String,
        actual_hash: String,
    },
    MissingSnapshot {
        epoch: u64,
    },
    ListenerFailure {
        error: String,
    },
    UnauthorizedSubmission {
        epoch: u64,
        submitter: String,
    },
}

/// Alert message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Destination channels for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannel {
    Email(String),
    Slack(String),
    Webhook(String),
}

/// Service for sending alerts
pub struct AlertService {
    email_service: Option<crate::email::service::EmailService>,
    slack_client: reqwest::Client,
    webhook_service: Option<crate::webhooks::WebhookService>,
}

impl AlertService {
    /// Create a new alert service
    #[must_use]
    pub fn new(
        email_service: Option<crate::email::service::EmailService>,
        webhook_service: Option<crate::webhooks::WebhookService>,
    ) -> Self {
        Self {
            email_service,
            slack_client: reqwest::Client::new(),
            webhook_service,
        }
    }

    /// Send an alert to all configured channels
    pub async fn send_alert(&self, alert: Alert) -> Result<()> {
        match alert.severity {
            AlertSeverity::Critical | AlertSeverity::Error => {
                error!(
                    "ALERT [{:?}]: {} - {:?}",
                    alert.severity, alert.message, alert.alert_type
                );
            }
            AlertSeverity::Warning => {
                warn!(
                    "ALERT [{:?}]: {} - {:?}",
                    alert.severity, alert.message, alert.alert_type
                );
            }
            AlertSeverity::Info => {
                info!(
                    "ALERT [{:?}]: {} - {:?}",
                    alert.severity, alert.message, alert.alert_type
                );
            }
        }

        // Auto-dispatch to default channels if configured in environment
        if let Ok(slack_webhook) = std::env::var("DEFAULT_SLACK_WEBHOOK") {
            let _ = self
                .send_alert_to_channel(&alert, AlertChannel::Slack(slack_webhook))
                .await;
        }

        if let Ok(admin_email) = std::env::var("ADMIN_EMAIL") {
            let _ = self
                .send_alert_to_channel(&alert, AlertChannel::Email(admin_email))
                .await;
        }

        Ok(())
    }

    /// Send an alert to a specific channel
    pub async fn send_alert_to_channel(&self, alert: &Alert, channel: AlertChannel) -> Result<()> {
        match channel {
            AlertChannel::Email(to) => {
                if let Some(ref service) = self.email_service {
                    let subject = format!("Stellar Insights Alert: {:?}", alert.severity);
                    let body = format!(
                        "<h2>Alert</h2><p>{}</p><p>Type: {:?}</p>",
                        alert.message, alert.alert_type
                    );
                    service.send_html(&to, &subject, &body)?;
                } else {
                    warn!("Email service not configured, skipping alert to {}", to);
                }
            }
            AlertChannel::Slack(webhook_url) => {
                let payload = serde_json::json!({
                    "text": format!("*ALERT [{:?}]*\n{}\n`{:?}`", alert.severity, alert.message, alert.alert_type)
                });
                self.slack_client
                    .post(&webhook_url)
                    .json(&payload)
                    .send()
                    .await?;
            }
            AlertChannel::Webhook(webhook_id) => {
                if let Some(ref service) = self.webhook_service {
                    service
                        .create_webhook_event(
                            &webhook_id,
                            "contract_alert",
                            serde_json::to_value(alert)?,
                        )
                        .await?;
                } else {
                    warn!(
                        "Webhook service not configured, skipping alert to {}",
                        webhook_id
                    );
                }
            }
        }
        Ok(())
    }

    /// Send verification failure alert
    pub async fn alert_verification_failed(
        &self,
        epoch: u64,
        expected_hash: String,
        actual_hash: String,
    ) -> Result<()> {
        let alert = Alert {
            alert_type: AlertType::VerificationFailed {
                epoch,
                expected_hash: expected_hash.clone(),
                actual_hash: actual_hash.clone(),
            },
            severity: AlertSeverity::Critical,
            message: format!(
                "Snapshot verification failed for epoch {epoch}. Expected hash: {expected_hash}, Actual hash: {actual_hash}"
            ),
            timestamp: chrono::Utc::now(),
        };

        self.send_alert(alert).await
    }

    /// Send missing snapshot alert
    pub async fn alert_missing_snapshot(&self, epoch: u64) -> Result<()> {
        let alert = Alert {
            alert_type: AlertType::MissingSnapshot { epoch },
            severity: AlertSeverity::Warning,
            message: format!("No snapshot found in database for epoch {epoch}"),
            timestamp: chrono::Utc::now(),
        };

        self.send_alert(alert).await
    }

    /// Send listener failure alert
    pub async fn alert_listener_failure(&self, error: String) -> Result<()> {
        let alert = Alert {
            alert_type: AlertType::ListenerFailure {
                error: error.clone(),
            },
            severity: AlertSeverity::Error,
            message: format!("Contract event listener failed: {error}"),
            timestamp: chrono::Utc::now(),
        };

        self.send_alert(alert).await
    }

    /// Send unauthorized submission alert
    pub async fn alert_unauthorized_submission(&self, epoch: u64, submitter: String) -> Result<()> {
        let alert = Alert {
            alert_type: AlertType::UnauthorizedSubmission {
                epoch,
                submitter: submitter.clone(),
            },
            severity: AlertSeverity::Critical,
            message: format!(
                "Unauthorized snapshot submission detected for epoch {epoch} from {submitter}"
            ),
            timestamp: chrono::Utc::now(),
        };

        self.send_alert(alert).await
    }
}

impl Default for AlertService {
    fn default() -> Self {
        Self::new(None, None)
    }
}
