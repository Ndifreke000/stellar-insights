use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

pub mod alerts;
pub mod api_gateway;
pub mod api_key;
pub mod api_versioning;
pub mod asset_verification;
pub mod batch_endpoints;
pub mod corridor;
pub mod corridor_alerts;
pub mod database_schema_separation;
pub mod distributed_tracing;
pub mod elasticsearch_integration;
pub mod etag_caching_support;
pub mod field_selection_parameter;
pub mod graphql_api;
pub mod jwt_token_refresh;
pub mod message_queue_system;
pub mod mobile_pagination_endpoints;
pub mod mobile_request_logging;
pub mod network_aware_rpc_client;
pub mod network_context_middleware;
pub mod network_status_endpoint;
pub mod push_notification_registration;
pub mod push_notification_service;
pub mod rate_limiting_advanced;
pub mod rate_limiting_by_client;
pub mod redis_caching_models;
pub mod response_compression;
pub mod sep10_for_mobile;
pub mod service_mesh;
pub mod websocket_real_time_updates;
pub mod websocket_streaming_models;

/// Number of decimal places used by Stellar asset amounts (stroops).
pub const STROOPS_PER_UNIT: i128 = 10_000_000;

/// Exact asset amount represented as an integer number of stroops (1e-7 units).
///
/// Stellar amounts have 7 decimal places; storing them as `i128` stroops avoids
/// the precision loss that occurs when summing `f64` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Amount(pub i128);

impl Amount {
    /// Zero amount.
    pub const ZERO: Self = Self(0);

    /// Construct from a raw stroop count.
    #[must_use]
    pub const fn from_stroops(stroops: i128) -> Self {
        Self(stroops)
    }

    /// Return the raw stroop count.
    #[must_use]
    pub const fn stroops(self) -> i128 {
        self.0
    }

    /// Parse a decimal string with up to 7 fractional digits into stroops.
    ///
    /// Returns `None` for malformed input or more than 7 fractional digits.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let (negative, digits) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s.strip_prefix('+').unwrap_or(s)),
        };
        let (int_part, frac_part) = match digits.split_once('.') {
            Some((i, f)) => (i, f),
            None => (digits, ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return None;
        }
        if frac_part.len() > 7 {
            return None;
        }
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let int_val: i128 = if int_part.is_empty() {
            0
        } else {
            int_part.parse().ok()?
        };
        let mut frac_val: i128 = if frac_part.is_empty() {
            0
        } else {
            frac_part.parse().ok()?
        };
        for _ in frac_part.len()..7 {
            frac_val *= 10;
        }
        let total = int_val.checked_mul(STROOPS_PER_UNIT)?.checked_add(frac_val)?;
        Some(Self(if negative { -total } else { total }))
    }

    /// Format as a decimal string with trailing zeros trimmed (e.g. `"1.5"`).
    #[must_use]
    pub fn to_string_trimmed(self) -> String {
        let negative = self.0 < 0;
        let abs = self.0.unsigned_abs();
        let int_part = abs / STROOPS_PER_UNIT as u128;
        let frac_part = abs % STROOPS_PER_UNIT as u128;
        let mut out = String::new();
        if negative {
            out.push('-');
        }
        out.push_str(&int_part.to_string());
        if frac_part != 0 {
            let frac = format!("{:07}", frac_part);
            let trimmed = frac.trim_end_matches('0');
            out.push('.');
            out.push_str(trimmed);
        }
        out
    }

    /// Checked addition; returns `None` on overflow.
    #[must_use]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_trimmed())
    }
}

impl std::str::FromStr for Amount {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
}

impl std::ops::Add for Amount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::iter::Sum for Amount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(utoipa::ToSchema)]
pub enum SortBy {
    #[serde(rename = "success_rate")]
    SuccessRate,
    /// Liquidity/volume in USD. `liquidity` is accepted as an alias (frontend naming).
    #[serde(rename = "volume", alias = "liquidity")]
    Volume,
    #[serde(rename = "health_score")]
    HealthScore,
}

impl Default for SortBy {
    fn default() -> Self {
        Self::SuccessRate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnchorStatus {
    Green,
    Yellow,
    Red,
}

impl AnchorStatus {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Red => "red",
        }
    }

    #[must_use]
    pub fn from_metrics(success_rate: f64, failure_rate: f64) -> Self {
        if success_rate > 98.0 && failure_rate <= 1.0 {
            Self::Green
        } else if success_rate >= 95.0 && failure_rate <= 5.0 {
            Self::Yellow
        } else {
            Self::Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "name": "Example Anchor",
    "stellar_account": "GAEXAMPLEACCOUNTXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "home_domain": "anchor.example.com"
}))]
pub struct CreateAnchorRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,

    #[validate(length(
        min = 56,
        max = 56,
        message = "Stellar account must be exactly 56 characters"
    ))]
    pub stellar_account: String,

    #[validate(length(max = 253, message = "Home domain must be at most 253 characters"))]
    pub home_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[schema(example = json!({
    "source_asset_code": "USDC",
    "source_asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    "dest_asset_code": "EURC",
    "dest_asset_issuer": "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2"
}))]
pub struct CreateCorridorRequest {
    #[validate(length(
        min = 1,
        max = 12,
        message = "Source asset code must be between 1 and 12 characters"
    ))]
    pub source_asset_code: String,

    #[validate(length(
        min = 56,
        max = 56,
        message = "Source asset issuer must be exactly 56 characters"
    ))]
    pub source_asset_issuer: String,

    #[validate(length(
        min = 1,
        max = 12,
        message = "Destination asset code must be between 1 and 12 characters"
    ))]
    pub dest_asset_code: String,

    #[validate(length(
        min = 56,
        max = 56,
        message = "Destination asset issuer must be exactly 56 characters"
    ))]
    pub dest_asset_issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorDetailResponse {
    pub anchor: Anchor,
    pub assets: Vec<Asset>,
    pub metrics_history: Vec<AnchorMetricsHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorWithAssets {
    #[serde(flatten)]
    pub anchor: Anchor,
    pub assets: Vec<Asset>,
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
    pub data: String,
    pub hash: Option<String>,
    pub epoch: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Database row for a payment fetched from the `payments` table.
/// For the analytics domain model, see [`crate::models::corridor::PaymentRecord`].
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentRow {
    pub id: String,
    pub transaction_hash: String,
    pub source_account: String,
    pub destination_account: String,
    pub asset_type: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    #[sqlx(default)]
    pub source_asset_code: String,
    #[sqlx(default)]
    pub source_asset_issuer: String,
    #[sqlx(default)]
    pub destination_asset_code: String,
    #[sqlx(default)]
    pub destination_asset_issuer: String,
    pub amount: f64,
    #[sqlx(default)]
    pub successful: bool,
    #[sqlx(default)]
    pub timestamp: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub submission_time: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub confirmation_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
