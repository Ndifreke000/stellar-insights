//! Consolidated API versioning and deprecation middleware (Issue #2833).
//!
//! Replaces the four legacy modules (`middleware/api_versioning.rs`,
//! `api_v1_middleware.rs`, `deprecation_middleware.rs`, and
//! `api_deprecation_middleware.rs`) with a single policy driven by a route
//! metadata table.
//!
//! # Policy summary
//!
//! 1. **URL versioning takes precedence** over `Accept` header negotiation.
//!    - `/api/v2/*` → `v2` (`X-API-Status: preview`, no deprecation headers)
//!    - `/api/v1/*` → `v1` (`X-API-Status: deprecated`, RFC 9745 + RFC 8594 headers)
//!    - `/api/v<N>/*` for unsupported `N` → `400 Bad Request` (`UNSUPPORTED_API_VERSION`)
//! 2. **Accept header negotiation** applies to unversioned `/api/*` routes:
//!    - `Accept: application/vnd.payraider.v2+json` → `v2` (`preview`)
//!    - `Accept: application/vnd.payraider.v1+json` → `v1` (`deprecated`)
//!    - Unsupported vendor version → `400 Bad Request` (`UNSUPPORTED_API_VERSION`)
//!    - Absent vendor media type → defaults to `v1`
//! 3. **Route metadata table** uses longest boundary-aware prefix match,
//!    allowing per-route deprecation schedules and exempting operational routes
//!    (`/health`, `/livez`, `/readyz`, `/api/version`, `/api/docs`, `/ws`, `/graphql`).
//! 4. **Standard deprecation headers**:
//!    - `Deprecation`: RFC 9745 Structured Field Date (`@<unix_epoch_seconds>`)
//!    - `Sunset`: RFC 8594 HTTP-date (IMF-fixdate)
//!    - `Link`: RFC 8288 `rel="successor-version"` and `rel="deprecation"`
//!    - `Warning`: `299 - "..."` fallback notice for legacy clients

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{ApiError, ErrorDetail, ErrorResponse};
use crate::models::api_versioning::{
    ApiVersionId, DeprecationNotice, RouteVersionMetadata, VersionLifecycle, VersioningConfig,
    V1_DEPRECATION_ISO8601, V1_SUNSET_ISO8601,
};
use crate::models::network_context_middleware::NetworkContext;

/// Vendor media-type prefix used for `Accept` header version negotiation.
pub const VENDOR_MEDIA_PREFIX: &str = "application/vnd.payraider.v";

/// Resolved version & deprecation decision for a single request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRouteVersion {
    pub version: Option<ApiVersionId>,
    pub lifecycle: VersionLifecycle,
    pub negotiated_via_accept: bool,
    pub deprecation: Option<DeprecationNotice>,
}

/// Errors raised while resolving an API version or enforcing a sunset deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersioningRejection {
    UnsupportedVersion {
        requested: u32,
        supported: Vec<&'static str>,
    },
    EndpointSunset {
        path: String,
        sunset_http_date: String,
        successor_uri: Option<String>,
    },
}

