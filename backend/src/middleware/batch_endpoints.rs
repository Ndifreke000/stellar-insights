//! Hardened batch request dispatcher and middleware audit remediation (Issue #2832).
//!
//! # Security invariants
//!
//! Every sub-request dispatched through [`BatchDispatcher`] is guaranteed to:
//!
//! 1. **Inherit verified outer caller identity only** — any `Authorization`,
//!    `Cookie`, `X-Forwarded-For`, `X-Real-IP`, `Forwarded`, `Host`, or
//!    `X-API-Key` header supplied inside a sub-request payload is stripped and
//!    replaced by the outer HTTP request's verified `Authorization` header and
//!    peer `ConnectInfo<SocketAddr>`.
//! 2. **Re-run route middleware per sub-request** — sub-requests are dispatched
//!    through the full Axum `Router` (including `auth_middleware`, `AdminUser`,
//!    `sep10_auth_middleware`, `ip_whitelist_middleware`, and
//!    `api_versioning_middleware`), never invoking raw handlers directly.
//! 3. **Charge rate limits per sub-request** — a batch of `N` sub-requests
//!    consumes `N` units from the caller's rate-limit bucket (and per-API-key
//!    bucket when using an API key), preventing batch rate-limit amplification.
//! 4. **Enforce admin IP whitelisting fail-closed** — when an `IpWhitelistConfig`
//!    is configured, any sub-request targeting an `/admin` path is verified
//!    against the real outer client IP before dispatch.
//! 5. **Enforce batch size, cost, path canonicalization, and anti-recursion caps** —
//!    blocks nested `/batch` calls, path traversal (`..` / `%2e%2e`), absolute
//!    URLs/schemes, and batches exceeding `max_batch_size` or `max_total_cost`.
//! 6. **Authorize field selection strictly after role resolution** — applies
//!    [`FieldSelectionParameter`] with the outer caller's verified `is_admin`
//!    flag and never alters non-2xx error responses.

use anyhow::{bail, Result};
use axum::{
    body::{to_bytes, Body},
    extract::{ConnectInfo, Extension, Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Json,
    Router,
};
use serde::Serialize;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use crate::auth_middleware::{
    resolve_optional_auth_user, AuthUser, JwtSecret, TokenRevocationStore,
};
use crate::error::{ApiError, ApiResult, ErrorDetail};
use crate::ip_whitelist_middleware::IpWhitelistConfig;
use crate::middleware::field_selection_parameter::{
    extract_fields_query, FieldSelectionParameter,
};
use crate::models::{
    batch_endpoints::{
        BatchConfig, BatchHttpRequest, BatchHttpResponse, BatchItemResult, BatchRequest,
        BatchResponse, BatchSubRequest, BatchSubResponse,
    },
    field_selection_parameter::FieldSelectionConfig,
    network_context_middleware::NetworkContext,
};
use crate::rate_limit::{OuterRateLimitCharged, RateLimiter};
use crate::request_id::RequestId;

/// Maximum buffered response body per sub-request (2 MiB).
const MAX_SUBRESPONSE_BYTES: usize = 2 * 1024 * 1024;

/// Headers that a batch sub-request payload is NEVER allowed to set or override.
const STRIPPED_SUBREQUEST_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "x-forwarded-for",
    "x-real-ip",
    "forwarded",
    "cf-connecting-ip",
    "true-client-ip",
    "host",
    "x-api-key",
    "x-auth-user",
    "x-admin",
    "x-forwarded-host",
    "x-forwarded-proto",
];

/// Verified caller identity extracted strictly from the outer HTTP request.
#[derive(Debug, Clone)]
pub struct BatchCallerIdentity {
    /// Raw `Authorization` header copied verbatim from the outer request.
    pub authorization_header: Option<String>,
    /// Verified client IP extracted from the outer request via `client_ip`.
    pub client_ip: IpAddr,
    /// Verified peer socket address propagated to sub-requests via `ConnectInfo`.
    pub peer_addr: SocketAddr,
    /// Optional authenticated user if already resolved on the outer request.
    pub auth_user: Option<AuthUser>,
    /// Correlation request ID from the outer request.
    pub request_id: Option<String>,
    /// If outer `rate_limit_middleware` already debited 1 unit for the outer
    /// request, this holds the endpoint key it debited so we debit exactly `N`
    /// units total for a batch of `N` items (instead of `N + 1`).
    pub outer_rate_limit_endpoint: Option<String>,
}

