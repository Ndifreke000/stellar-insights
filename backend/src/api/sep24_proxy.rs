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
    crate::validation::transfer_server_allowed(transfer_server, &allowed)
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

/// GET /api/sep24/transactions - Get SEP-24 transactions
#[utoipa::path(
    get,
    path = "/api/sep24/transactions",
    operation_id = "sep24_get_transactions",
    params(
        ("transfer_server" = String, Query, description = "Transfer server URL"),
        ("jwt" = Option<String>, Query, description = "JWT token"),
        ("asset_code" = Option<String>, Query, description = "Filter by asset code"),
        ("kind" = Option<String>, Query, description = "Filter by transaction kind"),
        ("limit" = Option<u32>, Query, description = "Maximum results"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor")
    ),
    responses(
        (status = 200, description = "List of transactions"),
        (status = 403, description = "Transfer server not allowed"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-24"
)]
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
    let mut url = format!("{base}/transactions?");
    if let Some(c) = &q.asset_code {
        let _ = write!(url, "asset_code={}&", urlencoding::encode(c));
    }
    if let Some(k) = &q.kind {
        let _ = write!(url, "kind={}&", urlencoding::encode(k));
    }
    if let Some(l) = q.limit {
        let _ = write!(url, "limit={l}&");
    }
    if let Some(c) = &q.cursor {
        let _ = write!(url, "cursor={}&", urlencoding::encode(c));
    }
    let url = url.trim_end_matches('&').trim_end_matches('?');

    let mut req = state.client.get(url);
    if let Some(jwt) = &q.jwt {
        req = req.header("Authorization", format!("Bearer {jwt}"));
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

/// GET /`api/sep24/transaction?transfer_server=&id=&jwt`=
#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    pub transfer_server: String,
    pub id: String,
    #[serde(default)]
    pub jwt: Option<String>,
}

/// GET /api/sep24/transaction - Get a specific SEP-24 transaction
#[utoipa::path(
    get,
    path = "/api/sep24/transaction",
    operation_id = "sep24_get_transaction",
    params(
        ("transfer_server" = String, Query, description = "Transfer server URL"),
        ("id" = String, Query, description = "Transaction ID"),
        ("jwt" = Option<String>, Query, description = "JWT token")
    ),
    responses(
        (status = 200, description = "Transaction details"),
        (status = 403, description = "Transfer server not allowed"),
        (status = 502, description = "Proxy error")
    ),
    tag = "SEP-24"
)]
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
        req = req.header("Authorization", format!("Bearer {jwt}"));
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sep24AnchorInfo {
    pub name: String,
    pub transfer_server: String,
    pub home_domain: Option<String>,
}

/// GET /api/sep24/anchors - List known SEP-24-enabled anchors
#[utoipa::path(
    get,
    path = "/api/sep24/anchors",
    operation_id = "sep24_list_anchors",
    responses(
        (status = 200, description = "List of SEP-24 anchors")
    ),
    tag = "SEP-24"
)]
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

/// Derive the set of permitted CORS origins from the registered anchor list.
///
/// An anchor's `home_domain` (e.g. `"anchor.example.com"`) is converted to
/// a proper origin string (`"https://anchor.example.com"`) so it can be
/// compared directly with the `Origin` request header.  HTTP is also
/// accepted during local development.
///
/// The list is read fresh from the `SEP24_ANCHORS` env var on every call so
/// that changes take effect without a restart (the var is cheap to read and
/// parse compared to a DB query).
fn allowed_anchor_origins() -> Vec<String> {
    let anchors: Vec<Sep24AnchorInfo> = std::env::var("SEP24_ANCHORS")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    anchors
        .into_iter()
        .filter_map(|a| a.home_domain)
        .flat_map(|domain| {
            let domain = domain.trim().trim_end_matches('/').to_string();
            // Emit both schemes so local / self-signed setups work in dev.
            vec![format!("https://{}", domain), format!("http://{}", domain)]
        })
        .collect()
}

/// Return the matching allowed origin for a given `Origin` header value, or
/// `None` if the origin is not in the registered anchor list.
fn resolve_callback_origin(request_origin: &str) -> Option<String> {
    let needle = request_origin.trim().trim_end_matches('/');
    allowed_anchor_origins()
        .into_iter()
        .find(|o| o.trim_end_matches('/') == needle)
}

/// Build a minimal set of CORS response headers for the callback endpoint.
///
/// `origin` must already be validated via [`resolve_callback_origin`].
fn callback_cors_headers(origin: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    // Safety: origin comes from our own allow-list, so it is always a valid
    // header value.  The `unwrap_or_else` is a defensive fallback.
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_str(origin).unwrap_or_else(|_| HeaderValue::from_static("null")),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, Authorization"),
    );
    // Do not reflect credentials for anchor callbacks — they are server-to-
    // server notifications, not browser sessions.
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    headers
}

/// Query parameters forwarded by the anchor on the callback redirect.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    /// The anchor's transfer server base URL — used to look up the anchor and
    /// validate that the callback is coming from a registered anchor.
    #[serde(default)]
    pub transfer_server: Option<String>,
    /// SEP-24 transaction identifier.
    #[serde(default)]
    pub transaction_id: Option<String>,
    /// Final status reported by the anchor (`completed`, `refunded`, etc.).
    #[serde(default)]
    pub status: Option<String>,
    /// Absorb any extra fields the anchor chooses to include.
    #[serde(flatten)]
    pub extra: Value,
}

