use crate::alerts::{Alert, AlertType};
use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use tokio::sync::broadcast;

/// Slack Bot Service for sending alerts to Slack channels
pub struct SlackBotService {
    webhook_url: String,
    http_client: Client,
    alert_rx: broadcast::Receiver<Alert>,
}

impl SlackBotService {
    /// Create a new SlackBotService
    pub fn new(webhook_url: String, alert_rx: broadcast::Receiver<Alert>) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            webhook_url,
            http_client,
            alert_rx,
        }
    }

    /// Start the slack bot listener loop
    pub async fn start(mut self) {
        tracing::info!("Slack Bot Service started, listening for alerts");

        while let Ok(alert) = self.alert_rx.recv().await {
            if let Err(e) = self.send_alert_to_slack(&alert).await {
                tracing::error!("Failed to send alert to Slack: {}", e);
            }
        }
    }

    /// Send a single alert to Slack
    async fn send_alert_to_slack(&self, alert: &Alert) -> Result<()> {
        let title = match alert.alert_type {
            AlertType::SuccessRateDrop => "🔴 Success Rate Drop",
            AlertType::LatencyIncrease => "🟡 Latency Increase",
            AlertType::LiquidityDecrease => "🟠 Liquidity Decrease",
        };

        let color = match alert.alert_type {
            AlertType::SuccessRateDrop => "#E01E5A",   // Red
            AlertType::LatencyIncrease => "#ECB22E",   // Yellow
            AlertType::LiquidityDecrease => "#E8912D", // Orange
        };

        let payload = serde_json::json!({
            "attachments": [
                {
                    "fallback": format!("{}: {}", title, alert.message),
                    "color": color,
                    "title": title,
                    "text": alert.message,
                    "fields": [
                        {
                            "title": "Corridor",
                            "value": alert.corridor_id,
                            "short": true
                        },
                        {
                            "title": "Timestamp",
                            "value": alert.timestamp,
                            "short": true
                        },
                        {
                            "title": "Previous Value",
                            "value": format!("{:.2}", alert.old_value),
                            "short": true
                        },
                        {
                            "title": "New Value",
                            "value": format!("{:.2}", alert.new_value),
                            "short": true
                        }
                    ],
                    "footer": "Stellar Insights Backend",
                    "ts": chrono::Utc::now().timestamp()
                }
            ]
        });

        let response = self
            .http_client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Slack webhook")?;

        // Store the status code before consuming the response body
        let status: StatusCode = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Slack API returned error status {}: {}", status, error_text);
        }

        tracing::info!("Alert sent to Slack successfully: {}", alert.message);
        Ok(())
    }
}
