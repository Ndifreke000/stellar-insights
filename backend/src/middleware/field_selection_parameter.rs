use anyhow::Result;
use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth_middleware::AuthUser;
use crate::models::field_selection_parameter::{
    FieldAccessPolicy, FieldSelection, FieldSelectionConfig, FieldSelectionError,
};
use crate::models::network_context_middleware::NetworkContext;

/// Maximum JSON response body size buffered by `field_selection_middleware` (4 MiB).
const MAX_FILTERED_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

/// Middleware & service that parses, authorizes, and applies the `fields`
/// query parameter (#2832).
#[derive(Clone)]
pub struct FieldSelectionParameter {
    config: FieldSelectionConfig,
    /// Tracks the total number of field-selection requests processed.
    request_count: Arc<RwLock<u64>>,
}

impl FieldSelectionParameter {
    #[must_use]
    pub fn new(config: FieldSelectionConfig) -> Self {
        Self {
            config,
            request_count: Arc::new(RwLock::new(0)),
        }
    }

    #[must_use]
    pub const fn policy(&self) -> &FieldAccessPolicy {
        &self.config.access_policy
    }

    /// Parse and authorize the raw `fields` parameter for a non-admin caller
    /// in the given network context.
    pub async fn process(
        &self,
        raw_fields: &str,
        context: &NetworkContext,
    ) -> Result<FieldSelection, FieldSelectionError> {
        self.process_for_role(raw_fields, context, false).await
    }

    /// Parse and authorize the raw `fields` parameter using the caller's role
    /// (`is_admin`). Rejects forbidden secret fields for all callers and
    /// admin-only fields for non-admin callers.
    pub async fn process_for_role(
        &self,
        raw_fields: &str,
        context: &NetworkContext,
        is_admin: bool,
    ) -> Result<FieldSelection, FieldSelectionError> {
        let selection = self.parse_and_authorize(raw_fields, is_admin)?;

        let mut count = self.request_count.write().await;
        *count += 1;

        tracing::info!(
            network = ?context.network,
            fields = ?selection.fields,
            is_admin,
            total_requests = *count,
            "Field selection parameter processed"
        );

        Ok(selection)
    }

    /// Pure, synchronous parse + authorization check (usable without HTTP or async).
    pub fn parse_and_authorize(
        &self,
        raw_fields: &str,
        is_admin: bool,
    ) -> Result<FieldSelection, FieldSelectionError> {
        let selection = FieldSelection::parse(raw_fields, self.config.max_fields)?;
        self.config.access_policy.authorize(&selection, is_admin)?;
        Ok(selection)
    }

    /// Apply role-aware field projection and sensitive-field redaction to a JSON value.
    #[must_use]
    pub fn filter_value(
        &self,
        value: &serde_json::Value,
        selection: Option<&FieldSelection>,
        is_admin: bool,
    ) -> serde_json::Value {
        self.config
            .access_policy
            .filter_json(value, selection, is_admin)
    }

    /// Returns the total number of requests processed so far.
    pub async fn request_count(&self) -> u64 {
        *self.request_count.read().await
    }
}

/// Extract the `fields` query parameter value from a URI query string, if present.
#[must_use]
pub fn extract_fields_query(query: Option<&str>) -> Option<String> {
    let query = query?;
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == "fields")
        .map(|(_, v)| v.into_owned())
}

