use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[derive(utoipa::ToSchema)]
pub struct CorridorPerformanceSnapshot {
    pub id: String,
    pub corridor_key: String,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub destination_asset_code: String,
    pub destination_asset_issuer: String,
    pub success_rate: f64,
    pub avg_settlement_latency_ms: f64,
    pub liquidity_depth_usd: f64,
    pub volume_usd: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub snapshot_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[derive(utoipa::ToSchema)]
pub struct CorridorAlertConfig {
    pub id: String,
    pub user_id: String,
    pub corridor_key: Option<String>,
    pub name: String,
    pub success_rate_threshold: Option<f64>,
    pub latency_threshold_ms: Option<f64>,
    pub liquidity_threshold_usd: Option<f64>,
    pub success_rate_drop_pct: f64,
    pub latency_increase_pct: f64,
    pub liquidity_drop_pct: f64,
    pub cooldown_seconds: i32,
    pub notify_email: bool,
    pub notify_webhook: bool,
    pub notify_in_app: bool,
    pub notify_slack: bool,
    pub notify_telegram: bool,
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[derive(utoipa::ToSchema)]
pub struct CorridorAlertEvent {
    pub id: String,
    pub config_id: String,
    pub user_id: String,
    pub corridor_key: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub old_value: Option<f64>,
    pub new_value: Option<f64>,
    pub threshold_value: Option<f64>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub triggered_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[derive(utoipa::ToSchema)]
pub struct CreateCorridorAlertConfigRequest {
    #[validate(length(max = 256, message = "corridor_key must not exceed 256 characters"))]
    pub corridor_key: Option<String>,
    #[validate(length(min = 1, max = 128, message = "name must be between 1 and 128 characters"))]
    pub name: String,
    pub success_rate_threshold: Option<f64>,
    pub latency_threshold_ms: Option<f64>,
    pub liquidity_threshold_usd: Option<f64>,
    #[validate(range(min = 0.1, max = 100.0, message = "success_rate_drop_pct must be between 0.1 and 100"))]
    pub success_rate_drop_pct: Option<f64>,
    #[validate(range(min = 0.1, max = 1000.0, message = "latency_increase_pct must be between 0.1 and 1000"))]
    pub latency_increase_pct: Option<f64>,
    #[validate(range(min = 0.1, max = 100.0, message = "liquidity_drop_pct must be between 0.1 and 100"))]
    pub liquidity_drop_pct: Option<f64>,
    #[validate(range(min = 0, max = 86400, message = "cooldown_seconds must be between 0 and 86400"))]
    pub cooldown_seconds: Option<i32>,
    pub notify_email: Option<bool>,
    pub notify_webhook: Option<bool>,
    pub notify_in_app: Option<bool>,
    pub notify_slack: Option<bool>,
    pub notify_telegram: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[derive(utoipa::ToSchema)]
pub struct UpdateCorridorAlertConfigRequest {
    pub name: Option<String>,
    pub success_rate_threshold: Option<f64>,
    pub latency_threshold_ms: Option<f64>,
    pub liquidity_threshold_usd: Option<f64>,
    pub success_rate_drop_pct: Option<f64>,
    pub latency_increase_pct: Option<f64>,
    pub liquidity_drop_pct: Option<f64>,
    pub cooldown_seconds: Option<i32>,
    pub notify_email: Option<bool>,
    pub notify_webhook: Option<bool>,
    pub notify_in_app: Option<bool>,
    pub notify_slack: Option<bool>,
    pub notify_telegram: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct CorridorPerformanceSummary {
    pub corridor_key: String,
    pub current_success_rate: f64,
    pub previous_success_rate: Option<f64>,
    pub current_latency_ms: f64,
    pub previous_latency_ms: Option<f64>,
    pub current_liquidity_usd: f64,
    pub previous_liquidity_usd: Option<f64>,
    pub success_rate_trend: f64,
    pub latency_trend: f64,
    pub liquidity_trend: f64,
    pub alert_count_24h: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(utoipa::ToSchema)]
pub struct CorridorPerformanceTimeline {
    pub corridor_key: String,
    pub snapshots: Vec<CorridorPerformanceSnapshot>,
    pub alerts: Vec<CorridorAlertEvent>,
}
