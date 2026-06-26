//! Shared helpers for SEP integration tests (feature: sep-integration).

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use tower::util::ServiceExt;

pub async fn mock_anchor_url() -> String {
    if let Ok(url) = std::env::var("MOCK_ANCHOR_URL") {
        return url.trim_end_matches('/').to_string();
    }

    let app = Router::new()
        .route("/info", get(|| async {
            Json(json!({
                "version": "1.0.0",
                "fee": { "enabled": true },
                "deposit": { "enabled": true },
                "withdraw": { "enabled": true }
            }))
        }))
        .route(
            "/transactions/deposit/interactive",
            post(|| async { Json(json!({ "type": "interactive_customer_info_needed" })) }),
        )
        .route(
            "/transactions/withdraw/interactive",
            post(|| async { Json(json!({ "type": "interactive_customer_info_needed" })) }),
        )
        .route(
            "/transactions",
            get(|| async { Json(json!({ "transactions": [] })) }).post(|| async {
                Json(json!({ "id": "tx-1", "status": "pending" }))
            }),
        )
        .route("/transaction", get(|| async {
            Json(json!({ "id": "tx-1", "status": "pending" }))
        }))
        .route("/quote", post(|| async {
            Json(json!({ "id": "quote-1", "price": "1.0" }))
        }))
        .route(
            "/customer",
            get(|| async { Json(json!({ "id": "customer-1", "status": "ACCEPTED" })) }).put(
                || async { Json(json!({ "id": "customer-1", "status": "ACCEPTED" })) },
            ),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock anchor");
    let addr = listener.local_addr().expect("mock anchor addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock anchor server");
    });

    format!("http://{addr}")
}

pub async fn response_json(app: Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.expect("request");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: Value = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&body).unwrap_or(json!({ "raw": String::from_utf8_lossy(&body) }))
    };
    (status, json)
}

pub fn sep24_router() -> Router {
    stellar_insights_backend::api::sep24_proxy::routes()
}

pub fn sep31_router() -> Router {
    stellar_insights_backend::api::sep31_proxy::routes()
}

pub fn sep10_router() -> Router {
    use std::sync::Arc;
    use stellar_insights_backend::api::sep10;
    use stellar_insights_backend::auth::sep10_simple::Sep10Service;

    let service = Arc::new(
        Sep10Service::new(
            "SBZVMB74YYQS3VQJMXZ7OZGD5GXZMHQHX3YYQS3VQJMXZ7OZGD5GXZMHQH",
            "Test SDF Network ; September 2015".to_string(),
            "testanchor.stellar.org".to_string(),
            Arc::new(tokio::sync::RwLock::new(None)),
        )
        .expect("sep10 service"),
    );

    sep10::routes(service)
}
