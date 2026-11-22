//! SEP-24 (Hosted Deposit and Withdrawal) proxy API.
//! Proxies requests to anchor transfer servers to avoid CORS and centralize auth.
//!
//! Hardening: anchor responses are size-bounded, schema-validated (unexpected
//! fields stripped), and URL fields are restricted to the anchor's home domain.

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Maximum accepted anchor response body size (bytes).
const MAX_RESPONSE_BYTES: usize = 512 * 1024;
/// Per-request upstream timeout.
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(15);
/// Circuit breaker: consecutive failures before opening.
const CB_FAILURE_THRESHOLD: u32 = 5;
/// Circuit breaker: how long to stay open before probing again.
const CB_OPEN_DURATION: Duration = Duration::from_secs(30);

/// Allowed transfer server hosts (env: `SEP24_ALLOWED_ORIGINS`, comma-separated).
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

/// Extract the host (authority) portion of an `http(s)` URL, lowercased.
fn url_host(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?; // drop any userinfo
    let host = host.split(':').next()?; // drop port
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

/// True if `url` is an `http(s)` URL whose host matches `home_domain`.
fn url_matches_home_domain(url: &str, home_domain: &str) -> bool {
    let home = home_domain
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if home.is_empty() {
        return false;
    }
    match url_host(url) {
        Some(h) => h == home || h.ends_with(&format!(".{home}")),
        None => false,
    }
}

/// Sanitize a URL-valued field: keep only if it is http(s) and matches the
/// anchor home domain; otherwise drop it.
fn sanitize_url_field(map: &mut Map<String, Value>, key: &str, home_domain: &str) {
    if let Some(Value::String(u)) = map.get(key) {
        if !url_matches_home_domain(u, home_domain) {
            map.remove(key);
        }
    }
}

/// Keep only `allowed` keys from a JSON object, dropping everything else.
fn strip_fields(value: &mut Value, allowed: &[&str]) {
    if let Value::Object(map) = value {
        map.retain(|k, _| allowed.contains(&k.as_str()));
    }
}

/// Validate and sanitize a SEP-24 `/info` response.
fn sanitize_info(mut body: Value, home_domain: &str) -> Value {
    if let Value::Object(map) = &mut body {
        map.retain(|k, _| k == "deposit" || k == "withdraw" || k == "fee" || k == "features");
        for section in ["deposit", "withdraw"] {
            if let Some(Value::Object(assets)) = map.get_mut(section) {
                for asset in assets.values_mut() {
                    if let Value::Object(a) = asset {
                        a.retain(|k, _| {
                            matches!(
                                k.as_str(),
                                "enabled"
                                    | "authentication_required"
                                    | "min_amount"
                                    | "max_amount"
                                    | "fee_fixed"
                                    | "fee_percent"
                                    | "fee_minimum"
                                    | "fields"
                            )
                        });
                    }
                }
            }
        }
    }
    body
}

/// Validate and sanitize a SEP-24 interactive / transaction response.
fn sanitize_interactive(mut body: Value, home_domain: &str) -> Value {
    if let Value::Object(map) = &mut body {
        map.retain(|k, _| {
            matches!(
                k.as_str(),
                "type" | "url" | "id" | "more_info_url" | "status" | "message"
            )
        });
        sanitize_url_field(map, "url", home_domain);
        sanitize_url_field(map, "more_info_url", home_domain);
    }
    body
}

/// Read a response body with a hard size cap.
async fn read_bounded(resp: reqwest::Response) -> Result<Value, Sep24Error> {
    if let Some(len) = resp.content_length() {
        if len as usize > MAX_RESPONSE_BYTES {
            return Err(Sep24Error::Proxy("anchor response too large".to_string()));
        }
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| Sep24Error::Proxy(e.to_string()))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Sep24Error::Proxy("anchor response too large".to_string()));
    }
    serde_json::from_slice::<Value>(&bytes).map_err(|e| Sep24Error::Proxy(e.to_string()))
}

/// Minimal per-anchor circuit breaker.
#[derive(Default)]
struct CircuitBreaker {
    failures: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    fn is_open(&mut self) -> bool {
        if let Some(at) = self.opened_at {
            if at.elapsed() >= CB_OPEN_DURATION {
                self.opened_at = None;
                self.failures = 0;
                return false;
            }
            return true;
        }
        false
    }

    fn record_success(&mut self) {
        self.failures = 0;
        self.opened_at = None;
    }

    fn record_failure(&mut self) {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= CB_FAILURE_THRESHOLD {
            self.opened_at = Some(Instant::now());
        }
    }
}

#[derive(Clone)]
pub struct Sep24State {
    pub client: Arc<Client>,
    breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
}

impl Default for Sep24State {
    fn default() -> Self {
        Self::new()
    }
}

