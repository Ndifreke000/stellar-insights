//! Security and API versioning integration tests for Issues #2832 and #2833.
//!
//! Verifies:
//! - Non-admin batch containing an admin route returns sub-request 403 (never bypasses auth/RBAC)
//! - Sub-request Authorization / X-Forwarded-For spoofing is stripped and ignored
//! - Admin IP whitelist is enforced per sub-request using the real outer peer IP
//! - A batch of 100 counts as 100 against the rate limiter (both remaining quota and 429 cutoff)
//! - Batch size, aggregate cost, path traversal, and recursive batch caps
//! - Role-aware field selection rejects forbidden & admin-only fields and projects 2xx payloads
//! - Consolidated API versioning middleware emits RFC 9745 / RFC 8594 headers on deprecated
//!   routes and omits them on current/v2/operational routes without duplication.

use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tower::ServiceExt;

use payraider_backend::admin_ip_whitelist::IpWhitelistService;
use payraider_backend::api::admin_ip_whitelist;
use payraider_backend::auth::Claims;
use payraider_backend::auth_middleware::{auth_middleware, JwtSecret, TokenRevocationStore};
use payraider_backend::ip_whitelist_middleware::IpWhitelistConfig;
use payraider_backend::middleware::api_versioning::{api_versioning_middleware, ApiVersionPolicy};
use payraider_backend::middleware::batch_endpoints::{
    batch_http_handler, BatchCallerIdentity, BatchDispatcher, BatchHandlerState,
};
use payraider_backend::middleware::field_selection_parameter::{
    field_selection_middleware, FieldSelectionParameter,
};
use payraider_backend::models::api_versioning::{V1_DEPRECATION_UNIX, V1_SUNSET_HTTP_DATE};
use payraider_backend::models::batch_endpoints::{BatchConfig, BatchSubRequest};
use payraider_backend::models::field_selection_parameter::FieldSelectionConfig;
use payraider_backend::rate_limit::{
    rate_limit_middleware, ClientRateLimits, RateLimitConfig, RateLimiter,
};

const TEST_JWT_SECRET: &str = "test-jwt-secret-for-batch-security-audit-2832-32bytes";

async fn setup_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("in-memory sqlite pool");
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS token_revocations (
            jti TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            revoked_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS admin_ip_whitelist (
            id TEXT PRIMARY KEY,
            ip_or_cidr TEXT NOT NULL UNIQUE,
            description TEXT,
            added_by_user_id TEXT,
            added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("schema created");
    pool
}

fn sign_token(user_id: &str, username: &str, is_admin: bool) -> String {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        jti: Some(format!("jti-{user_id}")),
        session_id: Some(format!("sess-{user_id}")),
        sid: None,
        is_admin,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("JWT signed")
}

async fn build_secured_app(
    pool: SqlitePool,
    rate_limiter: Arc<RateLimiter>,
    batch_config: BatchConfig,
    admin_whitelist: Option<Arc<IpWhitelistConfig>>,
) -> Router {
    let ip_service = Arc::new(IpWhitelistService::new(pool.clone()));
    let field_selection = Arc::new(FieldSelectionParameter::new(FieldSelectionConfig::default()));
    let version_policy = Arc::new(ApiVersionPolicy::default());
    let jwt_secret = JwtSecret(Arc::from(TEST_JWT_SECRET));
    let revocation_store = TokenRevocationStore(Arc::new(pool));

    let public_routes = Router::new()
        .route(
            "/public/corridors",
            get(|| async {
                Json(json!({
                    "id": "USDC:native->NGN:issuer",
                    "name": "USDC to NGN",
                    "success_rate": 98.5,
                    "password_hash": "must-never-leak",
                    "ip_address": "10.0.0.42"
                }))
            }),
        )
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }));

    let admin_routes = Router::new()
        .nest(
            "/admin/ip-whitelist",
            admin_ip_whitelist::routes(ip_service),
        )
        .layer(middleware::from_fn(auth_middleware));

    let v1_router = Router::new()
        .merge(public_routes)
        .merge(admin_routes)
        .layer(middleware::from_fn_with_state(
            field_selection.clone(),
            field_selection_middleware,
        ))
        .layer(Extension(jwt_secret.clone()))
        .layer(Extension(revocation_store.clone()));

    let sub_router = Router::new()
        .nest("/api/v1", v1_router.clone())
        .nest(
            "/api/v2",
            Router::new().route(
                "/status",
                get(|| async { Json(json!({ "version": "v2" })) }),
            ),
        )
        .merge(v1_router.clone())
        .layer(middleware::from_fn_with_state(
            version_policy.clone(),
            api_versioning_middleware,
        ));

    let dispatcher = Arc::new(BatchDispatcher::new(
        batch_config,
        rate_limiter.clone(),
        field_selection,
        admin_whitelist,
    ));

    Router::new()
        .route("/api/v1/batch", post(batch_http_handler))
        .with_state(BatchHandlerState {
            dispatcher,
            sub_router,
        })
        .layer(Extension(jwt_secret))
        .layer(Extension(revocation_store))
        .nest("/api/v1", v1_router)
        .nest(
            "/api/v2",
            Router::new().route(
                "/status",
                get(|| async { Json(json!({ "version": "v2" })) }),
            ),
        )
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .layer(middleware::from_fn_with_state(
            version_policy,
            api_versioning_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
}

fn post_batch_req(body: Value, token: Option<&str>, ip: [u8; 4]) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/v1/batch")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(tok) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {tok}"));
    }
    let mut req = builder
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::from(ip)),
        50000,
    )));
    req
}

