use async_graphql::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Entity Types ──────────────────────────────────────────────────────────────

/// Anchor entity with metrics — payment service providers on the Stellar network.
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
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

/// Asset entity issued by an anchor
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
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

/// Snapshot of entity state (on-chain verification)
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
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

/// Payment record
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "Payment")]
pub struct PaymentType {
    /// Unique identifier
    pub id: String,
    /// Transaction hash
    pub transaction_hash: String,
    /// Source account
    pub source_account: String,
    /// Destination account
    pub destination_account: String,
    /// Asset type
    pub asset_type: String,
    /// Asset code
    pub asset_code: Option<String>,
    /// Asset issuer
    pub asset_issuer: Option<String>,
    /// Source asset code
    pub source_asset_code: String,
    /// Source asset issuer
    pub source_asset_issuer: String,
    /// Destination asset code
    pub destination_asset_code: String,
    /// Destination asset issuer
    pub destination_asset_issuer: String,
    /// Amount transferred
    pub amount: f64,
    /// Whether the payment was successful
    pub successful: bool,
    /// Timestamp of the payment
    pub timestamp: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Liquidity pool information
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "LiquidityPool")]
pub struct LiquidityPoolType {
    /// Pool ID
    pub pool_id: String,
    /// Pool type
    pub pool_type: String,
    /// Fee basis points
    pub fee_bp: i32,
    /// Total trustlines
    pub total_trustlines: i32,
    /// Total shares
    pub total_shares: String,
    /// Asset A code
    pub reserve_a_asset_code: String,
    /// Asset A issuer
    pub reserve_a_asset_issuer: Option<String>,
    /// Reserve A amount
    pub reserve_a_amount: f64,
    /// Asset B code
    pub reserve_b_asset_code: String,
    /// Asset B issuer
    pub reserve_b_asset_issuer: Option<String>,
    /// Reserve B amount
    pub reserve_b_amount: f64,
    /// Total value in USD
    pub total_value_usd: f64,
    /// 24h volume in USD
    pub volume_24h_usd: f64,
    /// 24h fees earned in USD
    pub fees_earned_24h_usd: f64,
    /// Annual percentage yield
    pub apy: f64,
    /// Impermanent loss percentage
    pub impermanent_loss_pct: f64,
    /// 24h trade count
    pub trade_count_24h: i32,
    /// Last synced timestamp
    pub last_synced_at: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Liquidity pool snapshot
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "LiquidityPoolSnapshot")]
pub struct LiquidityPoolSnapshotType {
    /// Snapshot ID
    pub id: i64,
    /// Pool ID
    pub pool_id: String,
    /// Reserve A amount
    pub reserve_a_amount: f64,
    /// Reserve B amount
    pub reserve_b_amount: f64,
    /// Total value in USD
    pub total_value_usd: f64,
    /// Volume in USD
    pub volume_usd: f64,
    /// Fees in USD
    pub fees_usd: f64,
    /// APY
    pub apy: f64,
    /// Impermanent loss percentage
    pub impermanent_loss_pct: f64,
    /// Trade count
    pub trade_count: i32,
    /// Snapshot timestamp
    pub snapshot_at: DateTime<Utc>,
}

/// Liquidity pool aggregated statistics
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "LiquidityPoolStats")]
pub struct LiquidityPoolStatsType {
    /// Total number of pools
    pub total_pools: i64,
    /// Total liquidity in USD
    pub total_liquidity_usd: f64,
    /// Average pool size in USD
    pub avg_pool_size_usd: f64,
    /// Total value locked in USD
    pub total_value_locked_usd: f64,
    /// Total 24h volume in USD
    pub total_volume_24h_usd: f64,
    /// Total 24h fees in USD
    pub total_fees_24h_usd: f64,
    /// Average APY
    pub avg_apy: f64,
    /// Average impermanent loss
    pub avg_impermanent_loss: f64,
}

/// Trustline statistics
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
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

/// Trustline snapshot
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "TrustlineSnapshot")]
pub struct TrustlineSnapshotType {
    /// Snapshot ID
    pub id: i64,
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
    /// Snapshot timestamp
    pub snapshot_at: DateTime<Utc>,
}

