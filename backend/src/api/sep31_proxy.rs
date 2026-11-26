//! SEP-31 (Cross-Border Payments) proxy API.
//! Proxies requests to anchor SEP-31 endpoints for quotes, payments, and KYC.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Maximum number of bytes we will read from an anchor response body.
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
/// Per-request timeout for upstream anchor calls.
const ANCHOR_TIMEOUT: Duration = Duration::from_secs(15);
/// Number of consecutive failures before the circuit opens for an anchor.
const CIRCUIT_FAILURE_THRESHOLD: u32 = 5;
/// How long the circuit stays open before allowing a probe request.
const CIRCUIT_OPEN_DURATION: Duration = Duration::from_secs(30);

fn allowed_origins() -> Vec<String> {
    std::env::var("SEP31_ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default()
}

fn is_origin_allowed(transfer_server: &str) -> bool {
    let allowed = allowed_origins();
    if allowed.is_empty() {
        return true;
    }
    crate::validation::transfer_server_allowed(transfer_server, &allowed)
}

/// Extract the host (domain) of an anchor URL, if it is a valid http(s) URL.
fn anchor_host(transfer_server: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(transfer_server.trim()).ok()?;
    match parsed.scheme() {
        "http" | "https" => parsed.host_str().map(str::to_string),
        _ => None,
    }
}

/// Returns true when `candidate` is an http(s) URL whose host matches `home_host`.
fn url_matches_home_domain(candidate: &str, home_host: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(candidate) else {
        return false;
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return false;
    }
    parsed
        .host_str()
        .map(|h| h.eq_ignore_ascii_case(home_host))
        .unwrap_or(false)
}

/// Recursively sanitise a JSON value: drop URL fields that do not match the
/// anchor home domain and strip unexpected keys from known SEP-31 objects.
fn sanitize_value(value: &mut Value, home_host: &str) {
    match value {
        Value::Object(map) => {
            map.retain(|key, val| {
                if key.ends_with("_url") || key == "url" {
                    if let Value::String(s) = val {
                        return url_matches_home_domain(s, home_host);
                    }
                }
                true
            });
            for val in map.values_mut() {
                sanitize_value(val, home_host);
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_value(item, home_host);
            }
        }
        _ => {}
    }
}

/// Read a response body with a hard size cap, then parse it as JSON.
async fn read_json_capped(resp: reqwest::Response) -> Result<Value, Sep31Error> {
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Sep31Error::Proxy(format!(
            "anchor response exceeded {MAX_RESPONSE_BYTES} byte limit"
        )));
    }
    serde_json::from_slice::<Value>(&bytes).map_err(|e| Sep31Error::Proxy(e.to_string()))
}

#[derive(Default)]
struct CircuitState {
    failures: u32,
    opened_at: Option<Instant>,
}

#[derive(Default)]
struct CircuitBreakers {
    inner: Mutex<HashMap<String, CircuitState>>,
}

impl CircuitBreakers {
    async fn check(&self, host: &str) -> Result<(), Sep31Error> {
        let map = self.inner.lock().await;
        if let Some(state) = map.get(host) {
            if let Some(opened) = state.opened_at {
                if opened.elapsed() < CIRCUIT_OPEN_DURATION {
                    return Err(Sep31Error::Proxy(format!(
                        "circuit open for anchor {host}; try again later"
                    )));
                }
            }
        }
        Ok(())
    }

    async fn record_success(&self, host: &str) {
        let mut map = self.inner.lock().await;
        map.remove(host);
    }

    async fn record_failure(&self, host: &str) {
        let mut map = self.inner.lock().await;
        let state = map.entry(host.to_string()).or_default();
        state.failures = state.failures.saturating_add(1);
        if state.failures >= CIRCUIT_FAILURE_THRESHOLD {
            state.opened_at = Some(Instant::now());
        }
    }
}

#[derive(Clone)]
pub struct Sep31State {
    pub client: Arc<Client>,
    pub breakers: Arc<CircuitBreakers>,
}