async fn read_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).expect("valid json body")
}

// ── #2832 Security Tests ──────────────────────────────────────────────────────

#[tokio::test]
async fn batch_containing_admin_route_from_non_admin_returns_subrequest_403() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let app = build_secured_app(pool, limiter, BatchConfig::default(), None).await;

    let user_token = sign_token("user-1", "alice", false);
    let admin_token = sign_token("admin-1", "root", true);

    let payload = json!({
        "requests": [
            { "method": "GET", "path": "/api/v1/public/corridors" },
            {
                "method": "GET",
                "path": "/api/v1/admin/ip-whitelist",
                // Attempt to escalate privileges by injecting an admin token inside the batch item:
                "headers": {
                    "Authorization": format!("Bearer {admin_token}")
                }
            }
        ]
    });

    let resp = app
        .oneshot(post_batch_req(payload, Some(&user_token), [10, 0, 0, 5]))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["total"], 2);
    assert_eq!(body["succeeded"], 1);
    assert_eq!(body["failed"], 1);

    // Public sub-request succeeded and sensitive/admin fields were stripped.
    assert_eq!(body["results"][0]["status"], 200);
    assert_eq!(body["results"][0]["success"], true);
    assert!(body["results"][0]["body"].get("password_hash").is_none());
    assert!(body["results"][0]["body"].get("ip_address").is_none());

    // Admin sub-request MUST return 403 Forbidden (spoofed Authorization header stripped).
    assert_eq!(body["results"][1]["status"], 403);
    assert_eq!(body["results"][1]["success"], false);
}

#[tokio::test]
async fn batch_containing_admin_route_without_auth_returns_subrequest_401() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let app = build_secured_app(pool, limiter, BatchConfig::default(), None).await;

    let payload = json!({
        "requests": [
            { "method": "GET", "path": "/api/v1/admin/ip-whitelist" }
        ]
    });

    let resp = app
        .oneshot(post_batch_req(payload, None, [10, 0, 0, 6]))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["results"][0]["status"], 401);
    assert_eq!(body["results"][0]["success"], false);
}

