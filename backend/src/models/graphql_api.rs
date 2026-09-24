use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<serde_json::Value>,
    #[serde(default)]
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<GraphQLErrorDetail>>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLErrorDetail {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLHealthStatus {
    pub enabled: bool,
    pub endpoint: String,
    pub version: String,
}
