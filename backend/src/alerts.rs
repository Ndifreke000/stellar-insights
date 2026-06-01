use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    SuccessRateDrop,
    LatencyIncrease,
    LiquidityDecrease,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: AlertType,
    pub corridor_id: String,
    pub message: String,
    pub old_value: f64,
    pub new_value: f64,
    pub timestamp: String,
}

pub struct AlertManager {
    tx: broadcast::Sender<Alert>,
}

impl AlertManager {
    pub fn new() -> (Self, broadcast::Receiver<Alert>) {
        let (tx, rx) = broadcast::channel(100);
        (Self { tx }, rx)
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
                corridor_id: corridor_id.to_string(),
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
                corridor_id: corridor_id.to_string(),
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
                corridor_id: corridor_id.to_string(),
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
}
