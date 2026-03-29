// Corridor models moved from models.rs

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CorridorRecord {
    pub id: String,
    pub source_asset_code: String,
    pub source_asset_issuer: String,
    pub destination_asset_code: String,
    pub destination_asset_issuer: String,
    pub reliability_score: f64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorMetrics {
    pub success_rate: f64,
    // Add more metrics...
}

