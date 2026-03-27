use stellar_insights_backend::network::{NetworkConfig, StellarNetwork};

#[test]
fn test_network_config_mainnet() {
    let config = NetworkConfig::for_network(StellarNetwork::Mainnet);

    assert_eq!(config.network, StellarNetwork::Mainnet);
    assert!(config.is_mainnet());
    assert!(!config.is_testnet());
    assert_eq!(config.display_name(), "Stellar Mainnet");
    assert_eq!(config.color(), "#00D4AA");
    assert_eq!(
        config.network_passphrase(),
        "Public Global Stellar Network ; September 2015"
    );
}

#[test]
fn test_network_config_testnet() {
    let config = NetworkConfig::for_network(StellarNetwork::Testnet);

    assert_eq!(config.network, StellarNetwork::Testnet);
    assert!(!config.is_mainnet());
    assert!(config.is_testnet());
    assert_eq!(config.display_name(), "Stellar Testnet");
    assert_eq!(config.color(), "#FF6B35");
    assert_eq!(
        config.network_passphrase(),
        "Test SDF Network ; September 2015"
    );
}

#[test]
fn test_network_parsing() {
    assert_eq!(
        "mainnet".parse::<StellarNetwork>().unwrap(),
        StellarNetwork::Mainnet
    );
    assert_eq!(
        "testnet".parse::<StellarNetwork>().unwrap(),
        StellarNetwork::Testnet
    );
    assert_eq!(
        "MAINNET".parse::<StellarNetwork>().unwrap(),
        StellarNetwork::Mainnet
    );
    assert_eq!(
        "TESTNET".parse::<StellarNetwork>().unwrap(),
        StellarNetwork::Testnet
    );

    assert!("invalid".parse::<StellarNetwork>().is_err());
    assert!("".parse::<StellarNetwork>().is_err());
}

#[test]
fn test_network_display() {
    assert_eq!(StellarNetwork::Mainnet.to_string(), "mainnet");
    assert_eq!(StellarNetwork::Testnet.to_string(), "testnet");
}

#[tokio::test]
async fn test_network_api_endpoints() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use stellar_insights_backend::api::network;
    use tower::ServiceExt;

    let app = network::routes();

    // Test get network info
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Test get available networks
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/available")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Test switch network
    let request_body = r#"{"network": "testnet"}"#;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/switch")
                .header("content-type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