impl VersioningRejection {
    /// Convert into the project's standard [`ErrorResponse`] HTTP response.
    #[must_use]
    pub fn into_http_response(self, request_id: Option<String>) -> Response {
        match self {
            Self::UnsupportedVersion {
                requested,
                supported,
            } => {
                let mut details = HashMap::new();
                details.insert(
                    "requested_version".to_string(),
                    serde_json::Value::String(format!("v{requested}")),
                );
                details.insert(
                    "supported_versions".to_string(),
                    serde_json::json!(supported),
                );
                let api_error = ApiError::bad_request_with_details(
                    "UNSUPPORTED_API_VERSION",
                    format!("API version v{requested} is not supported"),
                    details,
                );
                let body = api_error.to_error_response(request_id);
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            Self::EndpointSunset {
                path,
                sunset_http_date,
                successor_uri,
            } => {
                let mut details = HashMap::new();
                details.insert("path".to_string(), serde_json::Value::String(path));
                details.insert(
                    "sunset".to_string(),
                    serde_json::Value::String(sunset_http_date.clone()),
                );
                if let Some(ref successor) = successor_uri {
                    details.insert(
                        "successor".to_string(),
                        serde_json::Value::String(successor.clone()),
                    );
                }
                let body = ErrorResponse {
                    error: ErrorDetail {
                        code: "ENDPOINT_SUNSET".to_string(),
                        message: format!(
                            "This API endpoint reached its sunset date ({sunset_http_date}) and is no longer available"
                        ),
                        details: Some(details),
                        request_id,
                        stack_trace: None,
                    },
                };
                let mut response = (StatusCode::GONE, Json(body)).into_response();
                if let Ok(v) = HeaderValue::from_str(&sunset_http_date) {
                    response.headers_mut().insert("Sunset", v);
                }
                if let Some(successor) = successor_uri {
                    if let Ok(v) =
                        HeaderValue::from_str(&format!("<{successor}>; rel=\"successor-version\""))
                    {
                        response.headers_mut().append(header::LINK, v);
                    }
                }
                response
            }
        }
    }
}

/// JSON payload served by `GET /api/version`, derived directly from [`ApiVersionPolicy`].
#[derive(Debug, Clone, Serialize)]
pub struct ApiVersionSummary {
    pub current: String,
    pub supported: Vec<String>,
    pub deprecated: Vec<String>,
    pub deprecation_dates: HashMap<String, String>,
    pub sunset_dates: HashMap<String, String>,
}

/// Pure, HTTP-independent versioning policy engine driven by a route metadata table.
#[derive(Debug, Clone)]
pub struct ApiVersionPolicy {
    config: VersioningConfig,
    /// Routes sorted by descending prefix length so the first boundary-aware
    /// match is always the most specific rule.
    sorted_routes: Vec<RouteVersionMetadata>,
}

impl Default for ApiVersionPolicy {
    fn default() -> Self {
        Self::new(VersioningConfig::default())
    }
}

impl ApiVersionPolicy {
    #[must_use]
    pub fn new(config: VersioningConfig) -> Self {
        let mut sorted_routes = config.routes.clone();
        sorted_routes.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));
        Self {
            config,
            sorted_routes,
        }
    }

    /// Add or override a route metadata entry and keep longest-prefix ordering.
    #[must_use]
    pub fn with_route(mut self, entry: RouteVersionMetadata) -> Self {
        self.sorted_routes
            .retain(|existing| existing.path_prefix != entry.path_prefix);
        self.sorted_routes.push(entry.clone());
        self.sorted_routes
            .sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));
        self.config
            .routes
            .retain(|existing| existing.path_prefix != entry.path_prefix);
        self.config.routes.push(entry);
        self
    }

    /// Return the underlying configuration.
    #[must_use]
    pub const fn config(&self) -> &VersioningConfig {
        &self.config
    }

    /// Build the `/api/version` response payload from the active policy table
    /// so discovery metadata can never drift from runtime middleware behavior.
    #[must_use]
    pub fn version_summary(&self) -> ApiVersionSummary {
        let supported: Vec<String> = self
            .config
            .supported_versions
            .iter()
            .map(|v| v.as_str().to_string())
            .collect();

        let mut deprecated = Vec::new();
        let mut deprecation_dates = HashMap::new();
        let mut sunset_dates = HashMap::new();

        for route in &self.config.routes {
            if route.path_prefix == "/api/v1"
                && matches!(
                    route.lifecycle,
                    VersionLifecycle::Deprecated | VersionLifecycle::Sunset
                )
            {
                if !deprecated.iter().any(|v| v == "v1") {
                    deprecated.push("v1".to_string());
                }
                deprecation_dates.insert("v1".to_string(), V1_DEPRECATION_ISO8601.to_string());
                sunset_dates.insert("v1".to_string(), V1_SUNSET_ISO8601.to_string());
            }
        }

        ApiVersionSummary {
            current: format!("v{}", self.config.current_version),
            supported,
            deprecated,
            deprecation_dates,
            sunset_dates,
        }
    }

    /// Resolve version and deprecation metadata for `(path, accept)` at the current UTC time.
    pub fn resolve(
        &self,
        path: &str,
        accept: Option<&str>,
    ) -> Result<ResolvedRouteVersion, VersioningRejection> {
        let now_unix = chrono::Utc::now().timestamp();
        self.resolve_at(path, accept, now_unix)
    }

    /// Deterministic variant of [`Self::resolve`] accepting an explicit Unix timestamp
    /// for unit testing sunset deadlines without wall-clock flakiness.
    pub fn resolve_at(
        &self,
        path: &str,
        accept: Option<&str>,
        now_unix: i64,
    ) -> Result<ResolvedRouteVersion, VersioningRejection> {
        let supported_labels: Vec<&'static str> = self
            .config
            .supported_versions
            .iter()
            .map(|v| v.as_str())
            .collect();

        // 1. Validate explicit `/api/v<N>` URL prefix before falling back to `/api`.
        if let Some(url_ver) = extract_url_version_number(path) {
            if !self.is_supported_version_number(url_ver) {
                return Err(VersioningRejection::UnsupportedVersion {
                    requested: url_ver,
                    supported: supported_labels,
                });
            }
        }

        // 2. Validate vendor `Accept` media type whenever present so clients
        //    requesting an unsupported version always receive a predictable 400.
        let negotiated_accept_ver = match accept.and_then(extract_accept_version_number) {
            Some(ver_num) => {
                let Some(version_id) = ApiVersionId::from_number(ver_num)
                    .filter(|id| self.config.supported_versions.contains(id))
                else {
                    return Err(VersioningRejection::UnsupportedVersion {
                        requested: ver_num,
                        supported: supported_labels,
                    });
                };
                Some(version_id)
            }
            None => None,
        };

        // 3. Longest boundary-aware prefix lookup in the route metadata table.
        let matched = self
            .sorted_routes
            .iter()
            .find(|entry| path_matches_prefix(path, &entry.path_prefix));

        let resolved = match matched {
            Some(entry) => {
                if entry.allow_accept_negotiation {
                    if let Some(negotiated) = negotiated_accept_ver {
                        self.resolved_for_version(negotiated, true)
                    } else {
                        ResolvedRouteVersion {
                            version: entry.version,
                            lifecycle: entry.lifecycle,
                            negotiated_via_accept: true,
                            deprecation: entry.deprecation.clone(),
                        }
                    }
                } else {
                    ResolvedRouteVersion {
                        version: entry.version,
                        lifecycle: entry.lifecycle,
                        negotiated_via_accept: false,
                        deprecation: entry.deprecation.clone(),
                    }
                }
            }
            None => {
                // Non-`/api` root paths (legacy bare aliases mounted directly on root):
                // negotiate via Accept if supplied, otherwise default to v1.
                if let Some(negotiated) = negotiated_accept_ver {
                    self.resolved_for_version(negotiated, true)
                } else {
                    self.resolved_for_version(ApiVersionId::V1, true)
                }
            }
        };

        // 4. Enforce sunset deadline if configured for this route.
        if let Some(ref notice) = resolved.deprecation {
            if notice.enforce_sunset {
                if let Some(sunset_unix) = notice.sunset_at_unix {
                    if now_unix >= sunset_unix {
                        return Err(VersioningRejection::EndpointSunset {
                            path: path.to_string(),
                            sunset_http_date: notice.sunset_http_date.clone(),
                            successor_uri: notice.successor_uri.clone(),
                        });
                    }
                }
            }
        }

        Ok(resolved)
    }

    /// Attach standardized versioning and RFC 9745 / RFC 8594 deprecation headers
    /// to an outgoing response header map.
    pub fn apply_headers(headers: &mut HeaderMap, resolved: &ResolvedRouteVersion, path: &str) {
        if resolved.lifecycle == VersionLifecycle::Unversioned {
            return;
        }

        if let Some(version) = resolved.version {
            headers.insert("X-API-Version", HeaderValue::from_static(version.as_str()));
        }
        headers.insert(
            "X-API-Status",
            HeaderValue::from_static(resolved.lifecycle.as_header_value()),
        );

        if resolved.negotiated_via_accept {
            append_vary_accept(headers);
        }

        if matches!(
            resolved.lifecycle,
            VersionLifecycle::Deprecated | VersionLifecycle::Sunset
        ) {
            if let Some(ref notice) = resolved.deprecation {
                // RFC 9745: Deprecation header uses Structured Field Date `@<unix_seconds>`.
                if let Ok(v) = HeaderValue::from_str(&notice.rfc9745_header_value()) {
                    headers.insert("Deprecation", v);
                }

                // RFC 8594: Sunset header uses IMF-fixdate HTTP-date.
                if let Ok(v) = HeaderValue::from_str(&notice.sunset_http_date) {
                    headers.insert("Sunset", v);
                }

                // RFC 8288: Link relations for migration target and policy docs.
                let mut link_parts = Vec::new();
                if let Some(ref successor) = notice.successor_uri {
                    link_parts.push(format!("<{successor}>; rel=\"successor-version\""));
                }
                if let Some(ref docs) = notice.docs_uri {
                    link_parts.push(format!("<{docs}>; rel=\"deprecation\""));
                }
                if !link_parts.is_empty() {
                    if let Ok(v) = HeaderValue::from_str(&link_parts.join(", ")) {
                        headers.append(header::LINK, v);
                    }
                }

                // Legacy human-readable fallback notice.
                if let Some(ref warning) = notice.warning_message {
                    if let Ok(v) = HeaderValue::from_str(&format!("299 - \"{warning}\"")) {
                        headers.insert(header::WARNING, v);
                    }
                }

                tracing::warn!(
                    path = %path,
                    api_version = resolved.version.map_or("unversioned", ApiVersionId::as_str),
                    deprecation = %notice.rfc9745_header_value(),
                    sunset = %notice.sunset_http_date,
                    "Deprecated API endpoint accessed"
                );
            }
        }
    }

    fn is_supported_version_number(&self, num: u32) -> bool {
        ApiVersionId::from_number(num)
            .is_some_and(|id| self.config.supported_versions.contains(&id))
    }

    fn resolved_for_version(
        &self,
        version: ApiVersionId,
        negotiated_via_accept: bool,
    ) -> ResolvedRouteVersion {
        let prefix = format!("/api/{}", version.as_str());
        if let Some(entry) = self.sorted_routes.iter().find(|r| r.path_prefix == prefix) {
            return ResolvedRouteVersion {
                version: entry.version,
                lifecycle: entry.lifecycle,
                negotiated_via_accept,
                deprecation: entry.deprecation.clone(),
            };
        }

        let lifecycle = match version {
            ApiVersionId::V1 => VersionLifecycle::Deprecated,
            ApiVersionId::V2 => VersionLifecycle::Preview,
        };
        ResolvedRouteVersion {
            version: Some(version),
            lifecycle,
            negotiated_via_accept,
            deprecation: None,
        }
    }
}

