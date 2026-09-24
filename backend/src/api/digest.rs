//! Email digest report endpoints (#2130).
//!
//! The weekly/monthly schedule is driven by [`DigestScheduler::start`]; these
//! endpoints expose the same report generation on demand, so an operator can
//! preview a digest or re-send one without waiting for the next tick.

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::cache::CacheManager;
use crate::email::scheduler::DigestScheduler;
use crate::email::service::EmailService;
use crate::error::ApiResult;
use crate::rpc::StellarRpcClient;

#[derive(Deserialize)]
pub struct SendDigestRequest {
    /// "Weekly" or "Monthly" — matches the scheduler's own period labels.
    pub period: String,
    /// Recipients for this send. Empty falls back to the configured list.
    #[serde(default)]
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
    match scheduler.send_digest(&req.period).await {
        Ok(()) => Ok(Json(SendDigestResponse {
            success: true,
            message: format!("{} digest dispatched", req.period),
        })),
        // A send failure is reported in the body rather than as a 5xx: the
        // scheduler already skips individual bad recipients, so a failure here
        // means the whole run could not start, and the caller still wants the
        // reason rather than an opaque error page.
        Err(e) => Ok(Json(SendDigestResponse {
            success: false,
            message: format!("Failed to send digest: {e}"),
        })),
    }
}

/// Build a [`DigestScheduler`] from the SMTP environment.
///
/// Mirrors the variables documented on [`DigestScheduler`]: `SMTP_HOST`,
/// `SMTP_USER`, `SMTP_PASS`, and a comma-separated `DIGEST_RECIPIENTS`.
#[must_use]
pub fn scheduler_from_env(
    cache: Arc<CacheManager>,
    rpc_client: Arc<StellarRpcClient>,
) -> Arc<DigestScheduler> {
    let email_service = Arc::new(EmailService::new(
        std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
        std::env::var("SMTP_USER").unwrap_or_default(),
        std::env::var("SMTP_PASS").unwrap_or_default(),
    ));

    let recipients = std::env::var("DIGEST_RECIPIENTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect();

    Arc::new(DigestScheduler::new(
        email_service,
        cache,
        rpc_client,
        recipients,
    ))
}

/// Digest routes, mounted under `/digest`.
#[must_use]
pub fn routes(scheduler: Arc<DigestScheduler>) -> Router {
    Router::new()
        .route("/send", post(send_digest_manual))
        .with_state(scheduler)
}
