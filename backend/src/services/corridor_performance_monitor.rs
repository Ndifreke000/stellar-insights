use crate::alerts::{AlertManager, AlertType};
use crate::database::Database;
use crate::models::corridor_alerts::CorridorAlertConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Background service that monitors corridor performance and triggers alerts.
pub struct CorridorPerformanceMonitor {
    db: Arc<Database>,
    alert_manager: Arc<AlertManager>,
    tx: broadcast::Sender<CorridorPerformanceAlert>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorridorPerformanceAlert {
    pub config_id: String,
    pub corridor_key: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub old_value: Option<f64>,
    pub new_value: Option<f64>,
    pub threshold_value: Option<f64>,
    pub timestamp: String,
}

impl CorridorPerformanceMonitor {
    pub fn new(db: Arc<Database>, alert_manager: Arc<AlertManager>) -> (Self, broadcast::Receiver<CorridorPerformanceAlert>) {
        let (tx, rx) = broadcast::channel(256);
        (
            Self {
                db,
                alert_manager,
                tx,
            },
            rx,
        )
    }

    /// Spawn the background monitoring loop.
    pub fn spawn(self: &Arc<Self>, interval_secs: u64) -> tokio::task::JoinHandle<()> {
        let monitor = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                if let Err(e) = monitor.evaluate_all_corridors().await {
                    tracing::error!(error = %e, "Corridor performance monitor evaluation failed");
                }
            }
        })
    }

    /// Evaluate all corridors against active alert configs.
    async fn evaluate_all_corridors(&self) -> anyhow::Result<()> {
        let configs = self.db.get_all_active_corridor_alert_configs().await?;
        if configs.is_empty() {
            return Ok(());
        }

        // Group configs by corridor_key (None = global monitoring)
        let mut corridor_configs: HashMap<Option<String>, Vec<&CorridorAlertConfig>> = HashMap::new();
        for config in &configs {
            corridor_configs
                .entry(config.corridor_key.clone())
                .or_default()
                .push(config);
        }

        // For corridor-specific configs, check each corridor
        for (corridor_key_opt, configs_for_corridor) in &corridor_configs {
            if let Some(corridor_key) = corridor_key_opt {
                self.evaluate_corridor(corridor_key, configs_for_corridor).await?;
            }
        }

        Ok(())
    }

    /// Evaluate a single corridor against its alert configs.
    async fn evaluate_corridor(
        &self,
        corridor_key: &str,
        configs: &[&CorridorAlertConfig],
    ) -> anyhow::Result<()> {
        let current = match self.db.get_latest_snapshot_for_corridor(corridor_key).await? {
            Some(s) => s,
            None => return Ok(()),
        };

        let previous = self.db.get_previous_snapshot_for_corridor(corridor_key).await?;

        for config in configs {
            // Check cooldown
            if let Some(last_triggered) = config.last_triggered_at {
                let elapsed = chrono::Utc::now()
                    .signed_duration_since(last_triggered)
                    .num_seconds();
                if elapsed < config.cooldown_seconds as i64 {
                    continue;
                }
            }

            let mut triggered_alerts: Vec<(String, String, String, Option<f64>, Option<f64>, Option<f64>)> = Vec::new();

            // Check absolute thresholds
            if let Some(threshold) = config.success_rate_threshold {
                if current.success_rate < threshold {
                    triggered_alerts.push((
                        "success_rate_below".to_string(),
                        "critical".to_string(),
                        format!(
                            "Corridor {} success rate {:.1}% dropped below threshold {:.1}%",
                            corridor_key, current.success_rate * 100.0, threshold * 100.0
                        ),
                        previous.as_ref().map(|p| p.success_rate),
                        Some(current.success_rate),
                        Some(threshold),
                    ));
                }
            }

            if let Some(threshold) = config.latency_threshold_ms {
                if current.avg_settlement_latency_ms > threshold {
                    triggered_alerts.push((
                        "latency_above".to_string(),
                        "warning".to_string(),
                        format!(
                            "Corridor {} latency {:.0}ms exceeded threshold {:.0}ms",
                            corridor_key, current.avg_settlement_latency_ms, threshold
                        ),
                        previous.as_ref().map(|p| p.avg_settlement_latency_ms),
                        Some(current.avg_settlement_latency_ms),
                        Some(threshold),
                    ));
                }
            }

            if let Some(threshold) = config.liquidity_threshold_usd {
                if current.liquidity_depth_usd < threshold {
                    triggered_alerts.push((
                        "liquidity_below".to_string(),
                        "critical".to_string(),
                        format!(
                            "Corridor {} liquidity ${:.0} dropped below threshold ${:.0}",
                            corridor_key, current.liquidity_depth_usd, threshold
                        ),
                        previous.as_ref().map(|p| p.liquidity_depth_usd),
                        Some(current.liquidity_depth_usd),
                        Some(threshold),
                    ));
                }
            }

            // Check relative thresholds (percentage changes)
            if let Some(ref prev) = previous {
                // Success rate drop
                if prev.success_rate > 0.0 {
                    let drop_pct = ((prev.success_rate - current.success_rate) / prev.success_rate) * 100.0;
                    if drop_pct > config.success_rate_drop_pct {
                        triggered_alerts.push((
                            "success_rate_drop".to_string(),
                            if drop_pct > 25.0 { "critical" } else { "warning" }.to_string(),
                            format!(
                                "Corridor {} success rate dropped {:.1}% (from {:.1}% to {:.1}%)",
                                corridor_key, drop_pct, prev.success_rate * 100.0, current.success_rate * 100.0
                            ),
                            Some(prev.success_rate),
                            Some(current.success_rate),
                            Some(config.success_rate_drop_pct),
                        ));
                    }
                }

                // Latency increase
                if prev.avg_settlement_latency_ms > 0.0 {
                    let increase_pct = ((current.avg_settlement_latency_ms - prev.avg_settlement_latency_ms)
                        / prev.avg_settlement_latency_ms)
                        * 100.0;
                    if increase_pct > config.latency_increase_pct {
                        triggered_alerts.push((
                            "latency_increase".to_string(),
                            "warning".to_string(),
                            format!(
                                "Corridor {} latency increased {:.1}% (from {:.0}ms to {:.0}ms)",
                                corridor_key, increase_pct, prev.avg_settlement_latency_ms, current.avg_settlement_latency_ms
                            ),
                            Some(prev.avg_settlement_latency_ms),
                            Some(current.avg_settlement_latency_ms),
                            Some(config.latency_increase_pct),
                        ));
                    }
                }

                // Liquidity drop
                if prev.liquidity_depth_usd > 0.0 {
                    let drop_pct = ((prev.liquidity_depth_usd - current.liquidity_depth_usd)
                        / prev.liquidity_depth_usd)
                        * 100.0;
                    if drop_pct > config.liquidity_drop_pct {
                        triggered_alerts.push((
                            "liquidity_drop".to_string(),
                            if drop_pct > 50.0 { "critical" } else { "warning" }.to_string(),
                            format!(
                                "Corridor {} liquidity dropped {:.1}% (from ${:.0} to ${:.0})",
                                corridor_key, drop_pct, prev.liquidity_depth_usd, current.liquidity_depth_usd
                            ),
                            Some(prev.liquidity_depth_usd),
                            Some(current.liquidity_depth_usd),
                            Some(config.liquidity_drop_pct),
                        ));
                    }
                }
            }

            // Process triggered alerts
            for (alert_type, severity, message, old_val, new_val, threshold) in triggered_alerts {
                let event = self
                    .db
                    .insert_corridor_alert_event(
                        &config.id,
                        &config.user_id,
                        corridor_key,
                        &alert_type,
                        &severity,
                        &message,
                        old_val,
                        new_val,
                        threshold,
                    )
                    .await?;

                self.db.update_last_triggered(&config.id).await?;

                // Broadcast to WebSocket subscribers
                let alert = CorridorPerformanceAlert {
                    config_id: config.id.clone(),
                    corridor_key: corridor_key.to_string(),
                    alert_type: event.alert_type.clone(),
                    severity: event.severity.clone(),
                    message: event.message.clone(),
                    old_value: event.old_value,
                    new_value: event.new_value,
                    threshold_value: event.threshold_value,
                    timestamp: event.triggered_at.to_rfc3339(),
                };
                let _ = self.tx.send(alert);

                // Also send through the core alert manager for webhook/email delivery
                let alert_type_enum = match alert_type.as_str() {
                    "success_rate_below" | "success_rate_drop" => AlertType::SuccessRateDrop,
                    "latency_above" | "latency_increase" => AlertType::LatencyIncrease,
                    "liquidity_below" | "liquidity_drop" => AlertType::LiquidityDecrease,
                    _ => AlertType::SuccessRateDrop,
                };

                self.alert_manager.send_anchor_alert(
                    alert_type_enum,
                    corridor_key,
                    message,
                    old_val.unwrap_or(0.0),
                    new_val.unwrap_or(0.0),
                );

                tracing::info!(
                    corridor_key,
                    alert_type = %event.alert_type,
                    severity = %event.severity,
                    "Corridor performance alert triggered"
                );
            }
        }

        Ok(())
    }

    /// Subscribe to performance alerts.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<CorridorPerformanceAlert> {
        self.tx.subscribe()
    }
}