/// Extract `<N>` from a URL starting with `/api/v<N>` followed by end-of-path or `/`.
fn extract_url_version_number(path: &str) -> Option<u32> {
    let rest = path.strip_prefix("/api/v")?;
    let digits_end = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits_end == 0 {
        return None;
    }
    let after = &rest[digits_end..];
    if !after.is_empty() && !after.starts_with('/') {
        return None;
    }
    rest[..digits_end].parse::<u32>().ok()
}

/// Extract `<N>` from an `Accept` header containing `application/vnd.payraider.v<N>`.
fn extract_accept_version_number(accept: &str) -> Option<u32> {
    let pos = accept.find(VENDOR_MEDIA_PREFIX)?;
    let rest = &accept[pos + VENDOR_MEDIA_PREFIX.len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// Boundary-aware prefix matching: `"/api/v1"` matches `"/api/v1"` and
/// `"/api/v1/corridors"`, but NOT `"/api/v10/corridors"`.
fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    let normalized_prefix = prefix.trim_end_matches('/');
    if normalized_prefix.is_empty() {
        return path.starts_with('/');
    }
    if path == normalized_prefix {
        return true;
    }
    path.strip_prefix(normalized_prefix)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Append `Accept` to the `Vary` header without losing any existing tokens.
fn append_vary_accept(headers: &mut HeaderMap) {
    let already_varies_on_accept = headers
        .get_all(header::VARY)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|val| {
            val.split(',')
                .any(|tok| tok.trim().eq_ignore_ascii_case("accept"))
        });

    if !already_varies_on_accept {
        headers.append(header::VARY, HeaderValue::from_static("Accept"));
    }
}

/// Thin Axum middleware executing the consolidated [`ApiVersionPolicy`].
pub async fn api_versioning_middleware(
    State(policy): State<Arc<ApiVersionPolicy>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let accept = request
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let request_id = request
        .extensions()
        .get::<crate::request_id::RequestId>()
        .map(|id| id.0.clone());

    let resolved = match policy.resolve(&path, accept.as_deref()) {
        Ok(resolved) => resolved,
        Err(rejection) => return rejection.into_http_response(request_id),
    };

    let mut response = next.run(request).await;
    ApiVersionPolicy::apply_headers(response.headers_mut(), &resolved, &path);
    response
}

/// Convenience wrapper retained for callers constructing the middleware via
/// [`VersioningConfig`] (also exposes request metrics for diagnostics).
#[derive(Clone)]
pub struct ApiVersioning {
    policy: Arc<ApiVersionPolicy>,
    request_count: Arc<RwLock<u64>>,
}

impl ApiVersioning {
    #[must_use]
    pub fn new(config: VersioningConfig) -> Self {
        Self {
            policy: Arc::new(ApiVersionPolicy::new(config)),
            request_count: Arc::new(RwLock::new(0)),
        }
    }

    #[must_use]
    pub fn policy(&self) -> Arc<ApiVersionPolicy> {
        Arc::clone(&self.policy)
    }

    pub async fn process(&self, context: &NetworkContext) -> anyhow::Result<()> {
        let mut count = self.request_count.write().await;
        *count += 1;
        tracing::info!(
            network = ?context.network,
            current_version = self.policy.config().current_version,
            min_supported_version = self.policy.config().min_supported_version,
            total_requests = *count,
            "API versioning processed"
        );
        Ok(())
    }

    pub async fn request_count(&self) -> u64 {
        *self.request_count.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::api_versioning::{V1_DEPRECATION_UNIX, V1_SUNSET_HTTP_DATE, V1_SUNSET_UNIX};
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    fn test_router(policy: Arc<ApiVersionPolicy>) -> Router {
        Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/livez", get(|| async { "ok" }))
            .route("/api/version", get(|| async { "ok" }))
            .route("/api/corridors", get(|| async { "ok" }))
            .route("/api/v1/corridors", get(|| async { "ok" }))
            .route("/api/v1/legacy/report", get(|| async { "ok" }))
            .route("/api/v2/status", get(|| async { "ok" }))
            .route("/api/v2/corridors", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                policy,
                api_versioning_middleware,
            ))
    }

    // ── Pure policy tests (no HTTP) ───────────────────────────────────────────

    #[test]
    fn resolves_v1_url_with_rfc9745_and_rfc8594_metadata() {
        let policy = ApiVersionPolicy::default();
        let resolved = policy
            .resolve_at("/api/v1/corridors", None, V1_DEPRECATION_UNIX + 10)
            .expect("v1 should resolve");

        assert_eq!(resolved.version, Some(ApiVersionId::V1));
        assert_eq!(resolved.lifecycle, VersionLifecycle::Deprecated);
        assert!(!resolved.negotiated_via_accept);
        let notice = resolved.deprecation.expect("deprecation notice present");
        assert_eq!(
            notice.rfc9745_header_value(),
            format!("@{V1_DEPRECATION_UNIX}")
        );
        assert_eq!(notice.sunset_http_date, V1_SUNSET_HTTP_DATE);
    }

    #[test]
    fn resolves_v2_url_as_preview_without_deprecation() {
        let policy = ApiVersionPolicy::default();
        let resolved = policy
            .resolve_at("/api/v2/corridors", None, V1_DEPRECATION_UNIX)
            .expect("v2 should resolve");

        assert_eq!(resolved.version, Some(ApiVersionId::V2));
        assert_eq!(resolved.lifecycle, VersionLifecycle::Preview);
        assert!(resolved.deprecation.is_none());
    }

    #[test]
    fn url_version_takes_precedence_over_accept_header() {
        let policy = ApiVersionPolicy::default();
        let resolved = policy
            .resolve_at(
                "/api/v1/corridors",
                Some("application/vnd.payraider.v2+json"),
                V1_DEPRECATION_UNIX,
            )
            .expect("explicit URL prefix wins");

        assert_eq!(resolved.version, Some(ApiVersionId::V1));
        assert_eq!(resolved.lifecycle, VersionLifecycle::Deprecated);
    }

    #[test]
    fn rejects_unsupported_url_version() {
        let policy = ApiVersionPolicy::default();
        let err = policy
            .resolve_at("/api/v99/corridors", None, V1_DEPRECATION_UNIX)
            .expect_err("v99 must be rejected");
        assert_eq!(
            err,
            VersioningRejection::UnsupportedVersion {
                requested: 99,
                supported: vec!["v1", "v2"],
            }
        );
    }

    #[test]
    fn boundary_prefix_does_not_confuse_v1_with_v10() {
        let policy = ApiVersionPolicy::default();
        let err = policy
            .resolve_at("/api/v10/corridors", None, V1_DEPRECATION_UNIX)
            .expect_err("v10 must not match /api/v1");
        assert!(matches!(
            err,
            VersioningRejection::UnsupportedVersion { requested: 10, .. }
        ));
    }

    #[test]
    fn enforces_sunset_deadline_when_enabled() {
        let policy = ApiVersionPolicy::default().with_route(RouteVersionMetadata {
            path_prefix: "/api/v1/legacy/report".to_string(),
            version: Some(ApiVersionId::V1),
            lifecycle: VersionLifecycle::Sunset,
            allow_accept_negotiation: false,
            deprecation: Some(DeprecationNotice {
                deprecated_at_unix: 1_700_000_000,
                sunset_http_date: "Thu, 01 Jan 2026 00:00:00 GMT".to_string(),
                sunset_at_unix: Some(1_767_225_600),
                successor_uri: Some("/api/v2/reports".to_string()),
                docs_uri: Some("/api/docs".to_string()),
                warning_message: None,
                enforce_sunset: true,
            }),
        });

        let before = policy.resolve_at("/api/v1/legacy/report", None, 1_767_225_599);
        assert!(before.is_ok());

        let after = policy.resolve_at("/api/v1/legacy/report", None, 1_767_225_600);
        assert!(matches!(
            after,
            Err(VersioningRejection::EndpointSunset { .. })
        ));
    }

    #[test]
    fn version_summary_matches_policy_table() {
        let policy = ApiVersionPolicy::default();
        let summary = policy.version_summary();
        assert_eq!(summary.current, "v1");
        assert_eq!(summary.supported, vec!["v1", "v2"]);
        assert_eq!(summary.deprecated, vec!["v1"]);
        assert!(summary.sunset_dates.contains_key("v1"));
        assert!(summary.deprecation_dates.contains_key("v1"));
    }

    // ── HTTP middleware header tests ──────────────────────────────────────────

    #[tokio::test]
    async fn deprecated_v1_route_emits_rfc9745_and_rfc8594_headers() {
        let app = test_router(Arc::new(ApiVersionPolicy::default()));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/corridors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("X-API-Version").unwrap(), "v1");
        assert_eq!(res.headers().get("X-API-Status").unwrap(), "deprecated");
        assert_eq!(
            res.headers().get("Deprecation").unwrap(),
            &format!("@{V1_DEPRECATION_UNIX}")
        );
        assert_eq!(res.headers().get("Sunset").unwrap(), V1_SUNSET_HTTP_DATE);
        let link = res.headers().get(header::LINK).unwrap().to_str().unwrap();
        assert!(link.contains("rel=\"successor-version\""));
        assert!(link.contains("rel=\"deprecation\""));
        assert!(res.headers().contains_key(header::WARNING));
        // Regression guard: Deprecation and Sunset headers must never be duplicated.
        assert_eq!(res.headers().get_all("Deprecation").iter().count(), 1);
        assert_eq!(res.headers().get_all("Sunset").iter().count(), 1);
    }

    #[tokio::test]
    async fn current_v2_route_and_accept_negotiation_omit_deprecation_headers() {
        let app = test_router(Arc::new(ApiVersionPolicy::default()));

        let res_url = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v2/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_url.status(), StatusCode::OK);
        assert_eq!(res_url.headers().get("X-API-Version").unwrap(), "v2");
        assert_eq!(res_url.headers().get("X-API-Status").unwrap(), "preview");
        assert!(!res_url.headers().contains_key("Deprecation"));
        assert!(!res_url.headers().contains_key("Sunset"));

        let res_accept = app
            .oneshot(
                Request::builder()
                    .uri("/api/corridors")
                    .header(header::ACCEPT, "application/vnd.payraider.v2+json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res_accept.status(), StatusCode::OK);
        assert_eq!(res_accept.headers().get("X-API-Version").unwrap(), "v2");
        assert_eq!(res_accept.headers().get("X-API-Status").unwrap(), "preview");
        assert_eq!(res_accept.headers().get(header::VARY).unwrap(), "Accept");
        assert!(!res_accept.headers().contains_key("Deprecation"));
        assert!(!res_accept.headers().contains_key("Sunset"));
    }

    #[tokio::test]
    async fn operational_routes_are_exempt_from_deprecation_headers() {
        let app = test_router(Arc::new(ApiVersionPolicy::default()));
        for path in ["/health", "/livez", "/api/version"] {
            let res = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(res.status(), StatusCode::OK, "path {path}");
            assert!(
                !res.headers().contains_key("Deprecation"),
                "{path} must not emit Deprecation header"
            );
            assert!(
                !res.headers().contains_key("Sunset"),
                "{path} must not emit Sunset header"
            );
        }
    }

    #[tokio::test]
    async fn route_metadata_override_wins_by_longest_prefix() {
        let policy = ApiVersionPolicy::default().with_route(RouteVersionMetadata {
            path_prefix: "/api/v1/legacy/report".to_string(),
            version: Some(ApiVersionId::V1),
            lifecycle: VersionLifecycle::Deprecated,
            allow_accept_negotiation: false,
            deprecation: Some(DeprecationNotice {
                deprecated_at_unix: 1_750_000_000,
                sunset_http_date: "Wed, 01 Jul 2026 00:00:00 GMT".to_string(),
                sunset_at_unix: Some(1_782_864_000),
                successor_uri: Some("/api/v2/reports".to_string()),
                docs_uri: Some("/api/docs#reports".to_string()),
                warning_message: Some("Legacy report endpoint deprecated".to_string()),
                enforce_sunset: false,
            }),
        });
        let app = test_router(Arc::new(policy));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/legacy/report")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("Deprecation").unwrap(), "@1750000000");
        assert_eq!(
            res.headers().get("Sunset").unwrap(),
            "Wed, 01 Jul 2026 00:00:00 GMT"
        );
        let link = res.headers().get(header::LINK).unwrap().to_str().unwrap();
        assert!(link.contains("</api/v2/reports>; rel=\"successor-version\""));
    }

    #[tokio::test]
    async fn unsupported_version_returns_standard_error_shape() {
        let app = test_router(Arc::new(ApiVersionPolicy::default()));
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/corridors")
                    .header(header::ACCEPT, "application/vnd.payraider.v3+json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "UNSUPPORTED_API_VERSION");
        assert_eq!(
            json["error"]["details"]["supported_versions"],
            serde_json::json!(["v1", "v2"])
        );
    }

    #[tokio::test]
    async fn past_sunset_enforced_route_returns_410_gone() {
        let policy = ApiVersionPolicy::default().with_route(RouteVersionMetadata {
            path_prefix: "/api/v1/legacy/report".to_string(),
            version: Some(ApiVersionId::V1),
            lifecycle: VersionLifecycle::Sunset,
            allow_accept_negotiation: false,
            deprecation: Some(DeprecationNotice {
                deprecated_at_unix: 1_600_000_000,
                sunset_http_date: "Thu, 01 Jan 2020 00:00:00 GMT".to_string(),
                sunset_at_unix: Some(1_577_836_800),
                successor_uri: Some("/api/v2/reports".to_string()),
                docs_uri: None,
                warning_message: None,
                enforce_sunset: true,
            }),
        });
        let app = test_router(Arc::new(policy));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/legacy/report")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::GONE);
        assert_eq!(
            res.headers().get("Sunset").unwrap(),
            "Thu, 01 Jan 2020 00:00:00 GMT"
        );
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "ENDPOINT_SUNSET");
    }

    #[test]
    fn v1_sunset_constant_matches_rfc8594_date() {
        let dt = chrono::DateTime::from_timestamp(V1_SUNSET_UNIX, 0).unwrap();
        assert_eq!(
            dt.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-12-31T00:00:00Z"
        );
    }
}
