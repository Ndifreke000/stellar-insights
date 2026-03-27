use stellar_insights_backend::rpc::StellarRpcClient;

#[tokio::test]
async fn test_fetch_all_payments_mock() {
    let client = StellarRpcClient::new_with_defaults(true);

    // Test with custom limit
    let payments = client.fetch_all_payments(Some(50)).await.unwrap();
    assert_eq!(payments.len(), 50);
    assert!(!payments.is_empty());
    assert!(!payments[0].id.is_empty());
}

#[tokio::test]
async fn test_fetch_all_trades_mock() {
    let client = StellarRpcClient::new_with_defaults(true);

    // Test with custom limit
    let trades = client.fetch_all_trades(Some(30)).await.unwrap();
    assert_eq!(trades.len(), 30);
    assert!(!trades.is_empty());
    assert!(!trades[0].id.is_empty());
}

#[tokio::test]
async fn test_fetch_all_account_payments_mock() {
    let client = StellarRpcClient::new_with_defaults(true);
    let account_id = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

    // Test with custom limit
    let payments = client
        .fetch_all_account_payments(account_id, Some(100))
        .await
        .unwrap();
    assert_eq!(payments.len(), 100);
    assert!(!payments.is_empty());
    assert!(!payments[0].id.is_empty());
}

#[tokio::test]
async fn test_pagination_with_large_limit() {
    let client = StellarRpcClient::new_with_defaults(true);

    // Request a large number
    let payments = client.fetch_all_payments(Some(500)).await.unwrap();

    // In mock mode, we should get exactly what we asked for
    assert_eq!(payments.len(), 500);
}

#[tokio::test]
async fn test_pagination_with_default_limit() {
    let client = StellarRpcClient::new_with_defaults(true);

    // Test with None (should use configured default)
    let payments = client.fetch_all_payments(None).await.unwrap();

    // Should get a large number (default is 10000)
    assert!(payments.len() >= 1000);
}
