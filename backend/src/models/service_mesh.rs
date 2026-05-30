use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshModel {
    pub service_id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMeshRequest {
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceMeshResponse {
    pub success: bool,
    pub service: Option<String>,
    pub message: String,
}