/// Trustline aggregate metrics
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "TrustlineMetrics")]
pub struct TrustlineMetricsType {
    /// Total assets tracked
    pub total_assets_tracked: i64,
    /// Total trustlines across network
    pub total_trustlines_across_network: i64,
    /// Active assets count
    pub active_assets: i64,
}

/// Anchor metrics history entry
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject, sqlx::FromRow)]
#[graphql(name = "AnchorMetricsHistory")]
pub struct AnchorMetricsHistoryType {
    /// Unique identifier
    pub id: String,
    /// Anchor ID
    pub anchor_id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Success rate (0-100)
    pub success_rate: f64,
    /// Failure rate (0-100)
    pub failure_rate: f64,
    /// Reliability score (0-100)
    pub reliability_score: f64,
    /// Total transactions
    pub total_transactions: i64,
    /// Successful transactions
    pub successful_transactions: i64,
    /// Failed transactions
    pub failed_transactions: i64,
    /// Average settlement time in ms
    pub avg_settlement_time_ms: Option<i32>,
    /// Volume in USD
    pub volume_usd: Option<f64>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// System health status
#[derive(Debug, Clone, SimpleObject)]
pub struct HealthType {
    /// Health status
    pub status: String,
    /// API version
    pub version: String,
    /// Database status
    pub database: String,
    /// Cache status
    pub cache: String,
    /// Active WebSocket connections
    pub active_connections: i64,
}

// ── Input Types ───────────────────────────────────────────────────────────────

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
    /// Filter by status (green, yellow, red)
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

/// Filter for payments
#[derive(Debug, Clone, InputObject)]
pub struct PaymentFilter {
    /// Filter by source account
    pub source_account: Option<String>,
    /// Filter by destination account
    pub destination_account: Option<String>,
    /// Filter by asset code
    pub asset_code: Option<String>,
    /// Filter by success status
    pub successful: Option<bool>,
    /// Minimum amount
    pub min_amount: Option<f64>,
    /// Maximum amount
    pub max_amount: Option<f64>,
}

/// Filter for liquidity pools
#[derive(Debug, Clone, InputObject)]
pub struct LiquidityPoolFilter {
    /// Filter by asset A code
    pub asset_a_code: Option<String>,
    /// Filter by asset B code
    pub asset_b_code: Option<String>,
    /// Minimum APY
    pub min_apy: Option<f64>,
    /// Minimum total value in USD
    pub min_total_value_usd: Option<f64>,
}

/// Filter for trustlines
#[derive(Debug, Clone, InputObject)]
pub struct TrustlineFilter {
    /// Filter by asset code
    pub asset_code: Option<String>,
    /// Minimum total trustlines
    pub min_total_trustlines: Option<i64>,
}

/// Time range filter
#[derive(Debug, Clone, InputObject)]
pub struct TimeRangeInput {
    /// Start time
    pub start: DateTime<Utc>,
    /// End time
    pub end: DateTime<Utc>,
}

/// Create anchor mutation input
#[derive(Debug, Clone, InputObject)]
pub struct CreateAnchorInput {
    /// Anchor name (1-100 chars)
    pub name: String,
    /// Stellar account (56 chars, G-address)
    pub stellar_account: String,
    /// Home domain (optional, max 253 chars)
    pub home_domain: Option<String>,
}

/// Create corridor mutation input
#[derive(Debug, Clone, InputObject)]
pub struct CreateCorridorInput {
    /// Source asset code (1-12 chars)
    pub source_asset_code: String,
    /// Source asset issuer (56 chars, G-address)
    pub source_asset_issuer: String,
    /// Destination asset code (1-12 chars)
    pub destination_asset_code: String,
    /// Destination asset issuer (56 chars, G-address)
    pub destination_asset_issuer: String,
}

/// Update anchor metrics mutation input
#[derive(Debug, Clone, InputObject)]
pub struct UpdateAnchorMetricsInput {
    /// Anchor ID
    pub anchor_id: String,
    /// Total transactions
    pub total_transactions: i64,
    /// Successful transactions
    pub successful_transactions: i64,
    /// Failed transactions
    pub failed_transactions: i64,
    /// Average settlement time in ms
    pub avg_settlement_time_ms: Option<i32>,
    /// Volume in USD
    pub volume_usd: Option<f64>,
}

// ── Connection / Paginated Types ──────────────────────────────────────────────

/// Paginated anchors connection
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

/// Paginated corridors connection
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

/// Paginated payments connection
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "PaymentsConnection")]
pub struct PaymentsConnection {
    /// List of payments
    pub nodes: Vec<PaymentType>,
    /// Total count
    pub total_count: i32,
    /// Whether there are more items
    pub has_next_page: bool,
}

