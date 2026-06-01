use async_graphql::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Anchor entity with metrics
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "Anchor")]
pub struct AnchorType {
    /// Unique identifier
    pub id: String,
    /// Anchor name
    pub name: String,
    /// Stellar account address
    pub stellar_account: String,
    /// Home domain
    pub home_domain: Option<String>,
    /// Total number of transactions
    pub total_transactions: i64,
    /// Number of successful transactions
    pub successful_transactions: i64,
    /// Number of failed transactions
    pub failed_transactions: i64,
    /// Total volume in USD
    pub total_volume_usd: f64,
    /// Average settlement time in milliseconds
    pub avg_settlement_time_ms: i64,
    /// Reliability score (0-100)
    pub reliability_score: f64,
    /// Status (green, yellow, red)
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Asset entity
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "Asset")]
pub struct AssetType {
    /// Unique identifier
    pub id: String,
    /// Associated anchor ID
    pub anchor_id: String,
    /// Asset code (e.g., USDC, EUR)
    pub asset_code: String,
    /// Asset issuer address
    pub asset_issuer: String,
    /// Total supply
    pub total_supply: Option<f64>,
    /// Number of holders
    pub num_holders: i64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Corridor entity representing a payment path
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "Corridor")]
pub struct CorridorType {
    /// Unique identifier
    pub id: String,
    /// Source asset code
    pub source_asset_code: String,
    /// Source asset issuer
    pub source_asset_issuer: String,
    /// Destination asset code
    pub destination_asset_code: String,
    /// Destination asset issuer
    pub destination_asset_issuer: String,
    /// Reliability score (0-100)
    pub reliability_score: f64,
    /// Status (active, inactive)
    pub status: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Metric data point
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "Metric")]
pub struct MetricType {
    /// Unique identifier
    pub id: String,
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Associated entity ID
    pub entity_id: Option<String>,
    /// Entity type (anchor, corridor, etc.)
    pub entity_type: Option<String>,
    /// Timestamp of the metric
    pub timestamp: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Snapshot of entity state
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "Snapshot")]
pub struct SnapshotType {
    /// Unique identifier
    pub id: String,
    /// Associated entity ID
    pub entity_id: String,
    /// Entity type
    pub entity_type: String,
    /// Snapshot data (JSON)
    pub data: String,
    /// Hash of the snapshot
    pub hash: Option<String>,
    /// Epoch number
    pub epoch: Option<i64>,
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Liquidity pool information
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "LiquidityPool")]
pub struct LiquidityPoolType {
    /// Pool ID
    pub pool_id: String,
    /// Asset A code
    pub asset_a_code: String,
    /// Asset A issuer
    pub asset_a_issuer: String,
    /// Asset B code
    pub asset_b_code: String,
    /// Asset B issuer
    pub asset_b_issuer: String,
    /// Total liquidity in USD
    pub total_liquidity_usd: f64,
    /// Total shares
    pub total_shares: f64,
    /// Fee basis points
    pub fee_bp: i32,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Trustline statistics
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[graphql(name = "TrustlineStat")]
pub struct TrustlineStatType {
    /// Asset code
    pub asset_code: String,
    /// Asset issuer
    pub asset_issuer: String,
    /// Total trustlines
    pub total_trustlines: i64,
    /// Authorized trustlines
    pub authorized_trustlines: i64,
    /// Unauthorized trustlines
    pub unauthorized_trustlines: i64,
    /// Total supply
    pub total_supply: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Pagination input
#[derive(Debug, Clone, InputObject)]
pub struct PaginationInput {
    /// Number of items to return (default: 10, max: 100)
    pub limit: Option<i32>,
    /// Number of items to skip
    pub offset: Option<i32>,
}

/// Filter for anchors
#[derive(Debug, Clone, InputObject)]
pub struct AnchorFilter {
    /// Filter by status
    pub status: Option<String>,
    /// Minimum reliability score
    pub min_reliability_score: Option<f64>,
    /// Search by name or account
    pub search: Option<String>,
}

/// Filter for corridors
#[derive(Debug, Clone, InputObject)]
pub struct CorridorFilter {
    /// Filter by source asset code
    pub source_asset_code: Option<String>,
    /// Filter by destination asset code
    pub destination_asset_code: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Minimum reliability score
    pub min_reliability_score: Option<f64>,
}

/// Time range filter
#[derive(Debug, Clone, InputObject)]
pub struct TimeRangeInput {
    /// Start time
    pub start: DateTime<Utc>,
    /// End time
    pub end: DateTime<Utc>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "AnchorsConnection")]
pub struct AnchorsConnection {
    /// List of anchors
    pub nodes: Vec<AnchorType>,
    /// Total count
    pub total_count: i32,
    /// Whether there are more items
    pub has_next_page: bool,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "CorridorsConnection")]
pub struct CorridorsConnection {
    /// List of corridors
    pub nodes: Vec<CorridorType>,
    /// Total count
    pub total_count: i32,
    /// Whether there are more items
    pub has_next_page: bool,
}
