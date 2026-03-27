use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware,
    response::Response,
    routing::get,
    Router,
};
use serde_json::Value;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

use stellar_insights_backend::ip_whitelist_middleware::{
    ip_whitelist_middleware, IpWhitelistConfig,
};

// Helper function to create a test handler
async fn test_handler() -> &'static str {
    "Admin endpoint accessed"
}

// Helper function to parse response body as JSON
async fn response_to_json(response: Response) -> Value {
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body_bytes).unwrap()
}

#[tokio::test]
async fn test_allowed_single_ip() {
    // Create config that allows 192.168.1.100
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("192.168.1.100").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test allowed IP
    let allowed_addr = SocketAddr::from_str("192.168.1.100:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(allowed_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_blocked_ip() {
    // Create config that allows 192.168.1.100
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("192.168.1.100").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test blocked IP
    let blocked_addr = SocketAddr::from_str("192.168.1.101:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(blocked_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let json = response_to_json(response).await;
    assert!(json["error"].as_str().unwrap().contains("not whitelisted"));
}

#[tokio::test]
async fn test_cidr_range_allowed() {
    // Create config that allows 192.168.1.0/24
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("192.168.1.0/24").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test IP within CIDR range
    let allowed_addr = SocketAddr::from_str("192.168.1.50:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(allowed_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cidr_range_blocked() {
    // Create config that allows 192.168.1.0/24
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("192.168.1.0/24").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test IP outside CIDR range
    let blocked_addr = SocketAddr::from_str("192.168.2.50:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(blocked_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_x_forwarded_for_with_trust_proxy() {
    // Create config that allows 203.0.113.50 and trusts proxy
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("203.0.113.50").unwrap()),
        trust_proxy: true,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test with X-Forwarded-For header
    let proxy_addr = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .header("x-forwarded-for", "203.0.113.50, 10.0.0.1")
        .extension(ConnectInfo(proxy_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_x_forwarded_for_blocked_with_trust_proxy() {
    // Create config that allows 203.0.113.50 and trusts proxy
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("203.0.113.50").unwrap()),
        trust_proxy: true,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test with X-Forwarded-For header containing non-whitelisted IP
    let proxy_addr = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .header("x-forwarded-for", "203.0.113.99, 10.0.0.1")
        .extension(ConnectInfo(proxy_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_x_forwarded_for_without_trust_proxy() {
    // Create config that allows 10.0.0.1 but does NOT trust proxy
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("10.0.0.1").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test with X-Forwarded-For header - should ignore it and use direct connection IP
    let proxy_addr = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .header("x-forwarded-for", "203.0.113.50, 10.0.0.1")
        .extension(ConnectInfo(proxy_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_x_real_ip_with_trust_proxy() {
    // Create config that allows 203.0.113.50 and trusts proxy
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("203.0.113.50").unwrap()),
        trust_proxy: true,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test with X-Real-IP header (nginx style)
    let proxy_addr = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .header("x-real-ip", "203.0.113.50")
        .extension(ConnectInfo(proxy_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ipv6_localhost() {
    // Create config that allows IPv6 localhost
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("::1").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test IPv6 localhost
    let allowed_addr = SocketAddr::from_str("[::1]:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(allowed_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ipv6_cidr_range() {
    // Create config that allows 2001:db8::/32
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("2001:db8::/32").unwrap()),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test IP within IPv6 CIDR range
    let allowed_addr = SocketAddr::from_str("[2001:db8::1]:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(allowed_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_multiple_networks() {
    // Create config with multiple networks
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(
            IpWhitelistConfig::parse_whitelist("192.168.1.0/24, 10.0.0.1, ::1").unwrap(),
        ),
        trust_proxy: false,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test first network
    let addr1 = SocketAddr::from_str("192.168.1.50:1234").unwrap();
    let request1 = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(addr1))
        .body(Body::empty())
        .unwrap();
    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    // Test second network
    let addr2 = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request2 = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(addr2))
        .body(Body::empty())
        .unwrap();
    let response2 = app.clone().oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    // Test third network (IPv6)
    let addr3 = SocketAddr::from_str("[::1]:1234").unwrap();
    let request3 = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(addr3))
        .body(Body::empty())
        .unwrap();
    let response3 = app.clone().oneshot(request3).await.unwrap();
    assert_eq!(response3.status(), StatusCode::OK);

    // Test non-whitelisted IP
    let addr4 = SocketAddr::from_str("172.16.0.1:1234").unwrap();
    let request4 = Request::builder()
        .uri("/admin/test")
        .extension(ConnectInfo(addr4))
        .body(Body::empty())
        .unwrap();
    let response4 = app.clone().oneshot(request4).await.unwrap();
    assert_eq!(response4.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_malformed_x_forwarded_for() {
    // Create config that trusts proxy
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("10.0.0.1").unwrap()),
        trust_proxy: true,
        max_forwarded_ips: 3,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test with malformed X-Forwarded-For - should fall back to direct connection IP
    let proxy_addr = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .header("x-forwarded-for", "invalid-ip-address")
        .extension(ConnectInfo(proxy_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_max_forwarded_ips_limit() {
    // Create config with max_forwarded_ips = 2
    let config = Arc::new(IpWhitelistConfig {
        allowed_networks: Arc::new(IpWhitelistConfig::parse_whitelist("203.0.113.50").unwrap()),
        trust_proxy: true,
        max_forwarded_ips: 2,
    });

    let app = Router::new().route("/admin/test", get(test_handler)).layer(
        middleware::from_fn_with_state(config, ip_whitelist_middleware),
    );

    // Test with long X-Forwarded-For chain - should only check first 2
    let proxy_addr = SocketAddr::from_str("10.0.0.1:1234").unwrap();
    let request = Request::builder()
        .uri("/admin/test")
        .header(
            "x-forwarded-for",
            "203.0.113.50, 10.0.0.1, 10.0.0.2, 10.0.0.3",
        )
        .extension(ConnectInfo(proxy_addr))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_parse_whitelist_invalid_ip() {
    let result = IpWhitelistConfig::parse_whitelist("invalid.ip.address");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid IP address"));
}

#[test]
fn test_parse_whitelist_empty() {
    let result = IpWhitelistConfig::parse_whitelist("");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot be empty"));
}

#[test]
fn test_parse_whitelist_mixed_valid_invalid() {
    let result = IpWhitelistConfig::parse_whitelist("192.168.1.1, invalid, 10.0.0.1");
    assert!(result.is_err());
}