/// Paginated snapshots connection
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "SnapshotsConnection")]
pub struct SnapshotsConnection {
    /// List of snapshots
    pub nodes: Vec<SnapshotType>,
    /// Total count
    pub total_count: i32,
    /// Whether there are more items
    pub has_next_page: bool,
}

/// Paginated liquidity pools connection
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "LiquidityPoolsConnection")]
pub struct LiquidityPoolsConnection {
    /// List of liquidity pools
    pub nodes: Vec<LiquidityPoolType>,
    /// Total count
    pub total_count: i32,
    /// Whether there are more items
    pub has_next_page: bool,
}

/// Paginated trustline stats connection
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "TrustlineStatsConnection")]
pub struct TrustlineStatsConnection {
    /// List of trustline stats
    pub nodes: Vec<TrustlineStatType>,
    /// Total count
    pub total_count: i32,
    /// Whether there are more items
    pub has_next_page: bool,
}

/// Search results combining multiple entity types
#[derive(Debug, Clone, SimpleObject)]
pub struct SearchResults {
    /// Matching anchors
    pub anchors: Vec<AnchorType>,
    /// Matching corridors
    pub corridors: Vec<CorridorType>,
    /// Matching payments
    pub payments: Vec<PaymentType>,
}

// ── Mutation Payloads ─────────────────────────────────────────────────────────

/// Result of creating an anchor
#[derive(Debug, Clone, SimpleObject)]
pub struct CreateAnchorPayload {
    /// The created anchor
    pub anchor: AnchorType,
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
}

/// Result of creating a corridor
#[derive(Debug, Clone, SimpleObject)]
pub struct CreateCorridorPayload {
    /// The created corridor
    pub corridor: CorridorType,
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
}

/// Result of updating anchor metrics
#[derive(Debug, Clone, SimpleObject)]
pub struct UpdateAnchorMetricsPayload {
    /// The updated anchor
    pub anchor: AnchorType,
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
}

// ── Subscription Types ────────────────────────────────────────────────────────

/// Real-time corridor update event
#[derive(Debug, Clone, SimpleObject)]
pub struct CorridorUpdateEvent {
    /// Corridor key
    pub corridor_key: String,
    /// Source asset code
    pub source_asset_code: String,
    /// Source asset issuer
    pub source_asset_issuer: String,
    /// Destination asset code
    pub destination_asset_code: String,
    /// Destination asset issuer
    pub destination_asset_issuer: String,
    /// Updated success rate
    pub success_rate: Option<f64>,
    /// Updated health score
    pub health_score: Option<f64>,
    /// Timestamp of the update
    pub last_updated: Option<String>,
}

/// Real-time anchor update event
#[derive(Debug, Clone, SimpleObject)]
pub struct AnchorUpdateEvent {
    /// Anchor ID
    pub anchor_id: String,
    /// Anchor name
    pub name: String,
    /// Updated reliability score
    pub reliability_score: f64,
    /// Updated status
    pub status: String,
}

/// Real-time snapshot update event
#[derive(Debug, Clone, SimpleObject)]
pub struct SnapshotUpdateEvent {
    /// Snapshot ID
    pub snapshot_id: String,
    /// Epoch number
    pub epoch: i64,
    /// Timestamp
    pub timestamp: String,
    /// Hash
    pub hash: String,
}

/// Real-time health alert event
#[derive(Debug, Clone, SimpleObject)]
pub struct HealthAlertEvent {
    /// Corridor ID
    pub corridor_id: String,
    /// Severity level
    pub severity: String,
    /// Alert message
    pub message: String,
    /// Timestamp
    pub timestamp: String,
}

/// Real-time new payment event
#[derive(Debug, Clone, SimpleObject)]
pub struct NewPaymentEvent {
    /// Corridor ID
    pub corridor_id: String,
    /// Payment amount
    pub amount: f64,
    /// Whether the payment was successful
    pub successful: bool,
    /// Timestamp
    pub timestamp: String,
}
