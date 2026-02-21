//! SEP-24 (Hosted Deposit and Withdrawal) proxy API.
//! Proxies requests to anchor transfer servers to avoid CORS and centralize auth.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Allowed transfer server hosts (env: SEP24_ALLOWED_ORIGINS, comma-separated).
/// If unset, any origin is allowed (use in dev only).
fn allowed_origins() -> Vec<String> {
    std::env::var("SEP24_ALLOWED_ORIGINS")
        .ok()
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default()
}

fn is_origin_allowed(transfer_server: &str) -> bool {
    let allowed = allowed_origins();
    if allowed.is_empty() {
        return true;
    }
    let url = match transfer_server.strip_suffix('/') {
        Some(u) => u,
        None => transfer_server,
    };
    allowed.iter().any(|o| url.starts_with(o) || o == "*")
}

#[derive(Clone)]
pub struct Sep24State {
    pub client: Arc<Client>,
}

impl Sep24State {
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
    let s = transfer_server.trim().trim_end_matches('/');
    s.to_string()
}

/// GET /api/sep24/info?transfer_server=<url>
#[derive(Debug, Deserialize)]
pub struct InfoQuery {
    pub transfer_server: String,
}

pub async fn get_info(
    State(state): State<Sep24State>,
    Query(q): Query<InfoQuery>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!("{}/info", base_url(&q.transfer_server));
    let resp = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep24Error::Anchor(status.as_u16(), body));
    }
    Ok(Json(body))
}

/// POST /api/sep24/deposit/interactive
#[derive(Debug, Deserialize)]
pub struct DepositInteractiveBody {
    pub transfer_server: String,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub memo_type: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    /// JWT from SEP-10 (optional for some anchors)
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

pub async fn post_deposit_interactive(
    State(state): State<Sep24State>,
    Json(body): Json<DepositInteractiveBody>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!(
        "{}/transactions/deposit/interactive",
        base_url(&body.transfer_server)
    );

    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {}", jwt));
    }
    let payload = serde_json::json!({
        "asset_code": body.asset_code,
        "account": body.account,
        "memo": body.memo,
        "memo_type": body.memo_type,
        "email": body.email,
        "amount": body.amount,
        "lang": body.lang,
    });
    let resp = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep24Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// POST /api/sep24/withdraw/interactive
#[derive(Debug, Deserialize)]
pub struct WithdrawInteractiveBody {
    pub transfer_server: String,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    #[serde(default)]
    pub memo_type: Option<String>,
    #[serde(default)]
    pub dest: Option<String>,
    #[serde(default)]
    pub dest_extra: Option<String>,
    #[serde(default)]
    pub amount: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

pub async fn post_withdraw_interactive(
    State(state): State<Sep24State>,
    Json(body): Json<WithdrawInteractiveBody>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!(
        "{}/transactions/withdraw/interactive",
        base_url(&body.transfer_server)
    );

    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {}", jwt));
    }
    let payload = serde_json::json!({
        "asset_code": body.asset_code,
        "account": body.account,
        "memo": body.memo,
        "memo_type": body.memo_type,
        "dest": body.dest,
        "dest_extra": body.dest_extra,
        "amount": body.amount,
        "lang": body.lang,
    });
    let resp = req
        .json(&payload)
        .send()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep24Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// GET /api/sep24/transactions?transfer_server=&jwt=&...
#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub transfer_server: String,
    #[serde(default)]
    pub jwt: Option<String>,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub cursor: Option<String>,
}

pub async fn get_transactions(
    State(state): State<Sep24State>,
    Query(q): Query<TransactionsQuery>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let base = base_url(&q.transfer_server);
    let mut url = format!("{}/transactions?", base);
    if let Some(c) = &q.asset_code {
        url.push_str(&format!("asset_code={}&", urlencoding::encode(c)));
    }
    if let Some(k) = &q.kind {
        url.push_str(&format!("kind={}&", urlencoding::encode(k)));
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
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep24Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// GET /api/sep24/transaction?transfer_server=&id=&jwt=
#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub transfer_server: String,
    pub id: String,
    #[serde(default)]
    pub jwt: Option<String>,
}

pub async fn get_transaction(
    State(state): State<Sep24State>,
    Query(q): Query<TransactionQuery>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let url = format!(
        "{}/transaction?id={}",
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
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    let status = resp.status();
    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;

    if !status.is_success() {
        return Err(Sep24Error::Anchor(status.as_u16(), data));
    }
    Ok(Json(data))
}

/// List known SEP-24-enabled anchors (from env or static list).
/// GET /api/sep24/anchors
#[derive(Debug, Serialize, Deserialize)]
pub struct Sep24AnchorInfo {
    pub name: String,
    pub transfer_server: String,
    pub home_domain: Option<String>,
}

pub async fn list_anchors() -> Json<Value> {
    // Env: SEP24_ANCHORS = JSON array of { "name", "transfer_server", "home_domain" }
    let anchors: Vec<Sep24AnchorInfo> = if let Ok(s) = std::env::var("SEP24_ANCHORS") {
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        // Default: no anchors; frontend can still use custom transfer_server
        vec![]
    };
    Json(serde_json::json!({ "anchors": anchors }))
}

#[derive(Debug)]
pub enum Sep24Error {
    Forbidden(String),
    Proxy(String),
    Anchor(u16, Value),
}

impl IntoResponse for Sep24Error {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match &self {
            Sep24Error::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                serde_json::json!({ "error": "forbidden", "message": msg }),
            ),
            Sep24Error::Proxy(msg) => (
                StatusCode::BAD_GATEWAY,
                serde_json::json!({ "error": "proxy", "message": msg }),
            ),
            Sep24Error::Anchor(code, data) => {
                let status = StatusCode::from_u16(*code).unwrap_or(StatusCode::BAD_GATEWAY);
                (status, data.clone())
            }
        };
        (status, Json(body)).into_response()
    }
}

/// Build SEP-24 API router
pub fn routes() -> axum::Router {
    let state = Sep24State::new();
    axum::Router::new()
        .route("/api/sep24/info", axum::routing::get(get_info))
        .route(
            "/api/sep24/deposit/interactive",
            axum::routing::post(post_deposit_interactive),
        )
        .route(
            "/api/sep24/withdraw/interactive",
            axum::routing::post(post_withdraw_interactive),
        )
        .route(
            "/api/sep24/transactions",
            axum::routing::get(get_transactions),
        )
        .route(
            "/api/sep24/transaction",
            axum::routing::get(get_transaction),
        )
        .route("/api/sep24/anchors", axum::routing::get(list_anchors))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url() {
        assert_eq!(
            base_url("https://api.example.com"),
            "https://api.example.com"
        );
        assert_eq!(
            base_url("https://api.example.com/"),
            "https://api.example.com"
        );
    }

    #[test]
    fn test_base_url_trim() {
        assert_eq!(
            base_url("  https://api.example.com  "),
            "https://api.example.com"
        );
    }

    #[test]
    fn test_deposit_interactive_body_deserialize() {
        let json = r#"{"transfer_server":"https://api.test.com","asset_code":"USDC"}"#;
        let body: DepositInteractiveBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.transfer_server, "https://api.test.com");
        assert_eq!(body.asset_code.as_deref(), Some("USDC"));
    }

    #[test]
    fn test_withdraw_interactive_body_deserialize() {
        let json = r#"{"transfer_server":"https://api.test.com","amount":"100"}"#;
        let body: WithdrawInteractiveBody = serde_json::from_str(json).unwrap();
        assert_eq!(body.transfer_server, "https://api.test.com");
        assert_eq!(body.amount.as_deref(), Some("100"));
    }
}
