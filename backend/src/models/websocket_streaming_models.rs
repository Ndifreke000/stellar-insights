use serde::{Deserialize, Serialize};

/// WebSocket subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub channels: Vec<String>,
}

/// WebSocket subscription response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionResponse {
    pub subscription_id: String,
    pub channels: Vec<String>,
    pub status: String,
}

/// Real-time data update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeUpdate {
    pub channel: String,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

/// Stream statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub active_subscribers: usize,
    pub messages_published: u64,
    pub uptime_seconds: u64,
}
