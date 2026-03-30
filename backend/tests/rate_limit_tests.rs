use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use stellar_insights_backend::rate_limit::{
    ClientIdentifier, ClientRateLimits, ClientTier, RateLimitConfig, RateLimiter,
};

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{nanos}-{counter}")
}

#[tokio::test]
async fn test_client_identifier_tier() {
    let api_key_client = ClientIdentifier::ApiKey("test_key_123".to_string());
    assert_eq!(api_key_client.tier(), ClientTier::Authenticated);

    let user_client = ClientIdentifier::User("user_456".to_string());
    assert_eq!(user_client.tier(), ClientTier::Authenticated);

    let ip_client = ClientIdentifier::IpAddress("192.168.1.1".to_string());
    assert_eq!(ip_client.tier(), ClientTier::Anonymous);
}

#[tokio::test]
async fn test_client_identifier_as_key() {
    let api_key_client = ClientIdentifier::ApiKey("test_key_123".to_string());
    assert_eq!(api_key_client.as_key(), "apikey:test_key_123");

    let user_client = ClientIdentifier::User("user_456".to_string());
    assert_eq!(user_client.as_key(), "user:user_456");

    let ip_client = ClientIdentifier::IpAddress("192.168.1.1".to_string());
    assert_eq!(ip_client.as_key(), "ip:192.168.1.1");
}

#[tokio::test]
async fn test_rate_limiter_initialization() {
    let limiter = RateLimiter::new().await;
    assert!(
        limiter.is_ok(),
        "Rate limiter should initialize successfully"
    );
}

#[tokio::test]
async fn test_rate_limit_config_default() {
    let config = RateLimitConfig::default();
    assert_eq!(config.requests_per_minute, 100);
    assert!(config.client_limits.is_some());

    let client_limits = config.client_limits.unwrap();
    assert_eq!(client_limits.authenticated, 200);
    assert_eq!(client_limits.premium, 1000);
    assert_eq!(client_limits.anonymous, 60);
}

#[tokio::test]
async fn test_rate_limit_anonymous_client() {
    let limiter = RateLimiter::new().await.unwrap();
    let suffix = unique_suffix();
    let endpoint = format!("/test/endpoint-anon-{suffix}");
    let ip = format!("192.168.1.{}", (suffix.len() % 200) + 1);

    // Register endpoint with client-specific limits
    limiter
        .register_endpoint(
            endpoint.clone(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 10,
                }),
            },
        )
        .await;

    let client = ClientIdentifier::IpAddress(ip.clone());

    // First 10 requests should succeed
    for i in 0..10 {
        let (allowed, info) = limiter
            .check_rate_limit_for_client(&client, &endpoint, &ip)
            .await;
        assert!(allowed, "Request {} should be allowed", i + 1);
        assert_eq!(info.limit, 10);
        assert_eq!(info.remaining, 10 - i - 1);
    }

    // 11th request should be rate limited
    let (allowed, info) = limiter
        .check_rate_limit_for_client(&client, &endpoint, &ip)
        .await;
    assert!(!allowed, "Request 11 should be rate limited");
    assert_eq!(info.remaining, 0);
}

#[tokio::test]
async fn test_rate_limit_authenticated_client() {
    let limiter = RateLimiter::new().await.unwrap();
    let suffix = unique_suffix();
    let endpoint = format!("/test/endpoint-auth-{suffix}");
    let client = ClientIdentifier::ApiKey(format!("test_api_key_{suffix}"));

    limiter
        .register_endpoint(
            endpoint.clone(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 20,
                    premium: 1000,
                    anonymous: 10,
                }),
            },
        )
        .await;

    // First 20 requests should succeed (authenticated limit)
    for i in 0..20 {
        let (allowed, info) = limiter
            .check_rate_limit_for_client(&client, &endpoint, "192.168.1.1")
            .await;
        assert!(allowed, "Request {} should be allowed", i + 1);
        assert_eq!(info.limit, 20);
    }

    // 21st request should be rate limited
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client, &endpoint, "192.168.1.1")
        .await;
    assert!(!allowed, "Request 21 should be rate limited");
}

