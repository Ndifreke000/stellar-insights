use crate::database::Database;
use crate::models::alerts::AlertHistory;
use reqwest::Client;
use std::sync::Arc;
use tokio::time::{interval, Duration};

pub struct AlertManager {
    db: Arc<Database>,
    http_client: Client,
}

impl AlertManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            http_client: Client::new(),
        }
    }

    pub async fn evaluate_corridor_metrics(
        &self,
        corridor_id: &str,
        metrics: &std::collections::HashMap<&str, f64>,
    ) -> anyhow::Result<()> {
        let rules = self.db.get_all_active_alert_rules().await?;

        for rule in rules {
            // Apply only to rules that either have no specific corridor_id (global) or match this one.
            if let Some(ref r_corridor_id) = rule.corridor_id {
                if r_corridor_id != corridor_id {
                    continue;
                }
            }

            // check if snoozed
            if let Some(snoozed_until) = rule.snoozed_until {
                if chrono::Utc::now() < snoozed_until {
                    continue; // Skip evaluation if rule is currently snoozed
                }
            }

            if let Some(&current_value) = metrics.get(rule.metric_type.as_str()) {
                let is_triggered = match rule.condition.as_str() {
                    "above" => current_value > rule.threshold,
                    "below" => current_value < rule.threshold,
                    "equals" => (current_value - rule.threshold).abs() < f64::EPSILON,
                    _ => false,
                };

                if is_triggered {
                    let message = format!(
                        "Alert! Metric '{}' went {} threshold {}: current value is {:.2}",
                        rule.metric_type, rule.condition, rule.threshold, current_value
                    );

                    // 1. Save to History
                    let history = self.db.insert_alert_history(
                        &rule.id,
                        &rule.user_id,
                        Some(corridor_id.to_string()),
                        &rule.metric_type,
                        current_value,
                        rule.threshold,
                        &rule.condition,
                        &message,
                    ).await?;

                    // 2. Transmit via requested channels
                    if rule.notify_email {
                        self.send_email_alert(&rule.user_id, &message).await;
                    }

                    if rule.notify_webhook {
                        self.send_webhook_alert(&rule.user_id, &history).await;
                    }

                    if rule.notify_in_app {
                        // Covered by history insertion
                    }
                }
            }
        }
        Ok(())
    }

    async fn send_email_alert(&self, user_id: &str, message: &str) {
        // Mocking email dispatcher for brevity 
        tracing::info!("Sending EMAIL alert to user {}: {}", user_id, message);
    }

    async fn send_webhook_alert(&self, user_id: &str, history: &AlertHistory) {
        // Mocking webhook dispatcher for brevity
        tracing::info!("Sending WEBHOOK alert to user {}", user_id);
    }
}