impl Default for Sep31State {
    fn default() -> Self {
        Self::new()
    }
}

impl Sep31State {
    #[must_use]
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(ANCHOR_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client: Arc::new(client),
            breakers: Arc::new(CircuitBreakers::default()),
        }
    }
}

fn base_url(transfer_server: &str) -> String {
    transfer_server.trim().trim_end_matches('/').to_string()
}

/// GET /`api/sep31/info?transfer_server`=<url>
#[derive(Debug, Deserialize)]
pub struct InfoQuery {
    pub transfer_server: String,
}

/// GET /api/sep31/info - Get SEP-31 anchor info
#[utoipa::path(
    get,
    path = "/api/sep31/info",
    operation_id = "sep31_get_info",
    params(
        ("transfer_server" = String, Query, description = "SEP-31 transfer server URL")
    ),
    responses(
        (status = 200, description = "SEP-31 anchor info"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn get_info(
    State(state): State<Sep31State>,
    Query(q): Query<InfoQuery>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let host = anchor_host(&q.transfer_server).ok_or_else(|| {
        Sep31Error::Forbidden("Transfer server must be a valid http(s) URL".to_string())
    })?;
    state.breakers.check(&host).await?;

    let url = format!("{}/info", base_url(&q.transfer_server));
    let resp = match state.client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(Sep31Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let mut body = match read_json_capped(resp).await {
        Ok(b) => b,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(e);
        }
    };

    if !status.is_success() {
        state.breakers.record_failure(&host).await;
        return Err(Sep31Error::Anchor(status.as_u16(), body));
    }
    state.breakers.record_success(&host).await;
    sanitize_value(&mut body, &host);
    Ok(Json(body))
}

/// POST /api/sep31/quote - get payment quote (SEP-38 style or anchor-specific)
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QuoteBody {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub payload: Value,
}

/// POST /api/sep31/quote - Create a SEP-31 payment quote
#[utoipa::path(
    post,
    path = "/api/sep31/quote",
    request_body = QuoteBody,
    responses(
        (status = 200, description = "Quote created successfully"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn post_quote(
    State(state): State<Sep31State>,
    Json(body): Json<QuoteBody>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let host = anchor_host(&body.transfer_server).ok_or_else(|| {
        Sep31Error::Forbidden("Transfer server must be a valid http(s) URL".to_string())
    })?;
    state.breakers.check(&host).await?;

    let url = format!("{}/quote", base_url(&body.transfer_server));
    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let resp = match req.json(&body.payload).send().await {
        Ok(r) => r,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(Sep31Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let mut data = match read_json_capped(resp).await {
        Ok(d) => d,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(e);
        }
    };

    if !status.is_success() {
        state.breakers.record_failure(&host).await;
        return Err(Sep31Error::Anchor(status.as_u16(), data));
    }
    state.breakers.record_success(&host).await;
    sanitize_value(&mut data, &host);
    Ok(Json(data))
}

/// POST /api/sep31/transactions - create cross-border payment
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateTransactionBody {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub payload: Value,
}

/// POST /api/sep31/transactions - Create a SEP-31 transaction
#[utoipa::path(
    post,
    path = "/api/sep31/transactions",
    request_body = CreateTransactionBody,
    responses(
        (status = 200, description = "Transaction created successfully"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn post_transaction(
    State(state): State<Sep31State>,
    Json(body): Json<CreateTransactionBody>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let host = anchor_host(&body.transfer_server).ok_or_else(|| {
        Sep31Error::Forbidden("Transfer server must be a valid http(s) URL".to_string())
    })?;
    state.breakers.check(&host).await?;

    let url = format!("{}/transactions", base_url(&body.transfer_server));
    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let resp = match req.json(&body.payload).send().await {
        Ok(r) => r,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(Sep31Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let mut data = match read_json_capped(resp).await {
        Ok(d) => d,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(e);
        }
    };

    if !status.is_success() {
        state.breakers.record_failure(&host).await;
        return Err(Sep31Error::Anchor(status.as_u16(), data));
    }
    state.breakers.record_success(&host).await;
    sanitize_value(&mut data, &host);
    Ok(Json(data))
}

/// GET /`api/sep31/transactions?transfer_server=&jwt`=&...
#[derive(Debug, Deserialize)]
pub struct ListTransactionsQuery {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub cursor: Option<String>,
}

/// GET /api/sep31/transactions - List SEP-31 transactions
#[utoipa::path(
    get,
    path = "/api/sep31/transactions",
    operation_id = "sep31_get_transactions",
    params(
        ("transfer_server" = String, Query, description = "SEP-31 transfer server URL"),
        ("jwt" = Option<String>, Query, description = "JWT authentication token"),
        ("status" = Option<String>, Query, description = "Filter by transaction status"),
        ("limit" = Option<u32>, Query, description = "Maximum number of transactions to return"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor")
    ),
    responses(
        (status = 200, description = "List of transactions"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn get_transactions(
    State(state): State<Sep31State>,
    Query(q): Query<ListTransactionsQuery>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let host = anchor_host(&q.transfer_server).ok_or_else(|| {
        Sep31Error::Forbidden("Transfer server must be a valid http(s) URL".to_string())
    })?;
    state.breakers.check(&host).await?;

    let base = base_url(&q.transfer_server);
    let mut url = format!("{base}/transactions?");
    if let Some(s) = &q.status {
        let _ = write!(url, "status={}&", urlencoding::encode(s));
    }
    if let Some(l) = q.limit {
        let _ = write!(url, "limit={l}&");
    }
    if let Some(c) = &q.cursor {
        let _ = write!(url, "cursor={}&", urlencoding::encode(c));
    }
    let mut req = state.client.get(&url);
    if let Some(jwt) = &q.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(Sep31Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let mut data = match read_json_capped(resp).await {
        Ok(d) => d,
        Err(e) => {
            state.breakers.record_failure(&host).await;
            return Err(e);
        }
    };

    if !status.is_success() {
        state.breakers.record_failure(&host).await;
        return Err(Sep31Error::Anchor(status.as_u16(), data));
    }
    state.breakers.record_success(&host).await;
    sanitize_value(&mut data, &host);
    Ok(Json(data))
}

/// GET /`api/sep31/transactions/:id?transfer_server=&jwt`=
#[derive(Debug, Deserialize)]
pub struct GetTransactionQuery {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
}

/// GET /api/sep31/transactions/{id} - Get a specific SEP-31 transaction
#[utoipa::path(
    get,
    path = "/api/sep31/transactions/{id}",
    operation_id = "sep31_get_transaction",
    params(
        ("id" = String, Path, description = "Transaction ID"),
        ("transfer_server" = String, Query, description = "SEP-31 transfer server URL"),
        ("jwt" = Option<String>, Query, description = "JWT authentication token")
    ),
    responses(
        (status = 200, description = "Transaction details"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 404, description = "Transaction not found"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn get_transaction(
    State(state): State<Sep31State>,
    Path(id): Path<String>,
    Query(q): Query<GetTransactionQuery>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!(
        "{}/transactions/{}",
        base_url(&q.transfer_server),
        urlencoding::encode(&id)
    );

    let mut req = state.client.get(&url);
    if let Some(jwt) = &q.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep31Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// GET /`api/sep31/customer?transfer_server=&jwt=&id`= - KYC customer fetch
#[derive(Debug, Deserialize)]
pub struct CustomerQuery {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    pub id: String,
}

/// GET /api/sep31/customer - Get KYC customer information
#[utoipa::path(
    get,
    path = "/api/sep31/customer",
    params(
        ("transfer_server" = String, Query, description = "SEP-31 transfer server URL"),
        ("jwt" = Option<String>, Query, description = "JWT authentication token"),
        ("id" = String, Query, description = "Customer ID")
    ),
    responses(
        (status = 200, description = "Customer information"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn get_customer(
    State(state): State<Sep31State>,
    Query(q): Query<CustomerQuery>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!(
        "{}/customer?id={}",
        base_url(&q.transfer_server),
        urlencoding::encode(&q.id)
    );

    let mut req = state.client.get(&url);
    if let Some(jwt) = &q.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep31Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// PUT /api/sep31/customer - KYC customer update (e.g. interactive callback)
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PutCustomerBody {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub payload: Value,
}

/// PUT /api/sep31/customer - Update KYC customer information
#[utoipa::path(
    put,
    path = "/api/sep31/customer",
    request_body = PutCustomerBody,
    responses(
        (status = 200, description = "Customer updated successfully"),
        (status = 403, description = "Transfer server not in allowed list"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-31"
)]
pub async fn put_customer(
    State(state): State<Sep31State>,
    Json(body): Json<PutCustomerBody>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!("{}/customer", base_url(&body.transfer_server));
    let mut req = state.client.put(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
    }
    let resp = req
        .json(&body.payload)
        .send()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep31Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// List configured SEP-31 anchors. GET /api/sep31/anchors
#[derive(Debug, Serialize, Deserialize)]
pub struct Sep31AnchorInfo {
    pub name: String,
    pub transfer_server: String,
    pub home_domain: Option<String>,
}

/// GET /api/sep31/anchors - List configured SEP-31 anchors
#[utoipa::path(
    get,
    path = "/api/sep31/anchors",
    operation_id = "sep31_list_anchors",
    responses(
        (status = 200, description = "List of configured SEP-31 anchors")
    ),
    tag = "SEP-31"
)]
pub async fn list_anchors() -> Json<Value> {
    let anchors: Vec<Sep31AnchorInfo> = if let Ok(s) = std::env::var("SEP31_ANCHORS") {
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        vec![]
    };
    Json(serde_json::json!({ "anchors": anchors }))
}

#[derive(Debug)]
pub enum Sep31Error {
    Forbidden(String),
    Proxy(String),
    Anchor(u16, Value),
}

impl IntoResponse for Sep31Error {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match &self {
            Self::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                serde_json::json!({ "error": "forbidden", "message": msg }),
            ),
            Self::Proxy(msg) => (
                StatusCode::BAD_GATEWAY,
                serde_json::json!({ "error": "proxy", "message": msg }),
            ),
            Self::Anchor(code, data) => {
                let status = StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY);
                (status, data.clone())
            }
        };
        (status, Json(body)).into_response()
    }
}

pub fn routes() -> axum::Router {
    let state = Sep31State::new();
    axum::Router::new()
        .route("/api/sep31/info", axum::routing::get(get_info))
        .route("/api/sep31/quote", axum::routing::post(post_quote))
        .route(
            "/api/sep31/transactions",
            axum::routing::get(get_transactions).post(post_transaction),
        )
        .route(
            "/api/sep31/transactions/{id}",
            axum::routing::get(get_transaction),
        )
        .route(
            "/api/sep31/customer",
            axum::routing::get(get_customer).put(put_customer),
        )
        .route("/api/sep31/anchors", axum::routing::get(list_anchors))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url() {
        assert_eq!(
            base_url("https://api.example.com/sep31"),
            "https://api.example.com/sep31"
        );
        assert_eq!(
            base_url("https://api.example.com/"),
            "https://api.example.com"
        );
    }

    #[test]
    fn test_quote_body_deserialize() {
        let json = r#"{"transfer_server":"https://api.test.com/sep31","payload":{"amount":"100","sell_asset":"USDC:issuer","buy_asset":"iso4217:USD"}}"#;
        let body: QuoteBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.transfer_server, "https://api.test.com/sep31");
    }

    #[test]
    fn test_create_transaction_body_deserialize() {
        let json = r#"{"transfer_server":"https://api.test.com/sep31","payload":{"amount":"100","receiver_id":"receiver123"}}"#;
        let body: CreateTransactionBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.transfer_server, "https://api.test.com/sep31");
    }
}