#[tokio::test]
async fn batch_enforces_admin_ip_whitelist_and_ignores_spoofed_xff() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let whitelist = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("192.168.10.0/24").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });
    let app = build_secured_app(pool, limiter, BatchConfig::default(), Some(whitelist)).await;
    let admin_token = sign_token("admin-1", "root", true);

    // Non-whitelisted IP tries to spoof X-Forwarded-For inside sub-request payload -> 403!
    let blocked_payload = json!({
        "requests": [
            {
                "method": "GET",
                "path": "/api/v1/admin/ip-whitelist",
                "headers": { "X-Forwarded-For": "192.168.10.5" }
            }
        ]
    });
    let blocked_resp = app
        .clone()
        .oneshot(post_batch_req(
            blocked_payload,
            Some(&admin_token),
            [203, 0, 113, 9],
        ))
        .await
        .unwrap();
    assert_eq!(blocked_resp.status(), StatusCode::OK);
    let blocked_body = read_json(blocked_resp).await;
    assert_eq!(blocked_body["results"][0]["status"], 403);
    assert_eq!(
        blocked_body["results"][0]["error"]["code"],
        "ADMIN_IP_NOT_WHITELISTED"
    );

    // Same admin from a whitelisted IP succeeds with 200!
    let allowed_payload = json!({
        "requests": [
            { "method": "GET", "path": "/api/v1/admin/ip-whitelist" }
        ]
    });
    let allowed_resp = app
        .oneshot(post_batch_req(
            allowed_payload,
            Some(&admin_token),
            [192, 168, 10, 5],
        ))
        .await
        .unwrap();
    assert_eq!(allowed_resp.status(), StatusCode::OK);
    let allowed_body = read_json(allowed_resp).await;
    assert_eq!(allowed_body["results"][0]["status"], 200);
    assert_eq!(allowed_body["results"][0]["success"], true);
}

