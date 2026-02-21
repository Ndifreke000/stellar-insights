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
use std::sync::Arc;
use std::time::Duration;

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
    let url = transfer_server.trim().trim_end_matches('/');
    allowed.iter().any(|o| url.starts_with(o) || o == "*")
}

#[derive(Clone)]
pub struct Sep31State {
    pub client: Arc<Client>,
}

impl Sep31State {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client: Arc::new(client),
        }
    }
}

fn base_url(transfer_server: &str) -> String {
    transfer_server.trim().trim_end_matches('/').to_string()
}

/// GET /api/sep31/info?transfer_server=<url>
#[derive(Debug, Deserialize)]
pub struct InfoQuery {
    pub transfer_server: String,
}

pub async fn get_info(
    State(state): State<Sep31State>,
    Query(q): Query<InfoQuery>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!("{}/info", base_url(&q.transfer_server));
    let resp = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep31Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep31Error::Anchor(status.as_u16(), body));
    }
    Ok(Json(body))
}

/// POST /api/sep31/quote - get payment quote (SEP-38 style or anchor-specific)
#[derive(Debug, Deserialize)]
pub struct QuoteBody {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub payload: Value,
}

pub async fn post_quote(
    State(state): State<Sep31State>,
    Json(body): Json<QuoteBody>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!("{}/quote", base_url(&body.transfer_server));
    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {}", jwt));
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

/// POST /api/sep31/transactions - create cross-border payment
#[derive(Debug, Deserialize)]
pub struct CreateTransactionBody {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub payload: Value,
}

pub async fn post_transaction(
    State(state): State<Sep31State>,
    Json(body): Json<CreateTransactionBody>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!("{}/transactions", base_url(&body.transfer_server));
    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {}", jwt));
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

/// GET /api/sep31/transactions?transfer_server=&jwt=&...
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

pub async fn get_transactions(
    State(state): State<Sep31State>,
    Query(q): Query<ListTransactionsQuery>,
) -> Result<Json<Value>, Sep31Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep31Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let base = base_url(&q.transfer_server);
    let mut url = format!("{}/transactions?", base);
    if let Some(s) = &q.status {
        url.push_str(&format!("status={}&", urlencoding::encode(s)));
    }
    if let Some(l) = q.limit {
        url.push_str(&format!("limit={}&", l));
    }
    if let Some(c) = &q.cursor {
        url.push_str(&format!("cursor={}&", urlencoding::encode(c)));
    }
    let url = url.trim_end_matches('&').trim_end_matches('?');

    let mut req = state.client.get(url);
    if let Some(jwt) = &q.jwt {
        req = req.header("Authorization", format!("Bearer {}", jwt));
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

/// GET /api/sep31/transactions/:id?transfer_server=&jwt=
#[derive(Debug, Deserialize)]
pub struct GetTransactionQuery {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
}

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
        req = req.header("Authorization", format!("Bearer {}", jwt));
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

/// GET /api/sep31/customer?transfer_server=&jwt=&id= - KYC customer fetch
#[derive(Debug, Deserialize)]
pub struct CustomerQuery {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    pub id: String,
}

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
        req = req.header("Authorization", format!("Bearer {}", jwt));
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
#[derive(Debug, Deserialize)]
pub struct PutCustomerBody {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub payload: Value,
}

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
        req = req.header("Authorization", format!("Bearer {}", jwt));
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
            Sep31Error::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                serde_json::json!({ "error": "forbidden", "message": msg }),
            ),
            Sep31Error::Proxy(msg) => (
                StatusCode::BAD_GATEWAY,
                serde_json::json!({ "error": "proxy", "message": msg }),
            ),
            Sep31Error::Anchor(code, data) => {
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
            "/api/sep31/transactions/:id",
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