/// Axum middleware that validates `?fields=...` against [`FieldAccessPolicy`]
/// at the boundary and projects successful (2xx) JSON responses.
///
/// Security invariants (#2832):
/// - Non-2xx responses (401/403/400/429/5xx) are NEVER altered or masked.
/// - Forbidden fields (`password_hash`, `key_hash`, `secret`, etc.) are rejected
///   with `403 Forbidden` even for admins, and stripped from responses.
/// - Admin-only fields are rejected with `403 Forbidden` for non-admin callers.
/// - Caller role is read strictly from the verified `AuthUser` request extension
///   (never from query parameters or unverified headers).
pub async fn field_selection_middleware(
    State(service): State<Arc<FieldSelectionParameter>>,
    req: Request,
    next: Next,
) -> Response {
    let raw_fields = extract_fields_query(req.uri().query());
    let is_admin = req
        .extensions()
        .get::<AuthUser>()
        .is_some_and(|u| u.is_admin);

    let selection = match raw_fields {
        Some(raw) => match service.parse_and_authorize(&raw, is_admin) {
            Ok(sel) => Some(sel),
            Err(err) => return err.to_api_error().into_response(),
        },
        None => None,
    };

    let response = next.run(req).await;

    // Only project successful JSON responses; never alter auth/error responses.
    if !response.status().is_success() {
        return response;
    }

    let is_json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.contains("application/json"));

    if !is_json {
        return response;
    }

    // Only buffer & rewrite when either a field selection was requested or
    // sensitive/admin redaction is needed on a JSON payload.
    let Some(sel) = selection else {
        return response;
    };

    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, MAX_FILTERED_RESPONSE_BYTES).await else {
        return Response::from_parts(parts, Body::empty());
    };

    let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    let filtered = service.filter_value(&json_val, Some(&sel), is_admin);
    let Ok(out_bytes) = serde_json::to_vec(&filtered) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(out_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_context_middleware::NetworkContext;
    use axum::{http::StatusCode, middleware, routing::get, Json, Router};
    use serde_json::json;
    use tower::ServiceExt;

    fn middleware_service() -> FieldSelectionParameter {
        FieldSelectionParameter::new(FieldSelectionConfig::default())
    }

    #[tokio::test]
    async fn test_basic_functionality() {
        let instance = middleware_service();
        let result = instance
            .process("id,name,status", &NetworkContext::testnet())
            .await;
        assert!(result.is_ok());
        let sel = result.unwrap();
        assert!(sel.includes("id"));
        assert!(sel.includes("name"));
        assert!(!sel.includes("unknown_field"));
    }

    #[tokio::test]
    async fn test_mainnet_context() {
        let result = middleware_service()
            .process("volume,success_rate", &NetworkContext::mainnet())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_empty_field_list_returns_error() {
        let result = middleware_service()
            .process("", &NetworkContext::testnet())
            .await;
        assert!(matches!(result, Err(FieldSelectionError::EmptyFieldList)));
    }

    #[tokio::test]
    async fn test_invalid_field_name_returns_error() {
        let result = middleware_service()
            .process("id,bad-field!", &NetworkContext::testnet())
            .await;
        assert!(matches!(
            result,
            Err(FieldSelectionError::InvalidFieldName(_))
        ));
    }

    #[tokio::test]
    async fn test_too_many_fields_returns_error() {
        let config = FieldSelectionConfig {
            max_fields: 2,
            ..Default::default()
        };
        let instance = FieldSelectionParameter::new(config);
        let result = instance.process("a,b,c", &NetworkContext::testnet()).await;
        assert!(matches!(
            result,
            Err(FieldSelectionError::TooManyFields { .. })
        ));
    }

    #[tokio::test]
    async fn test_request_count_increments() {
        let instance = middleware_service();
        let ctx = NetworkContext::testnet();
        let _ = instance.process("id", &ctx).await;
        let _ = instance.process("name", &ctx).await;
        assert_eq!(instance.request_count().await, 2);
    }

    #[tokio::test]
    async fn test_whitespace_trimmed_from_fields() {
        let result = middleware_service()
            .process(" id , name ", &NetworkContext::testnet())
            .await;
        assert!(result.is_ok());
        let sel = result.unwrap();
        assert!(sel.includes("id"));
        assert!(sel.includes("name"));
    }

    #[tokio::test]
    async fn test_includes_empty_selection_matches_all() {
        let sel = FieldSelection::default();
        assert!(sel.includes("anything"));
    }

    #[tokio::test]
    async fn test_forbidden_secret_field_rejected_for_all_roles() {
        let svc = middleware_service();
        let ctx = NetworkContext::testnet();

        let err_user = svc.process_for_role("id,password_hash", &ctx, false).await;
        assert!(matches!(
            err_user,
            Err(FieldSelectionError::ForbiddenField(_))
        ));

        let err_admin = svc.process_for_role("id,key_hash", &ctx, true).await;
        assert!(matches!(
            err_admin,
            Err(FieldSelectionError::ForbiddenField(_))
        ));
    }

    #[tokio::test]
    async fn test_admin_only_field_rejected_for_non_admin_and_allowed_for_admin() {
        let svc = middleware_service();
        let ctx = NetworkContext::testnet();

        let err_user = svc.process_for_role("id,ip_address", &ctx, false).await;
        assert!(matches!(
            err_user,
            Err(FieldSelectionError::AdminFieldRequired(_))
        ));

        let ok_admin = svc.process_for_role("id,ip_address", &ctx, true).await;
        assert!(ok_admin.is_ok());
    }

    #[test]
    fn test_filter_json_strips_secrets_and_projects_envelope_items() {
        let svc = middleware_service();
        let sel = svc.parse_and_authorize("id,name", false).unwrap();
        let payload = json!({
            "data": [
                {
                    "id": "c1",
                    "name": "USDC-NGN",
                    "success_rate": 99.1,
                    "password_hash": "leaked",
                    "ip_address": "10.0.0.1"
                }
            ],
            "pagination": { "limit": 50, "has_next": false }
        });

        let filtered = svc.filter_value(&payload, Some(&sel), false);
        assert_eq!(filtered["data"][0]["id"], "c1");
        assert_eq!(filtered["data"][0]["name"], "USDC-NGN");
        assert!(filtered["data"][0].get("success_rate").is_none());
        assert!(filtered["data"][0].get("password_hash").is_none());
        assert!(filtered["data"][0].get("ip_address").is_none());
        assert_eq!(filtered["pagination"]["limit"], 50);
    }

    #[tokio::test]
    async fn test_middleware_rejects_forbidden_field_and_preserves_403_errors() {
        let svc = Arc::new(middleware_service());
        let app = Router::new()
            .route(
                "/item",
                get(|| async {
                    Json(json!({
                        "id": "1",
                        "name": "corridor",
                        "password_hash": "secret",
                        "extra": 42
                    }))
                }),
            )
            .route(
                "/forbidden",
                get(|| async { (StatusCode::FORBIDDEN, Json(json!({ "error": "denied" }))) }),
            )
            .layer(middleware::from_fn_with_state(
                svc,
                field_selection_middleware,
            ));

        // Allowed fields project only those fields and strip password_hash.
        let ok_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/item?fields=id,name")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok_res.status(), StatusCode::OK);
        let body = to_bytes(ok_res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed, json!({ "id": "1", "name": "corridor" }));

        // Requesting forbidden field returns 403 standard error shape.
        let secret_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/item?fields=id,password_hash")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(secret_res.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(secret_res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["error"]["code"], "FORBIDDEN_FIELD_SELECTION");

        // Non-2xx response is never altered by field selection.
        let err_res = app
            .oneshot(
                Request::builder()
                    .uri("/forbidden?fields=id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(err_res.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(err_res.into_body(), usize::MAX).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["error"], "denied");
    }
}
