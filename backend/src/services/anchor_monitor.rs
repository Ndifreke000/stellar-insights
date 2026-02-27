use crate::alerts::{Alert, AlertManager, AlertType};
use crate::database::Database;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub struct AnchorMonitor {
    db: Arc<Database>,
    alert_manager: Arc<AlertManager>,
    last_metrics: Arc<tokio::sync::RwLock<HashMap<String, AnchorMetrics>>>,
}

#[derive(Clone, Debug)]
struct AnchorMetrics {
    transaction_count: f64,
    success_rate: f64,
    avg_latency: f64,
}

impl AnchorMonitor {
    pub fn new(db: Arc<Database>, alert_manager: Arc<AlertManager>) -> Self {
        Self {
            db,
            alert_manager,
            last_metrics: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(self) {
        let mut check_interval = interval(Duration::from_secs(300)); // Check every 5 minutes
        tracing::info!("Anchor monitor started");

        loop {
            check_interval.tick().await;
            if let Err(e) = self.check_anchors().await {
                tracing::error!("Anchor monitoring failed: {}", e);
            }
        }
    }

    async fn check_anchors(&self) -> Result<()> {
        let anchors = self.db.get_all_anchors().await?;

        for anchor in anchors {
            // Get metrics from anchor_metrics_history or calculate from transactions
            let current_metrics = AnchorMetrics {
                transaction_count: 0.0, // TODO: Calculate from transactions
                success_rate: 0.0,      // TODO: Calculate from transactions
                avg_latency: 0.0,       // TODO: Calculate from transactions
            };

            let mut last_metrics = self.last_metrics.write().await;

            if let Some(prev_metrics) = last_metrics.get(&anchor.id) {
                // Check for significant changes
                if current_metrics.success_rate < prev_metrics.success_rate - 10.0 {
                    self.alert_manager.send_anchor_alert(
                        AlertType::AnchorMetricChange,
                        &anchor.id,
                        format!(
                            "Anchor '{}' success rate dropped from {:.1}% to {:.1}%",
                            anchor.name, prev_metrics.success_rate, current_metrics.success_rate
                        ),
                        prev_metrics.success_rate,
                        current_metrics.success_rate,
                    );
                }

                if current_metrics.avg_latency > prev_metrics.avg_latency * 1.5 {
                    self.alert_manager.send_anchor_alert(
                        AlertType::AnchorMetricChange,
                        &anchor.id,
                        format!(
                            "Anchor '{}' latency increased from {:.0}ms to {:.0}ms",
                            anchor.name, prev_metrics.avg_latency, current_metrics.avg_latency
                        ),
                        prev_metrics.avg_latency,
                        current_metrics.avg_latency,
                    );
                }
            }

            last_metrics.insert(anchor.id.clone(), current_metrics);
        }

        Ok(())
    }
}
