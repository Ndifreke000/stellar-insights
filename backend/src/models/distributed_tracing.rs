use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTracingModel {
    pub trace_id: String,
    pub span_count: u32,
    pub start_time: i64,
    pub end_time: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistributedTracingRequest {
    pub operation_name: String,
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistributedTracingResponse {
    pub success: bool,
    pub trace_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanModel {
    pub span_id: String,
    pub trace_id: String,
    pub operation_name: String,
    pub service_name: String,
    pub tags: HashMap<String, String>,
}
