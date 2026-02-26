use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertRule {
    pub id: String,
    pub user_id: String,
    pub corridor_id: Option<String>,
    pub metric_type: String, // e.g., "success_rate", "latency", "liquidity"
    pub condition: String,   // e.g., "above", "below", "equals"
    pub threshold: f64,
    pub notify_email: bool,
    pub notify_webhook: bool,
    pub notify_in_app: bool,
    pub is_active: bool,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertHistory {
    pub id: String,
    pub rule_id: String,
    pub user_id: String,
    pub corridor_id: Option<String>,
    pub metric_type: String,
    pub trigger_value: f64,
    pub threshold_value: f64,
    pub condition: String,
    pub message: String,
    pub is_read: bool,
    pub is_dismissed: bool,
    pub triggered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub corridor_id: Option<String>,
    pub metric_type: String,
    pub condition: String,
    pub threshold: f64,
    #[serde(default)]
    pub notify_email: bool,
    #[serde(default)]
    pub notify_webhook: bool,
    #[serde(default = "default_true")]
    pub notify_in_app: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAlertRuleRequest {
    pub corridor_id: Option<String>,
    pub metric_type: Option<String>,
    pub condition: Option<String>,
    pub threshold: Option<f64>,
    pub notify_email: Option<bool>,
    pub notify_webhook: Option<bool>,
    pub notify_in_app: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnoozeAlertRequest {
    pub snoozed_until: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}
