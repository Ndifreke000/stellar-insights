use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingAdvancedModel {
    pub client_id: String,
    pub requests_count: u32,
    pub window_start: i64,
    pub burst_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitingAdvancedRequest {
    pub client_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitingAdvancedResponse {
    pub success: bool,
    pub remaining_requests: u32,
    pub message: String,
}
