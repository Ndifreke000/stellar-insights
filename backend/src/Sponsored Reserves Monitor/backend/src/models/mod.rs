use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Sponsorship {
    pub id: String,
    pub sponsor: String,
    pub sponsored_account: String,
    pub sponsored_reserves: i64,
    pub total_amount: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSponsorshipRequest {
    pub sponsor: String,
    pub sponsored_account: String,
    pub sponsored_reserves: i64,
    pub total_amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SponsorshipHistory {
    pub id: String,
    pub sponsorship_id: String,
    pub change_type: String,
    pub previous_amount: Option<String>,
    pub new_amount: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorLeaderboard {
    pub sponsor: String,
    pub total_sponsored_amount: String,
    pub sponsored_accounts_count: i64,
    pub rank: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorshipAnalytics {
    pub total_sponsorships: i64,
    pub total_amount_sponsored: String,
    pub unique_sponsors: i64,
    pub unique_sponsored_accounts: i64,
    pub average_sponsorship: String,
    pub largest_sponsorship: String,
    pub smallest_sponsorship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SponsorshipChangeAlert {
    pub sponsorship_id: String,
    pub sponsor: String,
    pub sponsored_account: String,
    pub change_type: String,
    pub previous_value: Option<String>,
    pub new_value: String,
    pub timestamp: String,
}
