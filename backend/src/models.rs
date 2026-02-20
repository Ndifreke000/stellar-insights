use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod corridor;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    SuccessRate,
    Volume,
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Anchor {
    pub id: String,
    pub name: String,
    pub stellar_account: String,
    pub home_domain: Option<String>,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub total_volume_usd: f64,
    pub avg_settlement_time_ms: i32,
    pub reliability_score: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub id: String,
    pub anchor_id: String,
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_supply: Option<f64>,
    pub num_holders: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AnchorMetricsHistory {
    pub id: String,
    pub anchor_id: String,
    pub timestamp: DateTime<Utc>,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub reliability_score: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub volume_usd: Option<f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMetrics {
    pub success_rate: f64,
    pub failure_rate: f64,
    pub reliability_score: f64,
    pub total_transactions: i64,
    pub successful_transactions: i64,
    pub failed_transactions: i64,
    pub avg_settlement_time_ms: Option<i32>,
    pub status: AnchorStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnchorStatus {
    Green,  // >98% success, <1% failures
    Yellow, // 95-98% success, 1-5% failures
    Red,    // <95% success, >5% failures
}

impl AnchorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnchorStatus::Green => "green",
            AnchorStatus::Yellow => "yellow",
            AnchorStatus::Red => "red",
        }
    }

    pub fn from_metrics(success_rate: f64, failure_rate: f64) -> Self {
        if success_rate > 98.0 && failure_rate <= 1.0 {
            AnchorStatus::Green
        } else if success_rate >= 95.0 && failure_rate <= 5.0 {
            AnchorStatus::Yellow
        } else {
            AnchorStatus::Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorWithAssets {
    #[serde(flatten)]
    pub anchor: Anchor,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorDetailResponse {
    pub anchor: Anchor,
    pub assets: Vec<Asset>,
    pub metrics_history: Vec<AnchorMetricsHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CorridorRecord {
    pub id: String,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub destination_asset_code: String,
    pub destination_asset_issuer: String,
    pub reliability_score: f64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MetricRecord {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SnapshotRecord {
    pub id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub data: String,         // JSON serialized
    pub hash: Option<String>, // SHA-256 hash
    pub epoch: Option<i64>,   // Epoch identifier
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnchorRequest {
    pub name: String,
    pub stellar_account: String,
    pub home_domain: Option<String>,
}

// =========================
// Corridor domain (new)
// =========================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCorridorRequest {
    pub name: Option<String>,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub dest_asset_code: String,
    pub dest_asset_issuer: String,
}

// =========================
// Payment domain
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentRecord {
    pub id: String,
    pub transaction_hash: String,
    pub source_account: String,
    pub destination_account: String,
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub amount: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IngestionState {
    pub task_name: String,
    pub last_cursor: String,
    pub updated_at: DateTime<Utc>,
}

// =========================
// Fee Bump domain
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FeeBumpTransaction {
    pub transaction_hash: String,
    pub ledger_sequence: i64,
    pub fee_source: String,
    pub fee_charged: i64,
    pub max_fee: i64,
    pub inner_transaction_hash: String,
    pub inner_max_fee: i64,
    pub signatures_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBumpStats {
    pub total_fee_bumps: i64,
    pub avg_fee_charged: f64,
    pub max_fee_charged: i64,
    pub min_fee_charged: i64,
    pub unique_fee_sources: i64,
}

// =========================
// Liquidity Pool domain
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LiquidityPool {
    pub pool_id: String,
    pub pool_type: String,
    pub fee_bp: i32,
    pub total_trustlines: i32,
    pub total_shares: String,
    pub reserve_a_asset_code: String,
    pub reserve_a_asset_issuer: Option<String>,
    pub reserve_a_amount: f64,
    pub reserve_b_asset_code: String,
    pub reserve_b_asset_issuer: Option<String>,
    pub reserve_b_amount: f64,
    pub total_value_usd: f64,
    pub volume_24h_usd: f64,
    pub fees_earned_24h_usd: f64,
    pub apy: f64,
    pub impermanent_loss_pct: f64,
    pub trade_count_24h: i32,
    pub last_synced_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LiquidityPoolSnapshot {
    pub id: i64,
    pub pool_id: String,
    pub reserve_a_amount: f64,
    pub reserve_b_amount: f64,
    pub total_value_usd: f64,
    pub volume_usd: f64,
    pub fees_usd: f64,
    pub apy: f64,
    pub impermanent_loss_pct: f64,
    pub trade_count: i32,
    pub snapshot_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPoolStats {
    pub total_pools: i64,
    pub total_value_locked_usd: f64,
    pub total_volume_24h_usd: f64,
    pub total_fees_24h_usd: f64,
    pub avg_apy: f64,
    pub avg_impermanent_loss: f64,
}

// =========================
// Trustline domain
// =========================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrustlineStat {
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_trustlines: i64,
    pub authorized_trustlines: i64,
    pub unauthorized_trustlines: i64,
    pub total_supply: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrustlineSnapshot {
    pub id: i64,
    pub asset_code: String,
    pub asset_issuer: String,
    pub total_trustlines: i64,
    pub authorized_trustlines: i64,
    pub unauthorized_trustlines: i64,
    pub total_supply: f64,
    pub snapshot_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustlineMetrics {
    pub total_assets_tracked: i64,
    pub total_trustlines_across_network: i64,
    pub active_assets: i64,
}