#[tokio::test]
async fn batch_of_100_counts_as_100_against_rate_limit() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    // Configure `/api/v1/batch` with an exact anonymous limit of 100/min.
    limiter
        .register_endpoint(
            "/api/v1/batch".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 100,
                }),
            },
        )
        .await;

    let app = build_secured_app(
        pool,
        limiter.clone(),
        BatchConfig {
            max_batch_size: 100,
            max_total_cost: 200,
            ..Default::default()
        },
        None,
    )
    .await;

    let hundred_requests: Vec<Value> = (0..100)
        .map(|_| json!({ "method": "GET", "path": "/api/v1/public/corridors" }))
        .collect();

    // Send 1 batch of 100 sub-requests from 198.51.100.10.
    let resp = app
        .clone()
        .oneshot(post_batch_req(
            json!({ "requests": hundred_requests }),
            None,
            [198, 51, 100, 10],
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["total"], 100);
    assert_eq!(body["succeeded"], 100);
    assert_eq!(body["failed"], 0);

    // Because the batch of 100 consumed all 100 units, the very next request
    // (101st unit) from the same IP MUST be rejected with 429 Too Many Requests.
    let next_resp = app
        .oneshot(post_batch_req(
            json!({ "requests": [{ "method": "GET", "path": "/api/v1/public/corridors" }] }),
            None,
            [198, 51, 100, 10],
        ))
        .await
        .unwrap();
    assert_eq!(next_resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn batch_of_100_on_60_rpm_limit_rate_limits_remaining_40_subrequests() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    // Default anonymous limit is 60 req/min.
    let app = build_secured_app(
        pool,
        limiter,
        BatchConfig {
            max_batch_size: 100,
            max_total_cost: 200,
            ..Default::default()
        },
        None,
    )
    .await;

    let hundred_requests: Vec<Value> = (0..100)
        .map(|_| json!({ "method": "GET", "path": "/api/v1/public/corridors" }))
        .collect();

    let resp = app
        .oneshot(post_batch_req(
            json!({ "requests": hundred_requests }),
            None,
            [198, 51, 100, 20],
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = read_json(resp).await;
    assert_eq!(body["total"], 100);
    assert_eq!(body["succeeded"], 60);
    assert_eq!(body["failed"], 40);
    assert_eq!(body["results"][59]["status"], 200);
    assert_eq!(body["results"][60]["status"], 429);
    assert_eq!(body["results"][99]["status"], 429);
}

#[tokio::test]
async fn batch_rejects_oversized_nested_traversal_and_cost_exceeded_requests() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let app = build_secured_app(
        pool,
        limiter,
        BatchConfig {
            max_batch_size: 2,
            max_total_cost: 5,
            ..Default::default()
        },
        None,
    )
    .await;

    // 1. Oversized batch (> max_batch_size) -> 400 BATCH_TOO_LARGE
    let oversized = app
        .clone()
        .oneshot(post_batch_req(
            json!({
                "requests": [
                    { "method": "GET", "path": "/api/v1/public/corridors" },
                    { "method": "GET", "path": "/api/v1/public/corridors" },
                    { "method": "GET", "path": "/api/v1/public/corridors" }
                ]
            }),
            None,
            [10, 1, 0, 1],
        ))
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json(oversized).await["error"]["code"],
        "BATCH_TOO_LARGE"
    );

    // 2. Recursive nested batch -> 400 NESTED_BATCH_FORBIDDEN
    let nested = app
        .clone()
        .oneshot(post_batch_req(
            json!({
                "requests": [
                    { "method": "POST", "path": "/api/v1/batch" }
                ]
            }),
            None,
            [10, 1, 0, 2],
        ))
        .await
        .unwrap();
    assert_eq!(nested.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json(nested).await["error"]["code"],
        "NESTED_BATCH_FORBIDDEN"
    );

    // 3. Encoded path traversal -> 400 PATH_TRAVERSAL_REJECTED
    let traversal = app
        .clone()
        .oneshot(post_batch_req(
            json!({
                "requests": [
                    { "method": "GET", "path": "/api/v1/public/%2e%2e/admin/ip-whitelist" }
                ]
            }),
            None,
            [10, 1, 0, 3],
        ))
        .await
        .unwrap();
    assert_eq!(traversal.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json(traversal).await["error"]["code"],
        "PATH_TRAVERSAL_REJECTED"
    );

    // 4. Cost cap exceeded (2 POSTs = 6 > 5) -> 400 BATCH_COST_EXCEEDED
    let costly = app
        .oneshot(post_batch_req(
            json!({
                "requests": [
                    { "method": "POST", "path": "/api/v1/public/corridors" },
                    { "method": "POST", "path": "/api/v1/public/corridors" }
                ]
            }),
            None,
            [10, 1, 0, 4],
        ))
        .await
        .unwrap();
    assert_eq!(costly.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_json(costly).await["error"]["code"],
        "BATCH_COST_EXCEEDED"
    );
}

#[tokio::test]
async fn batch_field_selection_enforces_role_policy() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let app = build_secured_app(pool, limiter, BatchConfig::default(), None).await;

    let user_token = sign_token("user-2", "bob", false);
    let admin_token = sign_token("admin-2", "super", true);

    // Non-admin selecting allowed fields + secret field + admin-only field:
    let user_resp = app
        .clone()
        .oneshot(post_batch_req(
            json!({
                "requests": [
                    { "method": "GET", "path": "/api/v1/public/corridors", "fields": "id,name" },
                    { "method": "GET", "path": "/api/v1/public/corridors", "fields": "id,password_hash" },
                    { "method": "GET", "path": "/api/v1/public/corridors", "fields": "id,ip_address" }
                ]
            }),
            Some(&user_token),
            [10, 2, 0, 1],
        ))
        .await
        .unwrap();
    let user_body = read_json(user_resp).await;
    assert_eq!(user_body["results"][0]["status"], 200);
    assert_eq!(
        user_body["results"][0]["body"],
        json!({ "id": "USDC:native->NGN:issuer", "name": "USDC to NGN" })
    );
    assert_eq!(user_body["results"][1]["status"], 403);
    assert_eq!(user_body["results"][2]["status"], 403);

    // Admin selecting admin-only field succeeds, but secret field still rejected:
    let admin_resp = app
        .oneshot(post_batch_req(
            json!({
                "requests": [
                    { "method": "GET", "path": "/api/v1/public/corridors", "fields": "id,ip_address" },
                    { "method": "GET", "path": "/api/v1/public/corridors", "fields": "id,password_hash" }
                ]
            }),
            Some(&admin_token),
            [10, 2, 0, 2],
        ))
        .await
        .unwrap();
    let admin_body = read_json(admin_resp).await;
    assert_eq!(admin_body["results"][0]["status"], 200);
    assert_eq!(admin_body["results"][0]["body"]["ip_address"], "10.0.0.42");
    assert_eq!(admin_body["results"][1]["status"], 403);
}

