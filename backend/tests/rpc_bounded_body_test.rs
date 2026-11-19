//! Oversize provider responses are aborted with bounded memory (#2778).

use payraider_backend::rpc::bounded_body::{read_bounded, read_horizon_records, BodyLimit};
use payraider_backend::rpc::error::RpcError;
use serde_json::Value;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn get(server: &MockServer) -> reqwest::Response {
    reqwest::get(format!("{}/x", server.uri())).await.unwrap()
}

#[tokio::test]
async fn oversize_body_is_rejected_before_full_buffering() {
    let server = MockServer::start().await;
    // Larger than the page limit; reading stops at the limit, not at the body end.
    let body = vec![b' '; BodyLimit::Page.bytes() * 2];
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&server)
        .await;

    let err = read_bounded(get(&server).await, BodyLimit::Page)
        .await
        .unwrap_err();
    assert!(matches!(err, RpcError::ResponseTooLarge { .. }), "{err:?}");
}

#[tokio::test]
async fn body_within_limit_is_read() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"_embedded":{"records":[{"id":"1"},{"id":"2"}]}}"#),
        )
        .mount(&server)
        .await;

    let records: Vec<Value> = read_horizon_records(get(&server).await).await.unwrap();
    assert_eq!(records.len(), 2);
}

#[tokio::test]
async fn single_limit_is_smaller_than_page_limit() {
    let server = MockServer::start().await;
    let body = vec![b'x'; BodyLimit::Single.bytes() + 1];
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&server)
        .await;

    let err = read_bounded(get(&server).await, BodyLimit::Single)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        RpcError::ResponseTooLarge { limit, .. } if limit == BodyLimit::Single.bytes()
    ));
}
