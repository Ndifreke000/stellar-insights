use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::email::scheduler::DigestScheduler;
use crate::error::ApiResult;

#[derive(Deserialize)]
pub struct SendDigestRequest {
    pub period: String,
    pub recipients: Vec<String>,
}

#[derive(Serialize)]
pub struct SendDigestResponse {
    pub success: bool,
    pub message: String,
}

pub async fn send_digest_manual(
    State(scheduler): State<Arc<DigestScheduler>>,
    Json(req): Json<SendDigestRequest>,
) -> ApiResult<Json<SendDigestResponse>> {
    // Trigger manual digest send
    match scheduler.send_digest(&req.period).await {
        Ok(_) => Ok(Json(SendDigestResponse {
            success: true,
            message: format!("Digest sent to {} recipients", req.recipients.len()),
        })),
        Err(e) => Ok(Json(SendDigestResponse {
            success: false,
            message: format!("Failed to send digest: {}", e),
        })),
    }
}
