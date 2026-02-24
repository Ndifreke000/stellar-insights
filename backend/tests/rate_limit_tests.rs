use stellar_insights_backend::rate_limit::{
    ClientIdentifier, ClientRateLimits, ClientTier, RateLimitConfig, RateLimiter,
};

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
    assert!(limiter.is_ok(), "Rate limiter should initialize successfully");
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

    // Register endpoint with client-specific limits
    limiter
        .register_endpoint(
            "/test/endpoint".to_string(),
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

    let client = ClientIdentifier::IpAddress("192.168.1.1".to_string());

    // First 10 requests should succeed
    for i in 0..10 {
        let (allowed, info) = limiter
            .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.1")
            .await;
        assert!(allowed, "Request {} should be allowed", i + 1);
        assert_eq!(info.limit, 10);
        assert_eq!(info.remaining, 10 - i - 1);
    }

    // 11th request should be rate limited
    let (allowed, info) = limiter
        .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.1")
        .await;
    assert!(!allowed, "Request 11 should be rate limited");
    assert_eq!(info.remaining, 0);
}

#[tokio::test]
async fn test_rate_limit_authenticated_client() {
    let limiter = RateLimiter::new().await.unwrap();

    limiter
        .register_endpoint(
            "/test/endpoint".to_string(),
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

    let client = ClientIdentifier::ApiKey("test_api_key".to_string());

    // First 20 requests should succeed (authenticated limit)
    for i in 0..20 {
        let (allowed, info) = limiter
            .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.1")
            .await;
        assert!(allowed, "Request {} should be allowed", i + 1);
        assert_eq!(info.limit, 20);
    }

    // 21st request should be rate limited
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.1")
        .await;
    assert!(!allowed, "Request 21 should be rate limited");
}

#[tokio::test]
async fn test_rate_limit_different_clients_independent() {
    let limiter = RateLimiter::new().await.unwrap();

    limiter
        .register_endpoint(
            "/test/endpoint".to_string(),
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

    let client1 = ClientIdentifier::IpAddress("192.168.1.1".to_string());
    let client2 = ClientIdentifier::IpAddress("192.168.1.2".to_string());

    // Exhaust client1's limit
    for _ in 0..5 {
        let (allowed, _) = limiter
            .check_rate_limit_for_client(&client1, "/test/endpoint", "192.168.1.1")
            .await;
        assert!(allowed);
    }

    // Client1 should be rate limited
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client1, "/test/endpoint", "192.168.1.1")
        .await;
    assert!(!allowed);

    // Client2 should still be allowed
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client2, "/test/endpoint", "192.168.1.2")
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

    limiter
        .register_endpoint(
            "/endpoint1".to_string(),
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
            "/endpoint2".to_string(),
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

    let client = ClientIdentifier::IpAddress("192.168.1.1".to_string());

    // Exhaust endpoint1 limit
    for _ in 0..5 {
        let (allowed, _) = limiter
            .check_rate_limit_for_client(&client, "/endpoint1", "192.168.1.1")
            .await;
        assert!(allowed);
    }

    // Endpoint1 should be rate limited
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client, "/endpoint1", "192.168.1.1")
        .await;
    assert!(!allowed);

    // Endpoint2 should still be allowed (different limit)
    let (allowed, _) = limiter
        .check_rate_limit_for_client(&client, "/endpoint2", "192.168.1.1")
        .await;
    assert!(allowed);
}

#[tokio::test]
async fn test_rate_limit_info_includes_client_id() {
    let limiter = RateLimiter::new().await.unwrap();

    limiter
        .register_endpoint(
            "/test/endpoint".to_string(),
            RateLimitConfig::default(),
        )
        .await;

    let client = ClientIdentifier::ApiKey("test_key_123".to_string());

    let (_, info) = limiter
        .check_rate_limit_for_client(&client, "/test/endpoint", "192.168.1.1")
        .await;

    assert!(info.client_id.is_some());
    assert_eq!(info.client_id.unwrap(), "apikey:test_key_123");
}