#[tokio::test]
async fn test_rate_limit_different_clients_independent() {
    let limiter = RateLimiter::new().await.unwrap();
    let suffix = unique_suffix();
    let endpoint = format!("/test/endpoint-clients-{suffix}");
    let client1_ip = format!("192.168.10.{}", (suffix.len() % 200) + 1);
    let client2_ip = format!("192.168.20.{}", (suffix.len() % 200) + 1);

    limiter
        .register_endpoint(
            endpoint.clone(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 5,
                }),
            },
        )
        .await;

    let client1 = ClientIdentifier::IpAddress(client1_ip.clone());
    let client2 = ClientIdentifier::IpAddress(client2_ip.clone());

    // Exhaust client1's limit
    for _ in 0..5 {
        let (allowed, _) = limiter
            .check_rate_limit_for_client(&client1, &endpoint, &client1_ip)
            .await;
        assert!(allowed);
    }

    // Client1 should be rate limited
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client1, &endpoint, &client1_ip)
        .await;
    assert!(!allowed);

    // Client2 should still be allowed
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client2, &endpoint, &client2_ip)
        .await;
    assert!(allowed);
}

#[tokio::test]
async fn test_rate_limit_whitelist() {
    let limiter = RateLimiter::new().await.unwrap();

    limiter
        .register_endpoint(
            "/test/endpoint".to_string(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec!["192.168.1.100".to_string()],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 5,
                }),
            },
        )
        .await;

    let client = ClientIdentifier::IpAddress("192.168.1.100".to_string());

    // Whitelisted IP should never be rate limited
    for _ in 0..200 {
        let (allowed, info) = limiter
            .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.100")
            .await;
        assert!(allowed);
        assert!(info.is_whitelisted);
    }
}

#[tokio::test]
async fn test_rate_limit_different_endpoints() {
    let limiter = RateLimiter::new().await.unwrap();
    let suffix = unique_suffix();
    let endpoint1 = format!("/endpoint1-{suffix}");
    let endpoint2 = format!("/endpoint2-{suffix}");
    let ip = format!("192.168.30.{}", (suffix.len() % 200) + 1);

    limiter
        .register_endpoint(
            endpoint1.clone(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 5,
                }),
            },
        )
        .await;

    limiter
        .register_endpoint(
            endpoint2.clone(),
            RateLimitConfig {
                requests_per_minute: 100,
                whitelist_ips: vec![],
                client_limits: Some(ClientRateLimits {
                    authenticated: 200,
                    premium: 1000,
                    anonymous: 10,
                }),
            },
        )
        .await;

    let client = ClientIdentifier::IpAddress(ip.clone());

    // Exhaust endpoint1 limit
    for _ in 0..5 {
        let (allowed, _) = limiter
            .check_rate_limit_for_client(&client, &endpoint1, &ip)
            .await;
        assert!(allowed);
    }

    // Endpoint1 should be rate limited
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client, &endpoint1, &ip)
        .await;
    assert!(!allowed);

    // Endpoint2 should still be allowed (different limit)
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client, &endpoint2, &ip)
        .await;
    assert!(allowed);
}

#[tokio::test]
async fn test_rate_limit_info_includes_client_id() {
    let limiter = RateLimiter::new().await.unwrap();

    limiter
        .register_endpoint("/test/endpoint".to_string(), RateLimitConfig::default())
        .await;

    let client = ClientIdentifier::ApiKey("test_key_123".to_string());

    let (_, info) = limiter
        .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.1")
        .await;

    assert!(info.client_id.is_some());
    assert_eq!(info.client_id.unwrap(), "apikey:test_key_123");
}

#[tokio::test]
async fn test_rate_limit_headers() {
    use axum::http::{header, StatusCode};
    use axum::response::IntoResponse;
    use stellar_insights_backend::rate_limit::{add_rate_limit_headers, RateLimitInfo};

    let info = RateLimitInfo {
        limit: 100,
        remaining: 0, // Should trigger Retry-After
        reset_at: 123456789,
        reset_after_seconds: 30,
        window_seconds: 60,
        is_whitelisted: false,
        client_id: Some("test_client".to_string()),
    };

    let response = (StatusCode::OK, "OK").into_response();
    let response = add_rate_limit_headers(response, &info).expect("Failed to add headers");

    let headers = response.headers();
    assert_eq!(headers.get("RateLimit-Limit").unwrap(), "100");
    assert_eq!(headers.get("RateLimit-Remaining").unwrap(), "0");
    assert_eq!(headers.get("RateLimit-Reset").unwrap(), "123456789");
    assert_eq!(headers.get(header::RETRY_AFTER).unwrap(), "30");
    assert_eq!(
        headers.get("X-RateLimit-Policy").unwrap(),
        "100 requests per 60 seconds"
    );
    assert_eq!(headers.get("X-RateLimit-Client").unwrap(), "test_client");
}
