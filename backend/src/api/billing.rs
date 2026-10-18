//! Paid API key tier: plan, invoices and on-chain payment confirmation.
//!
//! `GET /billing/plan` is public. Everything else needs the SEP-10 session of
//! the wallet that owns the API key.

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::auth::Sep10User;
use crate::billing::{self, BillingConfig, BillingError, Invoice, Subscription};

#[derive(Clone)]
pub struct BillingState {
    pub pool: SqlitePool,
    pub config: Option<Arc<BillingConfig>>,
    pub http: reqwest::Client,
}

/// The paid plan on offer.
#[derive(Debug, Serialize, ToSchema)]
pub struct PlanResponse {
    pub plan: String,
    /// Price per period, in `asset_code`.
    pub price: String,
    pub asset_code: String,
    pub asset_issuer: String,
    /// Account payments are sent to.
    pub destination: String,
    pub period_days: i64,
    pub limit_per_minute: i64,
    /// Requests per minute without a paid plan, with and without an API key.
    pub free_limit_per_minute: i64,
    pub anonymous_limit_per_minute: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateInvoiceRequest {
    /// API key to upgrade. It must belong to the signed-in wallet.
    pub api_key_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfirmInvoiceRequest {
    /// Hash of the Stellar transaction that paid the invoice.
    pub transaction_hash: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfirmInvoiceResponse {
    pub invoice: Invoice,
    pub subscription: Subscription,
}

fn error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({ "error": code, "message": message.into() })),
    )
        .into_response()
}

fn billing_error(e: BillingError) -> Response {
    match e {
        BillingError::NotFound(m) => error(StatusCode::NOT_FOUND, "NOT_FOUND", m),
        BillingError::Invalid(m) => error(StatusCode::UNPROCESSABLE_ENTITY, "PAYMENT_NOT_VALID", m),
        BillingError::Conflict(m) => error(StatusCode::CONFLICT, "CONFLICT", m),
        BillingError::Upstream(m) => error(StatusCode::BAD_GATEWAY, "HORIZON_UNAVAILABLE", m),
        BillingError::Internal(e) => {
            tracing::error!("billing error: {e:#}");
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
                "Billing request failed",
            )
        }
    }
}

fn not_configured() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "BILLING_NOT_CONFIGURED",
        "Paid plans are not enabled on this server",
    )
}

fn wallet(user: Option<Extension<Sep10User>>) -> Result<String, Response> {
    user.map(|Extension(u)| u.account).ok_or_else(|| {
        error(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "A SEP-10 wallet session is required",
        )
    })
}

/// Current paid plan and how to pay for it.
#[utoipa::path(
    get,
    path = "/api/billing/plan",
    responses(
        (status = 200, description = "Plan, price and payment destination", body = PlanResponse),
        (status = 503, description = "Paid plans are not enabled")
    ),
    tag = "Billing"
)]
pub async fn get_plan(State(state): State<BillingState>) -> Response {
    let Some(config) = state.config else {
        return not_configured();
    };
    Json(PlanResponse {
        plan: "pro".to_string(),
        price: config.price_usdc.normalize().to_string(),
        asset_code: config.asset_code.clone(),
        asset_issuer: config.asset_issuer.clone(),
        destination: config.treasury.clone(),
        period_days: config.period_days,
        limit_per_minute: config.limit_per_minute,
        free_limit_per_minute: 200,
        anonymous_limit_per_minute: 60,
    })
    .into_response()
}

/// Create an invoice to upgrade one of the wallet's API keys.
#[utoipa::path(
    post,
    path = "/api/billing/invoices",
    request_body = CreateInvoiceRequest,
    responses(
        (status = 201, description = "Invoice with payment instructions", body = Invoice),
        (status = 401, description = "No SEP-10 session"),
        (status = 404, description = "Key not found for this wallet"),
        (status = 503, description = "Paid plans are not enabled")
    ),
    tag = "Billing"
)]
pub async fn create_invoice(
    State(state): State<BillingState>,
    user: Option<Extension<Sep10User>>,
    Json(request): Json<CreateInvoiceRequest>,
) -> Response {
    let Some(config) = state.config else {
        return not_configured();
    };
    let wallet = match wallet(user) {
        Ok(w) => w,
        Err(r) => return r,
    };
    match billing::create_invoice(&state.pool, &config, &request.api_key_id, &wallet).await {
        Ok(invoice) => (StatusCode::CREATED, Json(invoice)).into_response(),
        Err(e) => billing_error(e),
    }
}

