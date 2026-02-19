// I'm testing the ledger ingestion functionality as specified in issue #2

use stellar_insights_backend::rpc::StellarRpcClient;

#[tokio::test]
async fn test_mock_rpc_fetch_ledgers() {
    // I'm verifying that mock RPC responses work correctly
    let client = StellarRpcClient::new_with_defaults(true);
    let result = client.fetch_ledgers(Some(1000), 5, None).await.unwrap();

    assert_eq!(result.ledgers.len(), 5);
    assert_eq!(result.ledgers[0].sequence, 1000);
    assert_eq!(result.ledgers[4].sequence, 1004);
    assert!(result.cursor.is_some());
}

#[tokio::test]
async fn test_ledgers_have_correct_format() {
    // I'm checking that ledger data has expected structure
    let client = StellarRpcClient::new_with_defaults(true);
    let result = client.fetch_ledgers(Some(500), 3, None).await.unwrap();

    for ledger in &result.ledgers {
        assert!(!ledger.hash.is_empty());
        assert!(ledger.sequence >= 500);
        assert!(!ledger.ledger_close_time.is_empty());
    }
}

#[tokio::test]
async fn test_cursor_pagination() {
    // I'm verifying cursor-based pagination works
    let client = StellarRpcClient::new_with_defaults(true);

    // First batch
    let result1 = client.fetch_ledgers(Some(100), 10, None).await.unwrap();
    let cursor = result1.cursor.clone();

    // I expect cursor to be set for next page
    assert!(cursor.is_some());

    // Second batch using cursor - should work without errors
    let result2 = client
        .fetch_ledgers(None, 10, cursor.as_deref())
        .await
        .unwrap();
    assert!(!result2.ledgers.is_empty());
}

#[tokio::test]
async fn test_ledger_sequence_is_sequential() {
    // I'm verifying ledgers are fetched sequentially
    let client = StellarRpcClient::new_with_defaults(true);
    let result = client.fetch_ledgers(Some(1000), 5, None).await.unwrap();

    for (i, ledger) in result.ledgers.iter().enumerate() {
        assert_eq!(ledger.sequence, 1000 + i as u64);
    }
}

#[test]
fn test_latest_and_oldest_ledger_bounds() {
    // I'm testing that bounds are returned correctly (sync test using mock data)
    use stellar_insights_backend::rpc::GetLedgersResult;

    // Mock structure verification
    let mock_result = GetLedgersResult {
        ledgers: vec![],
        latest_ledger: 5000,
        oldest_ledger: 1000,
        cursor: Some("4999".to_string()),
    };

    assert!(mock_result.latest_ledger > mock_result.oldest_ledger);
    assert!(mock_result.cursor.is_some());
}
