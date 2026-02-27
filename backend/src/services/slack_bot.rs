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
    /// Create a new `SlackBotService`
    #[must_use] 
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
        let (title, color, emoji) = match alert.alert_type {
            AlertType::SuccessRateDrop => ("Success Rate Drop", "#E01E5A", "🔴"),
            AlertType::LatencyIncrease => ("Latency Increase", "#ECB22E", "🟡"),
            AlertType::LiquidityDecrease => ("Liquidity Decrease", "#E8912D", "🟠"),
            AlertType::AnchorStatusChange => ("Anchor Status Change", "#36A64F", "🔵"),
            AlertType::AnchorMetricChange => ("Anchor Metric Change", "#2EB67D", "📊"),
        };

        let mut fields = vec![
            serde_json::json!({
                "title": "Timestamp",
                "value": alert.timestamp,
                "short": true
            }),
            serde_json::json!({
                "title": "Previous Value",
                "value": format!("{:.2}", alert.old_value),
                "short": true
            }),
            serde_json::json!({
                "title": "New Value",
                "value": format!("{:.2}", alert.new_value),
                "short": true
            }),
        ];

        if let Some(ref corridor_id) = alert.corridor_id {
            fields.insert(
                0,
                serde_json::json!({
                    "title": "Corridor",
                    "value": corridor_id,
                    "short": true
                }),
            );
        }

        if let Some(ref anchor_id) = alert.anchor_id {
            fields.insert(
                0,
                serde_json::json!({
                    "title": "Anchor",
                    "value": anchor_id,
                    "short": true
                }),
            );
        }

        let payload = serde_json::json!({
            "attachments": [
                {
                    "fallback": format!("{} {}: {}", emoji, title, alert.message),
                    "color": color,
                    "title": format!("{} {}", emoji, title),
                    "text": alert.message,
                    "fields": fields,
                    "footer": "Stellar Insights",
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

        let status: StatusCode = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Slack API returned error status {status}: {error_text}");
        }

        tracing::info!("Alert sent to Slack successfully: {}", alert.message);
        Ok(())
    }
}
