use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub async fn version_middleware(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let accept = request
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Check for unsupported vendor versions, e.g. application/vnd.payraider.v3...
    if let Some(pos) = accept.find("application/vnd.payraider.v") {
        let rest = &accept[pos + "application/vnd.payraider.v".len()..];
        let ver_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(ver_num) = ver_str.parse::<u32>() {
            if ver_num != 1 && ver_num != 2 {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    Json(json!({
                        "error": "unsupported_api_version",
                        "message": format!("API version v{} is not supported", ver_num),
                        "supported_versions": ["v1", "v2"],
                        "status": 400
                    })),
                )
                    .into_response();
            }
        }
    }

    let is_v2 = path.starts_with("/api/v2") || accept.contains("application/vnd.payraider.v2");

    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    if is_v2 {
        headers.insert("X-API-Version", HeaderValue::from_static("v2"));
        headers.insert("X-API-Status", HeaderValue::from_static("preview"));
    } else {
        headers.insert("X-API-Version", HeaderValue::from_static("v1"));
        headers.insert("X-API-Status", HeaderValue::from_static("deprecated"));
        headers.insert("Deprecation", HeaderValue::from_static("true"));
        headers.insert(
            "Sunset",
            HeaderValue::from_static("Thu, 31 Dec 2026 00:00:00 GMT"),
        );
        headers.insert(
            "Link",
            HeaderValue::from_static("</api/v2/>; rel=\"successor-version\""),
        );
        if let Ok(v) = HeaderValue::from_str("299 - \"API v1 is deprecated. Please migrate to v2. See docs/API_VERSIONING.md\"") {
            headers.insert("Warning", v);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn version_middleware_defaults_to_v1_with_deprecation_headers() {
        let app = Router::new()
            .route("/api/corridors", get(|| async { "ok" }))
            .layer(middleware::from_fn(version_middleware));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/corridors")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("X-API-Version").unwrap(), "v1");
        assert_eq!(res.headers().get("X-API-Status").unwrap(), "deprecated");
        assert_eq!(res.headers().get("Deprecation").unwrap(), "true");
        assert_eq!(res.headers().get("Sunset").unwrap(), "Thu, 31 Dec 2026 00:00:00 GMT");
        assert!(res.headers().contains_key("Link"));
        assert!(res.headers().contains_key("Warning"));
    }

    #[tokio::test]
    async fn version_middleware_negotiates_v2_via_accept_header() {
        let app = Router::new()
            .route("/api/corridors", get(|| async { "ok" }))
            .layer(middleware::from_fn(version_middleware));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/corridors")
                    .header("Accept", "application/vnd.payraider.v2+json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers().get("X-API-Version").unwrap(), "v2");
        assert_eq!(res.headers().get("X-API-Status").unwrap(), "preview");
        assert!(!res.headers().contains_key("Deprecation"));
    }

    #[tokio::test]
    async fn version_middleware_rejects_unsupported_version() {
        let app = Router::new()
            .route("/api/corridors", get(|| async { "ok" }))
            .layer(middleware::from_fn(version_middleware));

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/corridors")
                    .header("Accept", "application/vnd.payraider.v3+json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json_body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json_body["error"], "unsupported_api_version");
    }
}

