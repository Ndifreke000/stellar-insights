use stellar_insights_backend::services::price_feed::{
    default_asset_mapping, PriceFeedClient, PriceFeedConfig,
};

#[tokio::test]
async fn test_price_feed_client_creation() {
    let config = PriceFeedConfig::default();
    let mapping = default_asset_mapping();
    let client = PriceFeedClient::new(config, mapping);

    // Check cache stats on new client
    let (total, fresh) = client.cache_stats().await;
    assert_eq!(total, 0);
    assert_eq!(fresh, 0);
}

#[tokio::test]
async fn test_asset_mapping_contains_xlm() {
    let mapping = default_asset_mapping();

    // Verify XLM mappings exist
    assert!(mapping.contains_key("XLM:native"));
    assert!(mapping.contains_key("native"));
    assert_eq!(mapping.get("XLM:native").unwrap(), "stellar");
}

#[tokio::test]
async fn test_asset_mapping_contains_usdc() {
    let mapping = default_asset_mapping();

    // Verify USDC mapping exists
    let usdc_key = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";
    assert!(mapping.contains_key(usdc_key));
    assert_eq!(mapping.get(usdc_key).unwrap(), "usd-coin");
}

#[tokio::test]
async fn test_cache_clear() {
    let config = PriceFeedConfig::default();
    let mapping = default_asset_mapping();
    let client = PriceFeedClient::new(config, mapping);

    // Clear cache
    client.clear_cache().await;

    // Verify cache is empty
    let (total, fresh) = client.cache_stats().await;
    assert_eq!(total, 0);
    assert_eq!(fresh, 0);
}

#[tokio::test]
async fn test_config_from_env() {
    // Set environment variables
    std::env::set_var("PRICE_FEED_PROVIDER", "coingecko");
    std::env::set_var("PRICE_FEED_CACHE_TTL_SECONDS", "600");
    std::env::set_var("PRICE_FEED_REQUEST_TIMEOUT_SECONDS", "15");

    let config = PriceFeedConfig::from_env();

    assert_eq!(config.provider, "coingecko");
    assert_eq!(config.cache_ttl_seconds, 600);
    assert_eq!(config.request_timeout_seconds, 15);

    // Clean up
    std::env::remove_var("PRICE_FEED_PROVIDER");
    std::env::remove_var("PRICE_FEED_CACHE_TTL_SECONDS");
    std::env::remove_var("PRICE_FEED_REQUEST_TIMEOUT_SECONDS");
}

#[tokio::test]
async fn test_multiple_asset_mappings() {
    let mapping = default_asset_mapping();

    // Verify multiple assets are mapped
    assert!(mapping.len() >= 8, "Should have at least 8 asset mappings");

    // Check for various asset types
    assert!(mapping.contains_key("XLM:native")); // Native
    assert!(mapping.contains_key("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")); // Stablecoin
    assert!(mapping.contains_key("EURC:GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP2"));
    // Euro stablecoin
}

// Note: The following tests require actual API calls and should only be run with network access
// They are commented out to avoid CI failures

/*
#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_fetch_xlm_price_live() {
    let config = PriceFeedConfig::default();
    let mapping = default_asset_mapping();
    let client = PriceFeedClient::new(config, mapping);

    match client.get_price("XLM:native").await {
        Ok(price) => {
            assert!(price > 0.0, "XLM price should be positive");
            println!("XLM price: ${}", price);
        }
        Err(e) => {
            println!("Failed to fetch XLM price (this is expected without API access): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_convert_to_usd_live() {
    let config = PriceFeedConfig::default();
    let mapping = default_asset_mapping();
    let client = PriceFeedClient::new(config, mapping);

    match client.convert_to_usd("XLM:native", 100.0).await {
        Ok(usd_amount) => {
            assert!(usd_amount > 0.0, "USD amount should be positive");
            println!("100 XLM = ${}", usd_amount);
        }
        Err(e) => {
            println!("Failed to convert (this is expected without API access): {}", e);
        }
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_batch_price_fetch_live() {
    let config = PriceFeedConfig::default();
    let mapping = default_asset_mapping();
    let client = PriceFeedClient::new(config, mapping);

    let assets = vec![
        "XLM:native".to_string(),
        "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
    ];

    let prices = client.get_prices(&assets).await;

    if !prices.is_empty() {
        println!("Fetched {} prices", prices.len());
        for (asset, price) in prices {
            println!("{}: ${}", asset, price);
        }
    } else {
        println!("No prices fetched (this is expected without API access)");
    }
}
*/