impl BatchCallerIdentity {
    /// Build verified caller identity from outer HTTP request parts.
    #[must_use]
    pub fn from_outer_request(
        headers: &HeaderMap,
        peer: Option<SocketAddr>,
        auth_user: Option<AuthUser>,
        request_id: Option<String>,
        outer_rate_limit_endpoint: Option<String>,
    ) -> Self {
        let authorization_header = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let client_ip = crate::client_ip::client_ip(headers, peer)
            .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let peer_addr = peer.unwrap_or_else(|| SocketAddr::new(client_ip, 0));

        Self {
            authorization_header,
            client_ip,
            peer_addr,
            auth_user,
            request_id,
            outer_rate_limit_endpoint,
        }
    }

    /// Return bearer token stripped of the `"Bearer "` prefix, if present.
    #[must_use]
    pub fn bearer_token(&self) -> Option<&str> {
        self.authorization_header
            .as_deref()
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
    }
}

/// A boundary-validated sub-request ready for dispatch.
#[derive(Debug, Clone)]
pub struct ValidatedSubRequest {
    pub index: usize,
    pub method: Method,
    pub canonical_path: String,
    pub query: Option<String>,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body_bytes: Vec<u8>,
    pub fields_param: Option<String>,
    pub cost: u32,
}

/// Compute the deterministic cost weight of a sub-request.
///
/// Export, analytics, preflight, and admin routes carry higher weight to
/// prevent cost amplification through batching.
#[must_use]
pub fn estimate_subrequest_cost(method: &Method, canonical_path: &str) -> u32 {
    let lower = canonical_path.to_ascii_lowercase();
    let mut cost: u32 = match *method {
        Method::GET | Method::HEAD => 1,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE => 3,
        _ => 3,
    };

    if lower.contains("/export/")
        || lower.contains("/analytics/")
        || lower.contains("/preflight")
        || is_admin_path(&lower)
    {
        cost = cost.saturating_add(4);
    }

    cost
}

/// Returns `true` when `path` targets an administrative endpoint.
#[must_use]
pub fn is_admin_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower == "/admin"
        || lower.starts_with("/admin/")
        || lower.contains("/admin/")
        || lower.ends_with("/admin")
}

/// Returns `true` when `path` targets a batch endpoint (recursion check).
#[must_use]
pub fn is_batch_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower == "/batch"
        || lower.ends_with("/batch")
        || lower.contains("/batch/")
        || lower.contains("/preflight/batch")
}

/// Validate and normalize a sub-request path, rejecting traversal, scheme
/// injection, control characters, and nested batch paths.
pub fn validate_subrequest_path(
    raw_path: &str,
    deny_nested_batch: bool,
) -> Result<(String, Option<String>), ApiError> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') {
        return Err(ApiError::bad_request(
            "INVALID_SUBREQUEST_PATH",
            "Sub-request path must be a non-empty relative path starting with '/'",
        ));
    }

    if trimmed.starts_with("//")
        || trimmed.contains("://")
        || trimmed.contains('\\')
        || trimmed.bytes().any(|b| b < 0x20 || b == 0x7f)
    {
        return Err(ApiError::bad_request(
            "INVALID_SUBREQUEST_PATH",
            "Sub-request path contains forbidden characters or scheme",
        ));
    }

    let (path_only, query) = match trimmed.split_once('?') {
        Some((p, q)) => (p, Some(q.to_string())),
        None => (trimmed, None),
    };

    // Decode percent-encoding to detect encoded traversal (`%2e%2e`, `%2f`).
    let decoded = urlencoding::decode(path_only).map_err(|_| {
        ApiError::bad_request(
            "INVALID_SUBREQUEST_PATH",
            "Sub-request path contains invalid percent-encoding",
        )
    })?;

    if decoded.contains('\\') || decoded.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(ApiError::bad_request(
            "INVALID_SUBREQUEST_PATH",
            "Decoded sub-request path contains forbidden characters",
        ));
    }

    for segment in decoded.split('/') {
        if segment == ".." || segment == "." {
            return Err(ApiError::bad_request(
                "PATH_TRAVERSAL_REJECTED",
                "Sub-request path must not contain dot segments",
            ));
        }
    }

    // Collapse repeated slashes into a canonical single-slash path.
    let mut canonical = String::with_capacity(path_only.len());
    let mut prev_slash = false;
    for ch in path_only.chars() {
        if ch == '/' {
            if !prev_slash {
                canonical.push('/');
                prev_slash = true;
            }
        } else {
            canonical.push(ch);
            prev_slash = false;
        }
    }
    if canonical.len() > 1 && canonical.ends_with('/') {
        canonical.pop();
    }

    if deny_nested_batch && (is_batch_path(&canonical) || is_batch_path(&decoded)) {
        return Err(ApiError::bad_request(
            "NESTED_BATCH_FORBIDDEN",
            "Batch sub-requests cannot target batch endpoints",
        ));
    }

    Ok((canonical, query))
}

