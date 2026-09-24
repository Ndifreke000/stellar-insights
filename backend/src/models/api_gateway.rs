use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGatewayModel {
    pub route_id: String,
    pub path: String,
    pub method: String,
    pub target_service: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGatewayRequest {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGatewayResponse {
    pub success: bool,
    pub route: Option<String>,
    pub message: String,
}
