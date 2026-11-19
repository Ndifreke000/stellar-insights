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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::billing::MAINNET_USDC_ISSUER;
    use crate::database::Database;
    use crate::models::api_key::CreateApiKeyRequest;
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const WALLET: &str = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
    const OTHER_WALLET: &str = "GCJROW3RGUJNOUJZTFSKLWS4YVEROS5A7YBH5JABGW4HK4QN3A37HIKX";
    const TREASURY: &str = "GCJROW3RGUJNOUJZTFSKLWS4YVEROS5A7YBH5JABGW4HK4QN3A37HIKX";

    fn config(horizon_url: String) -> Arc<BillingConfig> {
        Arc::new(BillingConfig {
            treasury: TREASURY.to_string(),
            asset_code: "USDC".to_string(),
            asset_issuer: MAINNET_USDC_ISSUER.to_string(),
            price_usdc: Decimal::from_str("50.00").unwrap(),
            period_days: 30,
            limit_per_minute: 1000,
            horizon_url,
        })
    }

    async fn state(config: Option<Arc<BillingConfig>>) -> BillingState {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        BillingState {
            pool,
            config,
            http: reqwest::Client::new(),
        }
    }

    async fn new_key(pool: &SqlitePool) -> String {
        Database::new(pool.clone())
            .create_api_key(
                WALLET,
                CreateApiKeyRequest {
                    name: "offramp".to_string(),
                    scopes: None,
                    rate_limit_per_minute: None,
                    expires_at: None,
                },
            )
            .await
            .unwrap()
            .key
            .id
    }

    fn user(account: &str) -> Option<Extension<Sep10User>> {
        Some(Extension(Sep10User {
            account: account.to_string(),
            client_domain: None,
        }))
    }

    async fn body(response: Response) -> (StatusCode, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    async fn horizon_paying(hash: &str, memo: &str, amount: &str) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/transactions/{hash}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hash": hash,
                "successful": true,
                "memo_type": "text",
                "memo": memo,
                "created_at": chrono::Utc::now().to_rfc3339(),
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/transactions/{hash}/operations")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_embedded": { "records": [{
                    "type": "payment",
                    "to": TREASURY,
                    "asset_type": "credit_alphanum4",
                    "asset_code": "USDC",
                    "asset_issuer": MAINNET_USDC_ISSUER,
                    "amount": amount,
                }]}
            })))
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn plan_describes_the_configured_tier() {
        let (status, plan) =
            body(get_plan(State(state(Some(config(String::new()))).await)).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(plan["plan"], "pro");
        assert_eq!(plan["price"], "50");
        assert_eq!(plan["destination"], TREASURY);
        assert_eq!(plan["limit_per_minute"], 1000);
        assert_eq!(plan["free_limit_per_minute"], 200);
        assert_eq!(plan["anonymous_limit_per_minute"], 60);
    }

    #[tokio::test]
    async fn every_paid_endpoint_reports_when_billing_is_off() {
        let s = state(None).await;
        let (status, err) = body(get_plan(State(s.clone())).await).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err["error"], "BILLING_NOT_CONFIGURED");

        let request = CreateInvoiceRequest {
            api_key_id: "k".to_string(),
        };
        let (status, _) =
            body(create_invoice(State(s.clone()), user(WALLET), Json(request)).await).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

        let confirm = ConfirmInvoiceRequest {
            transaction_hash: "a".repeat(64),
        };
        let (status, _) = body(
            confirm_invoice(State(s), user(WALLET), Path("i".to_string()), Json(confirm)).await,
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn wallet_endpoints_require_a_session() {
        let s = state(Some(config(String::new()))).await;
        let request = CreateInvoiceRequest {
            api_key_id: "k".to_string(),
        };
        let (status, err) = body(create_invoice(State(s.clone()), None, Json(request)).await).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(err["error"], "UNAUTHORIZED");

        let (status, _) =
            body(get_invoice(State(s.clone()), None, Path("i".to_string())).await).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let confirm = ConfirmInvoiceRequest {
            transaction_hash: "a".repeat(64),
        };
        let (status, _) = body(
            confirm_invoice(State(s.clone()), None, Path("i".to_string()), Json(confirm)).await,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _) = body(get_subscription(State(s), None, Path("k".to_string())).await).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invoices_are_scoped_to_the_owning_wallet() {
        let s = state(Some(config(String::new()))).await;
        let key_id = new_key(&s.pool).await;

        // Another wallet cannot invoice this key.
        let request = CreateInvoiceRequest {
            api_key_id: key_id.clone(),
        };
        let (status, err) =
            body(create_invoice(State(s.clone()), user(OTHER_WALLET), Json(request)).await).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(err["error"], "NOT_FOUND");

        let request = CreateInvoiceRequest {
            api_key_id: key_id.clone(),
        };
        let (status, invoice) =
            body(create_invoice(State(s.clone()), user(WALLET), Json(request)).await).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(invoice["api_key_id"], key_id.as_str());
        assert_eq!(invoice["status"], "pending");
        let id = invoice["id"].as_str().unwrap().to_string();

        let (status, read) =
            body(get_invoice(State(s.clone()), user(WALLET), Path(id.clone())).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read["id"], id.as_str());

        let (status, _) = body(get_invoice(State(s), user(OTHER_WALLET), Path(id)).await).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn confirming_a_payment_activates_the_subscription() {
        let placeholder = state(Some(config(String::new()))).await;
        let key_id = new_key(&placeholder.pool).await;

        let (status, _) = body(
            get_subscription(
                State(placeholder.clone()),
                user(WALLET),
                Path(key_id.clone()),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let request = CreateInvoiceRequest {
            api_key_id: key_id.clone(),
        };
        let (_, invoice) =
            body(create_invoice(State(placeholder.clone()), user(WALLET), Json(request)).await)
                .await;
        let id = invoice["id"].as_str().unwrap().to_string();
        let memo = invoice["memo"].as_str().unwrap().to_string();

        let hash = "c".repeat(64);
        let horizon = horizon_paying(&hash, &memo, "50.0000000").await;
        let s = BillingState {
            config: Some(config(horizon.uri())),
            ..placeholder
        };

        // A transaction that underpays is rejected without changing anything.
        let short = "d".repeat(64);
        let underpaid = horizon_paying(&short, &memo, "1").await;
        let cheap = BillingState {
            config: Some(config(underpaid.uri())),
            ..s.clone()
        };
        let confirm = ConfirmInvoiceRequest {
            transaction_hash: short,
        };
        let (status, err) = body(
            confirm_invoice(State(cheap), user(WALLET), Path(id.clone()), Json(confirm)).await,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err["error"], "PAYMENT_NOT_VALID");

        let confirm = ConfirmInvoiceRequest {
            transaction_hash: hash.clone(),
        };
        let (status, paid) = body(
            confirm_invoice(
                State(s.clone()),
                user(WALLET),
                Path(id.clone()),
                Json(confirm),
            )
            .await,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(paid["invoice"]["status"], "paid");
        assert_eq!(paid["subscription"]["limit_per_minute"], 1000);

        // Paying twice is a conflict.
        let confirm = ConfirmInvoiceRequest {
            transaction_hash: hash,
        };
        let (status, err) =
            body(confirm_invoice(State(s.clone()), user(WALLET), Path(id), Json(confirm)).await)
                .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err["error"], "CONFLICT");

        let (status, subscription) =
            body(get_subscription(State(s), user(WALLET), Path(key_id)).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(subscription["limit_per_minute"], 1000);
    }

    #[tokio::test]
    async fn billing_errors_map_to_http_statuses() {
        let cases = [
            (
                BillingError::NotFound("x".into()),
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
            ),
            (
                BillingError::Invalid("x".into()),
                StatusCode::UNPROCESSABLE_ENTITY,
                "PAYMENT_NOT_VALID",
            ),
            (
                BillingError::Conflict("x".into()),
                StatusCode::CONFLICT,
                "CONFLICT",
            ),
            (
                BillingError::Upstream("x".into()),
                StatusCode::BAD_GATEWAY,
                "HORIZON_UNAVAILABLE",
            ),
            (
                BillingError::Internal(anyhow::anyhow!("boom")),
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
            ),
        ];
        for (error, status, code) in cases {
            let (got, json) = body(billing_error(error)).await;
            assert_eq!(got, status);
            assert_eq!(json["error"], code);
        }
    }
}