#[tokio::test]
async fn dispatcher_service_works_directly_without_http_layer() {
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let dispatcher = BatchDispatcher::new(
        BatchConfig::default(),
        limiter,
        Arc::new(FieldSelectionParameter::new(FieldSelectionConfig::default())),
        None,
    );
    let sub_router = Router::new().route("/ping", get(|| async { Json(json!({ "ok": true })) }));
    let identity = BatchCallerIdentity::from_outer_request(
        &axum::http::HeaderMap::new(),
        Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4000)),
        None,
        Some("req-1".to_string()),
        None,
    );

    let resp = dispatcher
        .dispatch_router(
            sub_router,
            &identity,
            vec![BatchSubRequest {
                method: "GET".to_string(),
                path: "/ping".to_string(),
                headers: HashMap::new(),
                body: None,
                fields: None,
            }],
        )
        .await
        .expect("direct dispatch succeeds");
    assert_eq!(resp.total, 1);
    assert_eq!(resp.succeeded, 1);
    assert_eq!(resp.results[0].body, Some(json!({ "ok": true })));
}

// ── #2833 Consolidated API Versioning Integration Tests ───────────────────────

#[tokio::test]
async fn consolidated_versioning_headers_for_deprecated_current_and_health_routes() {
    let pool = setup_pool().await;
    let limiter = Arc::new(RateLimiter::new_memory_only(None));
    let app = build_secured_app(pool, limiter, BatchConfig::default(), None).await;

    // 1. Deprecated v1 route emits single RFC 9745 Deprecation + RFC 8594 Sunset
    let v1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/public/corridors")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(v1.status(), StatusCode::OK);
    assert_eq!(v1.headers().get("X-API-Version").unwrap(), "v1");
    assert_eq!(v1.headers().get("X-API-Status").unwrap(), "deprecated");
    assert_eq!(
        v1.headers().get("Deprecation").unwrap(),
        &format!("@{V1_DEPRECATION_UNIX}")
    );
    assert_eq!(v1.headers().get("Sunset").unwrap(), V1_SUNSET_HTTP_DATE);
    assert_eq!(v1.headers().get_all("Deprecation").iter().count(), 1);
    assert_eq!(v1.headers().get_all("Sunset").iter().count(), 1);

    // 2. Preview v2 route omits Deprecation/Sunset headers
    let v2 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v2/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(v2.status(), StatusCode::OK);
    assert_eq!(v2.headers().get("X-API-Version").unwrap(), "v2");
    assert_eq!(v2.headers().get("X-API-Status").unwrap(), "preview");
    assert!(!v2.headers().contains_key("Deprecation"));
    assert!(!v2.headers().contains_key("Sunset"));

    // 3. Health route emits no deprecation headers
    let health = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);
    assert!(!health.headers().contains_key("Deprecation"));
    assert!(!health.headers().contains_key("Sunset"));
}
