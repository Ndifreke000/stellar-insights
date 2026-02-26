use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

pub async fn version_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    headers.insert("X-API-Version", HeaderValue::from_static("v1"));
    headers.insert("X-API-Status", HeaderValue::from_static("supported"));

    response
}
