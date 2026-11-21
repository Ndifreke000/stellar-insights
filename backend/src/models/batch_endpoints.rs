use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::ErrorDetail;

/// Default maximum number of sub-requests permitted in a single batch call.
pub const DEFAULT_MAX_BATCH_SIZE: usize = 100;
/// Default maximum aggregate weighted cost permitted across a single batch call.
pub const DEFAULT_MAX_BATCH_COST: u32 = 100;
/// Default maximum JSON payload size per sub-request body (64 KiB).
pub const DEFAULT_MAX_SUBREQUEST_BODY_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of items / sub-requests allowed in a single batch request.
    pub max_batch_size: usize,
    /// Maximum aggregate cost allowed across all sub-requests in one batch.
    pub max_total_cost: u32,
    /// Maximum serialized byte size of an individual sub-request body.
    pub max_subrequest_body_bytes: usize,
    /// Reject nested/recursive batch sub-requests (must stay `true` in production).
    pub deny_nested_batch: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
            max_total_cost: DEFAULT_MAX_BATCH_COST,
            max_subrequest_body_bytes: DEFAULT_MAX_SUBREQUEST_BODY_BYTES,
            deny_nested_batch: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItemResult<T> {
    pub index: usize,
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse<T> {
    pub results: Vec<BatchItemResult<T>>,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}

/// A single HTTP sub-request dispatched inside a batch call (`POST /api/v1/batch`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSubRequest {
    /// HTTP method (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`).
    pub method: String,
    /// Relative API path (and optional query string), e.g. `"/api/v1/corridors?limit=10"`.
    pub path: String,
    /// Optional non-sensitive request headers (e.g. `Accept`).
    /// Security-sensitive headers (`Authorization`, `Cookie`, `X-Forwarded-For`, etc.)
    /// are always stripped and replaced with the verified outer caller identity.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional JSON body for write methods.
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    /// Optional comma-separated field selection list evaluated against `FieldAccessPolicy`.
    #[serde(default)]
    pub fields: Option<String>,
}

/// Top-level payload for `POST /api/v1/batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHttpRequest {
    #[serde(alias = "items")]
    pub requests: Vec<BatchSubRequest>,
}

/// Outcome of an individual sub-request dispatched inside a batch call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSubResponse {
    pub index: usize,
    pub status: u16,
    pub success: bool,
    pub cost: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

/// Envelope returned by `POST /api/v1/batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHttpResponse {
    pub results: Vec<BatchSubResponse>,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub total_cost: u32,
}