fn validate_subrequest_method(raw_method: &str) -> Result<Method, ApiError> {
    match raw_method.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "PATCH" => Ok(Method::PATCH),
        "DELETE" => Ok(Method::DELETE),
        other => Err(ApiError::bad_request(
            "INVALID_SUBREQUEST_METHOD",
            format!("HTTP method '{other}' is not permitted in batch requests"),
        )),
    }
}

/// Filter sub-request headers, dropping every security-sensitive header so
/// callers cannot spoof identity, IP address, or cookies inside a batch.
fn sanitize_subrequest_headers(raw: &HashMap<String, String>) -> HashMap<String, String> {
    raw.iter()
        .filter(|(k, _)| {
            let lower = k.to_ascii_lowercase();
            !STRIPPED_SUBREQUEST_HEADERS.contains(&lower.as_str())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Core batch execution service (testable with an in-memory Axum `Router`
/// without binding a TCP socket).
#[derive(Clone)]
pub struct BatchDispatcher {
    config: BatchConfig,
    rate_limiter: Arc<RateLimiter>,
    field_selection: Arc<FieldSelectionParameter>,
    admin_ip_whitelist: Option<Arc<IpWhitelistConfig>>,
    processed_count: Arc<RwLock<u64>>,
}

impl BatchDispatcher {
    #[must_use]
    pub fn new(
        config: BatchConfig,
        rate_limiter: Arc<RateLimiter>,
        field_selection: Arc<FieldSelectionParameter>,
        admin_ip_whitelist: Option<Arc<IpWhitelistConfig>>,
    ) -> Self {
        Self {
            config,
            rate_limiter,
            field_selection,
            admin_ip_whitelist,
            processed_count: Arc::new(RwLock::new(0)),
        }
    }

    #[must_use]
    pub const fn config(&self) -> &BatchConfig {
        &self.config
    }

    /// Validate batch size, sub-request methods/paths/bodies, anti-recursion,
    /// and aggregate cost cap. Pure logic, testable without HTTP.
    pub fn validate_batch(
        &self,
        requests: &[BatchSubRequest],
    ) -> Result<(Vec<ValidatedSubRequest>, u32), ApiError> {
        if requests.is_empty() {
            return Err(ApiError::bad_request(
                "EMPTY_BATCH",
                "Batch must contain at least one sub-request",
            ));
        }

        if requests.len() > self.config.max_batch_size {
            let mut details = HashMap::new();
            details.insert("requested".to_string(), serde_json::json!(requests.len()));
            details.insert(
                "max_batch_size".to_string(),
                serde_json::json!(self.config.max_batch_size),
            );
            return Err(ApiError::bad_request_with_details(
                "BATCH_TOO_LARGE",
                format!(
                    "Batch size {} exceeds maximum of {}",
                    requests.len(),
                    self.config.max_batch_size
                ),
                details,
            ));
        }

        let mut validated = Vec::with_capacity(requests.len());
        let mut total_cost: u32 = 0;

        for (index, req) in requests.iter().enumerate() {
            let method = validate_subrequest_method(&req.method)?;
            let (canonical_path, query) =
                validate_subrequest_path(&req.path, self.config.deny_nested_batch)?;

            let body_bytes = match &req.body {
                Some(v) => {
                    let bytes = serde_json::to_vec(v).map_err(|e| {
                        ApiError::bad_request(
                            "INVALID_SUBREQUEST_BODY",
                            format!("Sub-request {index} has unserializable body: {e}"),
                        )
                    })?;
                    if bytes.len() > self.config.max_subrequest_body_bytes {
                        return Err(ApiError::bad_request(
                            "SUBREQUEST_TOO_LARGE",
                            format!(
                                "Sub-request {index} body exceeds maximum of {} bytes",
                                self.config.max_subrequest_body_bytes
                            ),
                        ));
                    }
                    bytes
                }
                None => Vec::new(),
            };

            let cost = estimate_subrequest_cost(&method, &canonical_path);
            total_cost = total_cost.saturating_add(cost);

            let uri = match &query {
                Some(q) if !q.is_empty() => format!("{canonical_path}?{q}"),
                _ => canonical_path.clone(),
            };

            let fields_param = req
                .fields
                .clone()
                .or_else(|| extract_fields_query(query.as_deref()));

            validated.push(ValidatedSubRequest {
                index,
                method,
                canonical_path,
                query,
                uri,
                headers: sanitize_subrequest_headers(&req.headers),
                body_bytes,
                fields_param,
                cost,
            });
        }

        if total_cost > self.config.max_total_cost {
            let mut details = HashMap::new();
            details.insert("total_cost".to_string(), serde_json::json!(total_cost));
            details.insert(
                "max_total_cost".to_string(),
                serde_json::json!(self.config.max_total_cost),
            );
            return Err(ApiError::bad_request_with_details(
                "BATCH_COST_EXCEEDED",
                format!(
                    "Batch total cost {} exceeds maximum allowed cost of {}",
                    total_cost, self.config.max_total_cost
                ),
                details,
            ));
        }

        Ok((validated, total_cost))
    }

    /// Dispatch validated sub-requests through `router`, re-running rate
    /// limiting, IP whitelisting, authentication, authorization, and field
    /// selection for every single sub-request with the verified outer identity.
    pub async fn dispatch_router(
        &self,
        router: Router,
        identity: &BatchCallerIdentity,
        requests: Vec<BatchSubRequest>,
    ) -> ApiResult<BatchHttpResponse> {
        let (validated, total_cost) = self.validate_batch(&requests)?;
        let total = validated.len();

        let ip_str = identity.client_ip.to_string();
        let bearer_owned = identity.bearer_token().map(str::to_string);
        let auth_user_id = identity.auth_user.as_ref().map(|u| u.user_id.clone());
        let is_admin = identity.auth_user.as_ref().is_some_and(|u| u.is_admin);

        let client = self
            .rate_limiter
            .resolve_client_identifier(bearer_owned.clone(), auth_user_id, ip_str.clone())
            .await;

        let api_key_info = match bearer_owned.as_deref() {
            Some(token) => match self.rate_limiter.resolve_api_key_id(token).await {
                Some(key_id) => {
                    let limit = self
                        .rate_limiter
                        .get_api_key_limit_per_minute(&key_id)
                        .await;
                    Some((key_id, limit))
                }
                None => None,
            },
            None => None,
        };

        // Shared rate-limit bucket key for the batch caller: ensures a batch
        // of N items (even targeting N distinct URLs) always consumes N units.
        let batch_bucket = identity
            .outer_rate_limit_endpoint
            .as_deref()
            .unwrap_or("/api/v1/batch");
        let outer_already_debited_first = identity.outer_rate_limit_endpoint.is_some();

        let mut results = Vec::with_capacity(total);
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        for sub in validated {
            // 1. Per-sub-request rate limit debit.
            //    Index 0 is skipped on the outer batch bucket ONLY when outer
            //    `rate_limit_middleware` already charged 1 unit for this HTTP
            //    call, so total units charged across the batch == `total`.
            let should_debit_batch_bucket = !(sub.index == 0 && outer_already_debited_first);
            if should_debit_batch_bucket {
                let (allowed, _info) = self
                    .rate_limiter
                    .check_rate_limit_for_client(&client, batch_bucket, &ip_str)
                    .await;
                if !allowed {
                    failed += 1;
                    results.push(BatchSubResponse {
                        index: sub.index,
                        status: StatusCode::TOO_MANY_REQUESTS.as_u16(),
                        success: false,
                        cost: sub.cost,
                        body: None,
                        error: Some(ErrorDetail {
                            code: "RATE_LIMIT_EXCEEDED".to_string(),
                            message: "Rate limit exceeded for batch sub-request".to_string(),
                            details: None,
                            request_id: identity.request_id.clone(),
                            stack_trace: None,
                        }),
                    });
                    continue;
                }
            }

            // Per-API-key bucket check for every sub-request when authenticated
            // with an API key.
            if let Some((ref key_id, limit)) = api_key_info {
                let should_debit_key = !(sub.index == 0 && outer_already_debited_first);
                if should_debit_key {
                    let (allowed, _info) = self.rate_limiter.rate_limit_api_key(key_id, limit).await;
                    if !allowed {
                        failed += 1;
                        results.push(BatchSubResponse {
                            index: sub.index,
                            status: StatusCode::TOO_MANY_REQUESTS.as_u16(),
                            success: false,
                            cost: sub.cost,
                            body: None,
                            error: Some(ErrorDetail {
                                code: "API_KEY_RATE_LIMIT_EXCEEDED".to_string(),
                                message: "API key rate limit exceeded for batch sub-request"
                                    .to_string(),
                                details: None,
                                request_id: identity.request_id.clone(),
                                stack_trace: None,
                            }),
                        });
                        continue;
                    }
                }
            }

            // 2. Fail-closed admin IP whitelist check for any `/admin` sub-request.
            if is_admin_path(&sub.canonical_path) {
                if let Some(ref whitelist) = self.admin_ip_whitelist {
                    if !whitelist.is_allowed(&identity.client_ip) {
                        tracing::warn!(
                            client_ip = %identity.client_ip,
                            path = %sub.canonical_path,
                            "Batch sub-request blocked by admin IP whitelist"
                        );
                        failed += 1;
                        results.push(BatchSubResponse {
                            index: sub.index,
                            status: StatusCode::FORBIDDEN.as_u16(),
                            success: false,
                            cost: sub.cost,
                            body: None,
                            error: Some(ErrorDetail {
                                code: "ADMIN_IP_NOT_WHITELISTED".to_string(),
                                message: "Access denied: IP address not whitelisted".to_string(),
                                details: None,
                                request_id: identity.request_id.clone(),
                                stack_trace: None,
                            }),
                        });
                        continue;
                    }
                }
            }

            // 3. Validate sub-request field selection against caller role.
            let parsed_selection = match sub.fields_param.as_deref() {
                Some(raw_fields) => match self
                    .field_selection
                    .parse_and_authorize(raw_fields, is_admin)
                {
                    Ok(sel) => Some(sel),
                    Err(err) => {
                        let api_err = err.to_api_error();
                        let err_resp = api_err.to_error_response(identity.request_id.clone());
                        let status = match err {
                            crate::models::field_selection_parameter::FieldSelectionError::ForbiddenField(_)
                            | crate::models::field_selection_parameter::FieldSelectionError::AdminFieldRequired(_) => {
                                StatusCode::FORBIDDEN
                            }
                            _ => StatusCode::BAD_REQUEST,
                        };
                        failed += 1;
                        results.push(BatchSubResponse {
                            index: sub.index,
                            status: status.as_u16(),
                            success: false,
                            cost: sub.cost,
                            body: None,
                            error: Some(err_resp.error),
                        });
                        continue;
                    }
                },
                None => None,
            };

            // 4. Construct isolated sub-request with verified outer identity.
            let mut builder = Request::builder().method(sub.method.clone()).uri(&sub.uri);

            for (name, value) in &sub.headers {
                if let (Ok(hn), Ok(hv)) = (
                    HeaderName::from_bytes(name.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    builder = builder.header(hn, hv);
                }
            }

            if !sub.body_bytes.is_empty() {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
            }

            // Force outer verified Authorization header (never caller-supplied).
            if let Some(ref auth_hdr) = identity.authorization_header {
                if let Ok(hv) = HeaderValue::from_str(auth_hdr) {
                    builder = builder.header(header::AUTHORIZATION, hv);
                }
            }

            let mut http_req = match builder.body(Body::from(sub.body_bytes)) {
                Ok(r) => r,
                Err(e) => {
                    failed += 1;
                    results.push(BatchSubResponse {
                        index: sub.index,
                        status: StatusCode::BAD_REQUEST.as_u16(),
                        success: false,
                        cost: sub.cost,
                        body: None,
                        error: Some(ErrorDetail {
                            code: "INVALID_SUBREQUEST".to_string(),
                            message: format!("Failed to construct sub-request: {e}"),
                            details: None,
                            request_id: identity.request_id.clone(),
                            stack_trace: None,
                        }),
                    });
                    continue;
                }
            };

            // Propagate verified peer address for inner IP/rate-limit middleware.
            http_req
                .extensions_mut()
                .insert(ConnectInfo(identity.peer_addr));

            // 5. Execute through the full Axum Router (auth + RBAC + middleware).
            let response = match router.clone().oneshot(http_req).await {
                Ok(resp) => resp,
                Err(infallible) => match infallible {},
            };

            let status = response.status();
            let raw_body = to_bytes(response.into_body(), MAX_SUBRESPONSE_BYTES)
                .await
                .unwrap_or_default();

            let parsed_body = if raw_body.is_empty() {
                None
            } else {
                serde_json::from_slice::<serde_json::Value>(&raw_body)
                    .ok()
                    .or_else(|| {
                        Some(serde_json::Value::String(
                            String::from_utf8_lossy(&raw_body).into_owned(),
                        ))
                    })
            };

            if status.is_success() {
                succeeded += 1;
                let filtered_body = parsed_body.map(|val| {
                    self.field_selection
                        .filter_value(&val, parsed_selection.as_ref(), is_admin)
                });
                results.push(BatchSubResponse {
                    index: sub.index,
                    status: status.as_u16(),
                    success: true,
                    cost: sub.cost,
                    body: filtered_body,
                    error: None,
                });
            } else {
                failed += 1;
                let error_message = parsed_body
                    .as_ref()
                    .and_then(|v| {
                        v.get("error")
                            .and_then(|e| {
                                e.get("message")
                                    .and_then(|m| m.as_str())
                                    .or_else(|| e.as_str())
                            })
                            .or_else(|| v.get("message").and_then(|m| m.as_str()))
                    })
                    .unwrap_or_else(|| status.canonical_reason().unwrap_or("Sub-request failed"))
                    .to_string();

                results.push(BatchSubResponse {
                    index: sub.index,
                    status: status.as_u16(),
                    success: false,
                    cost: sub.cost,
                    body: parsed_body,
                    error: Some(ErrorDetail {
                        code: format!("SUBREQUEST_{}", status.as_u16()),
                        message: error_message,
                        details: None,
                        request_id: identity.request_id.clone(),
                        stack_trace: None,
                    }),
                });
            }
        }

        let mut count = self.processed_count.write().await;
        *count += total as u64;

        Ok(BatchHttpResponse {
            results,
            total,
            succeeded,
            failed,
            total_cost,
        })
    }

    pub async fn processed_count(&self) -> u64 {
        *self.processed_count.read().await
    }
}

/// Axum state for the thin `POST /batch` HTTP handler.
#[derive(Clone)]
pub struct BatchHandlerState {
    pub dispatcher: Arc<BatchDispatcher>,
    pub sub_router: Router,
}

/// Thin HTTP handler for `POST /api/v1/batch` (and `/api/batch`).
///
/// Extracts only verified outer request context and delegates all validation,
/// rate-limit accounting, middleware re-execution, and field selection to
/// [`BatchDispatcher`].
pub async fn batch_http_handler(
    State(state): State<BatchHandlerState>,
    headers: HeaderMap,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    auth_user: Option<Extension<AuthUser>>,
    jwt_secret: Option<Extension<JwtSecret>>,
    revocation_store: Option<Extension<TokenRevocationStore>>,
    request_id: Option<Extension<RequestId>>,
    outer_rate_limit: Option<Extension<OuterRateLimitCharged>>,
    Json(payload): Json<BatchHttpRequest>,
) -> ApiResult<Json<BatchHttpResponse>> {
    let resolved_user = match auth_user {
        Some(Extension(user)) => Some(user),
        None => match (jwt_secret, revocation_store) {
            (Some(Extension(secret)), Some(Extension(store))) => {
                resolve_optional_auth_user(&headers, &secret, &store).await
            }
            _ => None,
        },
    };

    let identity = BatchCallerIdentity::from_outer_request(
        &headers,
        connect_info.map(|Extension(c)| c.0),
        resolved_user,
        request_id.map(|Extension(id)| id.0),
        outer_rate_limit.map(|Extension(m)| m.endpoint),
    );

    let response = state
        .dispatcher
        .dispatch_router(state.sub_router.clone(), &identity, payload.requests)
        .await?;

    Ok(Json(response))
}

/// Generic in-process batch helper retained for callers processing typed slices.
#[derive(Clone)]
pub struct BatchEndpoints {
    config: BatchConfig,
    /// Running count of total items processed across all batches.
    state: Arc<RwLock<u64>>,
}

impl BatchEndpoints {
    #[must_use]
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(0)),
        }
    }

    /// Build a full HTTP [`BatchDispatcher`] sharing this instance's [`BatchConfig`].
    #[must_use]
    pub fn into_dispatcher(
        &self,
        rate_limiter: Arc<RateLimiter>,
        admin_ip_whitelist: Option<Arc<IpWhitelistConfig>>,
    ) -> BatchDispatcher {
        BatchDispatcher::new(
            self.config.clone(),
            rate_limiter,
            Arc::new(FieldSelectionParameter::new(FieldSelectionConfig::default())),
            admin_ip_whitelist,
        )
    }

    /// Process a batch of items using the provided per-item handler.
    ///
    /// Returns an error only for structural problems (e.g. batch too large).
    /// Individual item failures are captured inside [`BatchItemResult`].
    pub async fn process<T, U, F>(
        &self,
        context: &NetworkContext,
        request: BatchRequest<T>,
        mut handler: F,
    ) -> Result<BatchResponse<U>>
    where
        T: Serialize,
        U: Serialize,
        F: FnMut(usize, T) -> Result<U>,
    {
        let total = request.items.len();

        if total > self.config.max_batch_size {
            bail!(
                "Batch size {} exceeds maximum of {}",
                total,
                self.config.max_batch_size
            );
        }

        let mut results = Vec::with_capacity(total);
        let mut succeeded = 0usize;
        let mut failed = 0usize;

        for (index, item) in request.items.into_iter().enumerate() {
            match handler(index, item) {
                Ok(data) => {
                    succeeded += 1;
                    results.push(BatchItemResult {
                        index,
                        success: true,
                        data: Some(data),
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    tracing::warn!(
                        network = ?context.network,
                        index,
                        error = %e,
                        "Batch item failed"
                    );
                    results.push(BatchItemResult {
                        index,
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        let mut count = self.state.write().await;
        *count += total as u64;

        tracing::info!(
            network = ?context.network,
            total,
            succeeded,
            failed,
            total_processed = *count,
            "Batch processed"
        );

        Ok(BatchResponse {
            results,
            total,
            succeeded,
            failed,
        })
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn instance() -> BatchEndpoints {
        BatchEndpoints::new(BatchConfig::default())
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec![1u32, 2, 3],
        };
        let result = inst.process(&ctx, req, |_, v| Ok(v * 2)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.total, 3);
        assert_eq!(resp.succeeded, 3);
        assert_eq!(resp.failed, 0);
    }

    #[tokio::test]
    async fn test_mainnet_supported() {
        let inst = instance();
        let ctx = NetworkContext::mainnet();
        let req = BatchRequest {
            items: vec!["a", "b"],
        };
        let result = inst.process(&ctx, req, |_, v| Ok(v.to_uppercase())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_partial_failures_captured() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec![0u32, 1, 2],
        };
        let result = inst
            .process(&ctx, req, |_, v| {
                if v == 0 {
                    bail!("zero not allowed")
                } else {
                    Ok(v)
                }
            })
            .await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.total, 3);
        assert_eq!(resp.succeeded, 2);
        assert_eq!(resp.failed, 1);
        assert!(!resp.results[0].success);
        assert_eq!(resp.results[0].error.as_deref(), Some("zero not allowed"));
    }

    #[tokio::test]
    async fn test_exceeds_max_batch_size_returns_error() {
        let inst = BatchEndpoints::new(BatchConfig {
            max_batch_size: 2,
            ..Default::default()
        });
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec![1, 2, 3],
        };
        let result = inst.process(&ctx, req, |_, v: i32| Ok(v)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[tokio::test]
    async fn test_empty_batch_succeeds() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req: BatchRequest<i32> = BatchRequest { items: vec![] };
        let result = inst.process(&ctx, req, |_, v| Ok(v)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.total, 0);
        assert_eq!(resp.succeeded, 0);
        assert_eq!(resp.failed, 0);
    }

    #[tokio::test]
    async fn test_result_indices_are_correct() {
        let inst = instance();
        let ctx = NetworkContext::testnet();
        let req = BatchRequest {
            items: vec!["x", "y", "z"],
        };
        let resp = inst
            .process(&ctx, req, |_, v| Ok(v.to_string()))
            .await
            .unwrap();
        for (i, item) in resp.results.iter().enumerate() {
            assert_eq!(item.index, i);
        }
    }

    #[test]
    fn test_validate_subrequest_path_blocks_traversal_and_nested_batch() {
        assert!(validate_subrequest_path("/api/v1/corridors", true).is_ok());
        assert!(validate_subrequest_path("/api/v1/corridors/../../admin/audit-log", true).is_err());
        assert!(validate_subrequest_path("/api/v1/%2e%2e/admin/audit-log", true).is_err());
        assert!(validate_subrequest_path("http://169.254.169.254/latest/meta-data", true).is_err());
        assert!(validate_subrequest_path("//evil.example/admin", true).is_err());
        assert!(validate_subrequest_path("/api/v1/batch", true).is_err());
        assert!(validate_subrequest_path("/api/v1/preflight/batch", true).is_err());
    }

    #[test]
    fn test_sanitize_headers_strips_security_sensitive_headers() {
        let mut raw = HashMap::new();
        raw.insert("Authorization".to_string(), "Bearer spoofed".to_string());
        raw.insert("X-Forwarded-For".to_string(), "127.0.0.1".to_string());
        raw.insert("Cookie".to_string(), "session=evil".to_string());
        raw.insert("Accept".to_string(), "application/json".to_string());

        let sanitized = sanitize_subrequest_headers(&raw);
        assert!(!sanitized.contains_key("Authorization"));
        assert!(!sanitized.contains_key("X-Forwarded-For"));
        assert!(!sanitized.contains_key("Cookie"));
        assert_eq!(sanitized.get("Accept").map(String::as_str), Some("application/json"));
    }

    #[test]
    fn test_validate_batch_enforces_cost_cap() {
        let limiter = Arc::new(RateLimiter::new_memory_only(None));
        let dispatcher = BatchDispatcher::new(
            BatchConfig {
                max_batch_size: 20,
                max_total_cost: 6,
                ..Default::default()
            },
            limiter,
            Arc::new(FieldSelectionParameter::new(FieldSelectionConfig::default())),
            None,
        );

        // Two POST requests to /admin/backfill cost 7 each = 14 > 6.
        let reqs = vec![
            BatchSubRequest {
                method: "POST".to_string(),
                path: "/admin/backfill".to_string(),
                headers: HashMap::new(),
                body: None,
                fields: None,
            },
            BatchSubRequest {
                method: "POST".to_string(),
                path: "/admin/backfill".to_string(),
                headers: HashMap::new(),
                body: None,
                fields: None,
            },
        ];

        let err = dispatcher.validate_batch(&reqs).unwrap_err();
        assert_eq!(err.to_error_response(None).error.code, "BATCH_COST_EXCEEDED");
    }
}