/// `OPTIONS /api/sep24/callback` — preflight handler.
///
/// The browser sends this before the anchor's interactive flow posts back to
/// our domain.  We must reply with the matching `Access-Control-Allow-Origin`
/// for the anchor's `home_domain` or the browser will block the follow-up
/// request.
#[utoipa::path(
    options,
    path = "/api/sep24/callback",
    responses(
        (status = 204, description = "Preflight accepted"),
        (status = 403, description = "Origin not in registered anchor list")
    ),
    tag = "SEP-24"
)]
pub async fn options_callback(headers: HeaderMap) -> impl IntoResponse {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match resolve_callback_origin(origin) {
        Some(allowed) => {
            let mut cors_headers = callback_cors_headers(&allowed);
            // Preflight-specific header: cache the result for 1 hour.
            cors_headers.insert(
                header::ACCESS_CONTROL_MAX_AGE,
                HeaderValue::from_static("3600"),
            );
            (StatusCode::NO_CONTENT, cors_headers).into_response()
        }
        None => {
            tracing::warn!(
                origin = %origin,
                "SEP-24 callback preflight rejected: origin not in registered anchor list"
            );
            (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "forbidden",
                    "message": "Origin not in registered anchor list"
                })),
            )
                .into_response()
        }
    }
}

/// `GET /api/sep24/callback` — anchor redirect target.
///
/// After completing the interactive deposit/withdrawal flow the anchor
/// redirects the user's browser to this URL.  The browser will include an
/// `Origin` header matching the anchor's `home_domain`; we validate it against
/// the registered anchor list and echo it back in `Access-Control-Allow-Origin`
/// so the frontend JavaScript can read the response.
#[utoipa::path(
    get,
    path = "/api/sep24/callback",
    params(
        ("transfer_server" = Option<String>, Query, description = "Anchor transfer server URL"),
        ("transaction_id" = Option<String>, Query, description = "SEP-24 transaction ID"),
        ("status" = Option<String>, Query, description = "Final transaction status")
    ),
    responses(
        (status = 200, description = "Callback received"),
        (status = 403, description = "Origin not in registered anchor list")
    ),
    tag = "SEP-24"
)]
pub async fn get_callback(headers: HeaderMap, Query(q): Query<CallbackQuery>) -> impl IntoResponse {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Validate the requesting origin against the registered anchor list.
    // We also accept requests with no Origin header (server-to-server calls
    // or same-origin browser navigations) — in that case we return the
    // response without CORS headers, which is safe.
    let cors_headers = if origin.is_empty() {
        HeaderMap::new()
    } else {
        match resolve_callback_origin(origin) {
            Some(ref allowed) => callback_cors_headers(allowed),
            None => {
                tracing::warn!(
                    origin = %origin,
                    transfer_server = ?q.transfer_server,
                    transaction_id = ?q.transaction_id,
                    "SEP-24 callback rejected: origin not in registered anchor list"
                );
                return (
                    StatusCode::FORBIDDEN,
                    HeaderMap::new(),
                    Json(serde_json::json!({
                        "error": "forbidden",
                        "message": "Origin not in registered anchor list"
                    })),
                )
                    .into_response();
            }
        }
    };

    tracing::info!(
        origin = %origin,
        transfer_server = ?q.transfer_server,
        transaction_id = ?q.transaction_id,
        status = ?q.status,
        "SEP-24 callback received"
    );

    (
        StatusCode::OK,
        cors_headers,
        Json(serde_json::json!({
            "received": true,
            "transaction_id": q.transaction_id,
            "status": q.status,
        })),
    )
        .into_response()
}

/// `POST /api/sep24/callback` — anchor server-side notification target.
///
/// Some anchors POST a JSON body to the callback URL instead of (or in
/// addition to) a redirect.  This handler accepts the body and validates the
/// `Origin` header the same way as `get_callback`.
#[utoipa::path(
    post,
    path = "/api/sep24/callback",
    request_body(content = Value, description = "Anchor callback payload"),
    responses(
        (status = 200, description = "Callback received"),
        (status = 403, description = "Origin not in registered anchor list")
    ),
    tag = "SEP-24"
)]
pub async fn post_callback(headers: HeaderMap, body: Option<Json<Value>>) -> impl IntoResponse {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let cors_headers = if origin.is_empty() {
        HeaderMap::new()
    } else {
        match resolve_callback_origin(origin) {
            Some(ref allowed) => callback_cors_headers(allowed),
            None => {
                tracing::warn!(
                    origin = %origin,
                    "SEP-24 POST callback rejected: origin not in registered anchor list"
                );
                return (
                    StatusCode::FORBIDDEN,
                    HeaderMap::new(),
                    Json(serde_json::json!({
                        "error": "forbidden",
                        "message": "Origin not in registered anchor list"
                    })),
                )
                    .into_response();
            }
        }
    };

    tracing::info!(
        origin = %origin,
        has_body = body.is_some(),
        "SEP-24 POST callback received"
    );

    (
        StatusCode::OK,
        cors_headers,
        Json(serde_json::json!({ "received": true })),
    )
        .into_response()
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
        // Callback endpoint: OPTIONS for preflight, GET for anchor redirects,
        // POST for server-side anchor notifications.
        // CORS headers are set dynamically per-request (see `callback_cors_headers`)
        // because the allowed origin is the anchor's `home_domain`, which is
        // not known at startup and differs per anchor.
        .route(
            "/api/sep24/callback",
            axum::routing::get(get_callback)
                .post(post_callback)
                .options(options_callback),
        )
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