/// Read one of the wallet's invoices.
#[utoipa::path(
    get,
    path = "/api/billing/invoices/{id}",
    params(("id" = String, Path, description = "Invoice id")),
    responses(
        (status = 200, description = "Invoice", body = Invoice),
        (status = 404, description = "Invoice not found for this wallet")
    ),
    tag = "Billing"
)]
pub async fn get_invoice(
    State(state): State<BillingState>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
) -> Response {
    let wallet = match wallet(user) {
        Ok(w) => w,
        Err(r) => return r,
    };
    match billing::get_invoice(&state.pool, &id, &wallet).await {
        Ok(invoice) => Json(invoice).into_response(),
        Err(e) => billing_error(e),
    }
}

/// Confirm payment of an invoice by transaction hash.
#[utoipa::path(
    post,
    path = "/api/billing/invoices/{id}/confirm",
    params(("id" = String, Path, description = "Invoice id")),
    request_body = ConfirmInvoiceRequest,
    responses(
        (status = 200, description = "Invoice paid; subscription extended", body = ConfirmInvoiceResponse),
        (status = 409, description = "Invoice already paid, expired, or transaction already used"),
        (status = 422, description = "The transaction does not pay this invoice"),
        (status = 502, description = "Horizon could not be reached")
    ),
    tag = "Billing"
)]
pub async fn confirm_invoice(
    State(state): State<BillingState>,
    user: Option<Extension<Sep10User>>,
    Path(id): Path<String>,
    Json(request): Json<ConfirmInvoiceRequest>,
) -> Response {
    let Some(config) = state.config else {
        return not_configured();
    };
    let wallet = match wallet(user) {
        Ok(w) => w,
        Err(r) => return r,
    };
    match billing::confirm_invoice(
        &state.pool,
        &config,
        &state.http,
        &id,
        &wallet,
        &request.transaction_hash,
    )
    .await
    {
        Ok((invoice, subscription)) => Json(ConfirmInvoiceResponse {
            invoice,
            subscription,
        })
        .into_response(),
        Err(e) => billing_error(e),
    }
}

/// A key's paid subscription.
#[utoipa::path(
    get,
    path = "/api/billing/subscriptions/{api_key_id}",
    params(("api_key_id" = String, Path, description = "API key id")),
    responses(
        (status = 200, description = "Subscription", body = Subscription),
        (status = 404, description = "No subscription for this key")
    ),
    tag = "Billing"
)]
pub async fn get_subscription(
    State(state): State<BillingState>,
    user: Option<Extension<Sep10User>>,
    Path(api_key_id): Path<String>,
) -> Response {
    let wallet = match wallet(user) {
        Ok(w) => w,
        Err(r) => return r,
    };
    match billing::get_subscription(&state.pool, &api_key_id, &wallet).await {
        Ok(Some(subscription)) => Json(subscription).into_response(),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "No subscription for this key",
        ),
        Err(e) => billing_error(e),
    }
}

/// Billing routes. The plan is public; the rest sit behind SEP-10.
pub fn routes(
    state: BillingState,
    sep10_service: Arc<crate::auth::sep10_simple::Sep10Service>,
) -> Router {
    let protected = Router::new()
        .route("/invoices", post(create_invoice))
        .route("/invoices/{id}", get(get_invoice))
        .route("/invoices/{id}/confirm", post(confirm_invoice))
        .route("/subscriptions/{api_key_id}", get(get_subscription))
        .layer(axum::middleware::from_fn_with_state(
            sep10_service,
            crate::auth::sep10_auth_middleware,
        ));

    Router::new()
        .route("/plan", get(get_plan))
        .merge(protected)
        .with_state(state)
}