impl Sep24State {
    #[must_use]
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(UPSTREAM_TIMEOUT)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client: Arc::new(client),
            breakers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns `Err` if the anchor's circuit breaker is open.
    fn check_breaker(&self, anchor: &str) -> Result<(), Sep24Error> {
        let mut map = self.breakers.lock().unwrap_or_else(|e| e.into_inner());
        let cb = map.entry(anchor.to_string()).or_default();
        if cb.is_open() {
            return Err(Sep24Error::Proxy(
                "anchor temporarily unavailable (circuit open)".to_string(),
            ));
        }
        Ok(())
    }

    fn record(&self, anchor: &str, ok: bool) {
        let mut map = self.breakers.lock().unwrap_or_else(|e| e.into_inner());
        let cb = map.entry(anchor.to_string()).or_default();
        if ok {
            cb.record_success();
        } else {
            cb.record_failure();
        }
    }
}

fn base_url(transfer_server: &str) -> String {
    let s = transfer_server.trim().trim_end_matches('/');
    s.to_string()
}

/// GET /api/sep24/info - Get SEP-24 server information
#[derive(Debug, Deserialize)]
pub struct InfoQuery {
    pub transfer_server: String,
}

#[utoipa::path(
    get,
    path = "/api/sep24/info",
    operation_id = "sep24_get_info",
    params(
        ("transfer_server" = String, Query, description = "Transfer server URL")
    ),
    responses(
        (status = 200, description = "SEP-24 server information"),
        (status = 403, description = "Transfer server not allowed"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-24"
)]
pub async fn get_info(
    State(state): State<Sep24State>,
    Query(q): Query<InfoQuery>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&q.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let anchor = base_url(&q.transfer_server);
    state.check_breaker(&anchor)?;
    let url = format!("{anchor}/info");
    let resp = match state.client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            state.record(&anchor, false);
            return Err(Sep24Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let body = match read_bounded(resp).await {
        Ok(b) => b,
        Err(e) => {
            state.record(&anchor, false);
            return Err(e);
        }
    };

    if !status.is_success() {
        state.record(&anchor, false);
        return Err(Sep24Error::Anchor(status.as_u16(), body));
    }
    state.record(&anchor, true);
    Ok(Json(sanitize_info(body, &anchor)))
}

/// POST /api/sep24/deposit/interactive
#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

/// POST /api/sep24/deposit/interactive - Initiate interactive deposit
#[utoipa::path(
    post,
    path = "/api/sep24/deposit/interactive",
    request_body = DepositInteractiveBody,
    responses(
        (status = 200, description = "Interactive deposit started"),
        (status = 403, description = "Transfer server not allowed"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-24"
)]
pub async fn post_deposit_interactive(
    State(state): State<Sep24State>,
    Json(body): Json<DepositInteractiveBody>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let anchor = base_url(&body.transfer_server);
    state.check_breaker(&anchor)?;
    let url = format!("{anchor}/transactions/deposit/interactive");

    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
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
    let resp = match req.json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            state.record(&anchor, false);
            return Err(Sep24Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let data = match read_bounded(resp).await {
        Ok(d) => d,
        Err(e) => {
            state.record(&anchor, false);
            return Err(e);
        }
    };

    if !status.is_success() {
        state.record(&anchor, false);
        return Err(Sep24Error::Anchor(status.as_u16(), data));
    }
    state.record(&anchor, true);
    Ok(Json(sanitize_interactive(data, &anchor)))
}

/// POST /api/sep24/withdraw/interactive
#[derive(Debug, Deserialize, utoipa::ToSchema)]
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

/// POST /api/sep24/withdraw/interactive - Initiate interactive withdrawal
#[utoipa::path(
    post,
    path = "/api/sep24/withdraw/interactive",
    request_body = WithdrawInteractiveBody,
    responses(
        (status = 200, description = "Interactive withdrawal started"),
        (status = 403, description = "Transfer server not allowed"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-24"
)]
pub async fn post_withdraw_interactive(
    State(state): State<Sep24State>,
    Json(body): Json<WithdrawInteractiveBody>,
) -> Result<Json<Value>, Sep24Error> {
    if !is_origin_allowed(&body.transfer_server) {
        return Err(Sep24Error::Forbidden(
            "Transfer server not in allowed list".to_string(),
        ));
    }
    let anchor = base_url(&body.transfer_server);
    state.check_breaker(&anchor)?;
    let url = format!("{anchor}/transactions/withdraw/interactive");

    let mut req = state.client.post(&url);
    if let Some(jwt) = &body.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
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
    let resp = match req.json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            state.record(&anchor, false);
            return Err(Sep24Error::Proxy(e.to_string()));
        }
    };

    let status = resp.status();
    let data = match read_bounded(resp).await {
        Ok(d) => d,
        Err(e) => {
            state.record(&anchor, false);
            return Err(e);
        }
    };

    if !status.is_success() {
        state.record(&anchor, false);
        return Err(Sep24Error::Anchor(status.as_u16(), data));
    }
    state.record(&anchor, true);
    Ok(Json(sanitize_interactive(data, &anchor)))
}

/// GET /`api/sep24/transactions?transfer_server=&jwt`=&...
#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub transfer_server: String,
    #[serde(d

/* … truncated 18232 chars — edit only what you need near the top … */
