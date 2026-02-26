use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    SuccessRateDrop,
    LatencyIncrease,
    LiquidityDecrease,
    AnchorStatusChange,
    AnchorMetricChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: AlertType,
    pub corridor_id: Option<String>,
    pub anchor_id: Option<String>,
    pub message: String,
    pub old_value: f64,
    pub new_value: f64,
    pub timestamp: String,
}

pub struct AlertManager {
    tx: broadcast::Sender<Alert>,
    webhook_event_service: Option<Arc<crate::services::webhook_event_service::WebhookEventService>>,
}

impl AlertManager {
    pub fn new() -> (Self, broadcast::Receiver<Alert>) {
        let (tx, rx) = broadcast::channel(100);
        (
            Self {
                tx,
                webhook_event_service: None,
            },
            rx,
        )
    }

    pub fn new_with_webhooks(
        webhook_event_service: Arc<crate::services::webhook_event_service::WebhookEventService>,
    ) -> (Self, broadcast::Receiver<Alert>) {
        let (tx, rx) = broadcast::channel(100);
        (
            Self {
                tx,
                webhook_event_service: Some(webhook_event_service),
            },
            rx,
        )
    }

    pub fn check_and_alert(
        &self,
        corridor_id: &str,
        old_success: f64,
        new_success: f64,
        old_latency: f64,
        new_latency: f64,
        old_liquidity: f64,
        new_liquidity: f64,
    ) {
        if new_success < old_success - 10.0 {
            let _ = self.tx.send(Alert {
                alert_type: AlertType::SuccessRateDrop,
                corridor_id: Some(corridor_id.to_string()),
                anchor_id: None,
                message: format!(
                    "Success rate dropped from {:.1}% to {:.1}%",
                    old_success, new_success
                ),
                old_value: old_success,
                new_value: new_success,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        if new_latency > old_latency * 1.5 {
            let _ = self.tx.send(Alert {
                alert_type: AlertType::LatencyIncrease,
                corridor_id: Some(corridor_id.to_string()),
                anchor_id: None,
                message: format!(
                    "Latency increased from {:.0}ms to {:.0}ms",
                    old_latency, new_latency
                ),
                old_value: old_latency,
                new_value: new_latency,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        if new_liquidity < old_liquidity * 0.7 {
            let _ = self.tx.send(Alert {
                alert_type: AlertType::LiquidityDecrease,
                corridor_id: Some(corridor_id.to_string()),
                anchor_id: None,
                message: format!(
                    "Liquidity decreased from ${:.0} to ${:.0}",
                    old_liquidity, new_liquidity
                ),
                old_value: old_liquidity,
                new_value: new_liquidity,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Alert> {
        self.tx.subscribe()
    }

    pub fn send_anchor_alert(
        &self,
        alert_type: AlertType,
        anchor_id: &str,
        message: String,
        old_value: f64,
        new_value: f64,
    ) {
        let alert = Alert {
            alert_type: alert_type.clone(),
            corridor_id: None,
            anchor_id: Some(anchor_id.to_string()),
            message: message.clone(),
            old_value,
            new_value,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let _ = self.tx.send(alert);

        // Trigger webhook event for anchor status change
        if let Some(webhook_service) = &self.webhook_event_service {
            let old_status = if old_value > 90.0 {
                "healthy"
            } else {
                "degraded"
            };
            let new_status = if new_value > 90.0 {
                "healthy"
            } else {
                "degraded"
            };

            tokio::spawn({
                let webhook_service = webhook_service.clone();
                let anchor_id = anchor_id.to_string();
                let message_clone = message.clone();
                async move {
                    if let Err(e) = webhook_service
                        .trigger_anchor_status_changed(
                            &anchor_id, &anchor_id, // Using anchor_id as name for now
                            old_status, new_status, new_value,
                            0, // failed_txn_count - would need to be tracked separately
                        )
                        .await
                    {
                        tracing::error!("Failed to trigger anchor status webhook: {}", e);
                    }
                }
            });
        }
    }
}
