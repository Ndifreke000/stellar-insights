use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use std::sync::Arc;
use stellar_insights_backend::api::auth;
use stellar_insights_backend::auth::{AuthService, User};
use tokio::sync::RwLock;
use tower::util::ServiceExt;

fn create_auth_router() -> Router {
    if std::env::var("JWT_SECRET").is_err() {
        std::env::set_var(
            "JWT_SECRET",
            "test_jwt_secret_key_that_is_long_enough_for_tests_32",
        );
    }

    let redis = Arc::new(RwLock::new(None));
    let auth_service = Arc::new(AuthService::new(redis));
    auth::routes(auth_service)
}

#[tokio::test]
async fn test_login_endpoint_rejects_invalid_credentials() {
    let app = create_auth_router();
    let payload = json!({
        "username": "nonexistent-user",
        "password": "wrong-password"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&body).unwrap();
    let serialized = parsed.to_string();
    assert!(serialized.contains("INVALID_CREDENTIALS"));
}

#[tokio::test]
async fn test_refresh_and_logout_reject_invalid_tokens() {
    let app = create_auth_router();

    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "refresh_token": "not-a-valid-token" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(refresh_response.status(), StatusCode::UNAUTHORIZED);

    let logout_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/logout")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "refresh_token": "not-a-valid-token" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(logout_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_generation_and_validation_for_access_tokens() {
    if std::env::var("JWT_SECRET").is_err() {
        std::env::set_var(
            "JWT_SECRET",
            "test_jwt_secret_key_that_is_long_enough_for_tests_32",
        );
    }

    let redis = Arc::new(RwLock::new(None));
    let auth_service = AuthService::new(redis);

    let user = User {
        id: "user-1".to_string(),
        username: "test-user".to_string(),
    };

    let access_token = auth_service.generate_access_token(&user).unwrap();
    let claims = auth_service.validate_token(&access_token).unwrap();

    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.username, "test-user");
    assert_eq!(claims.token_type, "access");
}
