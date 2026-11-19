//! Unit tests for the Stellar RPC client (moved out of `stellar.rs`).

#![allow(
    clippy::assertions_on_constants,
    clippy::branches_sharing_code,
    clippy::uninlined_format_args
)]
use super::*;
use crate::rpc::mock_stellar;
use anyhow::Context;

#[tokio::test]
async fn test_mock_health_check() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let health = client
        .check_health()
        .await
        .context("failed to check health in mock mode")?;

    assert_eq!(health.status, "healthy");
    assert!(health.latest_ledger > 0);
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_ledger() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let ledger = client
        .fetch_latest_ledger()
        .await
        .context("failed to fetch latest ledger in mock mode")?;

    assert!(ledger.sequence > 0);
    assert!(!ledger.hash.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_payments() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let payments = client
        .fetch_payments(5, None)
        .await
        .context("failed to fetch payments in mock mode")?;

    assert_eq!(payments.len(), 5);
    assert!(!payments[0].id.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_trades() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let trades = client
        .fetch_trades(3, None)
        .await
        .context("failed to fetch trades in mock mode")?;

    assert_eq!(trades.len(), 3);
    assert!(!trades[0].id.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_order_book() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);

    let selling = Asset {
        asset_type: "native".to_string(),
        asset_code: None,
        asset_issuer: None,
    };

    let buying = Asset {
        asset_type: "credit_alphanum4".to_string(),
        asset_code: Some("USDC".to_string()),
        asset_issuer: Some("GBXXXXXXX".to_string()),
    };

    let order_book = client
        .fetch_order_book(&selling, &buying, 10)
        .await
        .context("failed to fetch order book in mock mode")?;

    assert!(!order_book.bids.is_empty());
    assert!(!order_book.asks.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_liquidity_pools() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let pools = client
        .fetch_liquidity_pools(3, None)
        .await
        .context("failed to fetch liquidity pools in mock mode")?;

    assert_eq!(pools.len(), 3);
    assert!(!pools[0].id.is_empty());
    assert_eq!(pools[0].reserves.len(), 2);
    assert_eq!(pools[0].fee_bp, 30);
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_single_liquidity_pool() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let pool = client
        .fetch_liquidity_pool("test_pool_id")
        .await
        .context("failed to fetch liquidity pool in mock mode")?;

    assert_eq!(pool.id, "test_pool_id");
    assert_eq!(pool.reserves.len(), 2);
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_pool_trades() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let trades = client
        .fetch_pool_trades("test_pool_id", 5)
        .await
        .context("failed to fetch pool trades in mock mode")?;

    assert_eq!(trades.len(), 5);
    assert!(!trades[0].id.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_operations_for_ledger() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let operations = client
        .fetch_operations_for_ledger(123)
        .await
        .context("failed to fetch operations for ledger in mock mode")?;

    assert_eq!(operations.len(), 3);
    assert_eq!(operations[0].operation_type, "account_merge");
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_operation_effects() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let effects = client
        .fetch_operation_effects("op_123_0")
        .await
        .context("failed to fetch operation effects in mock mode")?;

    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect_type, "account_credited");
    Ok(())
}

#[tokio::test]
async fn test_mock_fetch_ledgers_stops_at_latest() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let result = client
        .fetch_ledgers(
            Some(mock_stellar::MOCK_LATEST_LEDGER.saturating_add(1)),
            5,
            None,
        )
        .await
        .context("failed to fetch ledgers in mock mode")?;

    assert!(result.ledgers.is_empty());
    assert_eq!(result.latest_ledger, mock_stellar::MOCK_LATEST_LEDGER);
    Ok(())
}

#[tokio::test]
async fn test_pagination_config_defaults() {
    let client = StellarRpcClient::new_with_defaults(true);

    // Verify default pagination config is loaded with security limits
    assert_eq!(
        client.max_records_per_request,
        DEFAULT_MAX_RECORDS_PER_REQUEST
    );
    assert_eq!(client.max_total_records, DEFAULT_MAX_TOTAL_RECORDS);
    assert_eq!(client.pagination_delay_ms, DEFAULT_PAGINATION_DELAY_MS);
}

#[tokio::test]
async fn test_fetch_all_payments_mock() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);

    // Test with custom limit
    let payments = client
        .fetch_all_payments(Some(50))
        .await
        .context("failed to fetch all payments (custom limit) in mock mode")?;
    assert_eq!(payments.len(), 50);

    // Test with default limit (should use max_total_records)
    let payments = client
        .fetch_all_payments(None)
        .await
        .context("failed to fetch all payments (default limit) in mock mode")?;
    assert_eq!(payments.len(), client.max_total_records as usize);
    Ok(())
}

#[tokio::test]
async fn test_fetch_all_trades_mock() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);

    // Test with custom limit
    let trades = client
        .fetch_all_trades(Some(30))
        .await
        .context("failed to fetch all trades (custom limit) in mock mode")?;
    assert_eq!(trades.len(), 30);

    // Test with default limit
    let trades = client
        .fetch_all_trades(None)
        .await
        .context("failed to fetch all trades (default limit) in mock mode")?;
    assert_eq!(trades.len(), client.max_total_records as usize);
    Ok(())
}

#[tokio::test]
async fn test_fetch_all_account_payments_mock() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);
    let account_id = "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";

    // Test with custom limit
    let payments = client
        .fetch_all_account_payments(account_id, Some(100))
        .await
        .context("failed to fetch all account payments (custom limit) in mock mode")?;
    assert_eq!(payments.len(), 100);

    // Test with default limit
    let payments = client
        .fetch_all_account_payments(account_id, None)
        .await
        .context("failed to fetch all account payments (default limit) in mock mode")?;
    assert_eq!(payments.len(), client.max_total_records as usize);
    Ok(())
}

#[tokio::test]
async fn test_pagination_respects_max_records() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);

    // Request more than available, should stop when no more data
    let payments = client
        .fetch_all_payments(Some(500))
        .await
        .context("failed to fetch all payments in mock mode")?;

    // In mock mode, we should get exactly what we asked for
    assert_eq!(payments.len(), 500);
    Ok(())
}

#[tokio::test]
async fn test_dos_protection_caps_total_records() -> Result<()> {
    let client = StellarRpcClient::new_with_defaults(true);

    // Try to fetch more than the hard limit
    // Should cap at ABSOLUTE_MAX_TOTAL_RECORDS
    let payments = client
        .fetch_all_payments(Some(ABSOLUTE_MAX_TOTAL_RECORDS * 2))
        .await
        .context("failed to fetch all payments (DoS protection test) in mock mode")?;

    // Should not exceed hard limit
    assert!(payments.len() <= ABSOLUTE_MAX_TOTAL_RECORDS as usize);
    Ok(())
}

#[tokio::test]
async fn test_dos_protection_caps_per_request() {
    // Verify that per-request limits are capped
    // This is checked during client initialization
    assert!(DEFAULT_MAX_RECORDS_PER_REQUEST <= ABSOLUTE_MAX_RECORDS_PER_REQUEST);
    assert!(DEFAULT_MAX_TOTAL_RECORDS <= ABSOLUTE_MAX_TOTAL_RECORDS);
    assert!(DEFAULT_PAGINATION_DELAY_MS >= MIN_PAGINATION_DELAY_MS);
}

// ============================================================================
// Issue #188 – AssetBalanceChange / new Horizon API format tests
// ============================================================================

#[test]
fn test_legacy_payment_format_returns_direct_fields() {
    let payment = Payment {
        id: "1".into(),
        paging_token: "pt".into(),
        transaction_hash: "tx".into(),
        source_account: "GSRC".into(),
        destination: "GDEST".into(),
        asset_type: "credit_alphanum4".into(),
        asset_code: Some("USDC".into()),
        asset_issuer: Some("GISSUER".into()),
        amount: "500.0000000".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        operation_type: Some("payment".into()),
        source_asset_type: None,
        source_asset_code: None,
        source_asset_issuer: None,
        source_amount: None,
        from: Some("GSRC".into()),
        to: Some("GDEST".into()),
        asset_balance_changes: None,
        transaction_successful: None,
    };

    assert_eq!(payment.get_destination(), Some("GDEST".to_string()));
    assert_eq!(payment.get_amount(), "500.0000000");
    assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
    assert_eq!(payment.get_asset_issuer(), Some("GISSUER".to_string()));
}

#[test]
fn test_new_format_takes_priority_over_legacy() {
    let payment = Payment {
        transaction_successful: None,
        id: "2".into(),
        paging_token: "pt".into(),
        transaction_hash: "tx".into(),
        source_account: "GSRC".into(),
        // Legacy fields are empty – just like the real new Horizon response.
        destination: String::new(),
        asset_type: "credit_alphanum4".into(),
        asset_code: None,
        asset_issuer: None,
        amount: String::new(),
        created_at: "2026-01-01T00:00:00Z".into(),
        operation_type: Some("payment".into()),
        source_asset_type: None,
        source_asset_code: None,
        source_asset_issuer: None,
        source_amount: None,
        from: None,
        to: None,
        asset_balance_changes: Some(vec![AssetBalanceChange {
            asset_type: "credit_alphanum4".into(),
            asset_code: Some("USDC".into()),
            asset_issuer: Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".into()),
            change_type: "transfer".into(),
            from: Some("GSRC".into()),
            to: Some("GDEST_NEW".into()),
            amount: "999.0000000".into(),
        }]),
    };

    assert_eq!(payment.get_destination(), Some("GDEST_NEW".to_string()));
    assert_eq!(payment.get_amount(), "999.0000000");
    assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
    assert_eq!(
        payment.get_asset_issuer(),
        Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string())
    );
}

#[test]
fn failed_payments_are_detected_from_horizon_records() {
    let record = |flag: &str| {
        serde_json::from_str::<Payment>(&format!(
            r#"{{"id":"1","paging_token":"1","transaction_hash":"h","source_account":"GA",
                "asset_type":"native","amount":"1.0","created_at":"2026-01-01T00:00:00Z"{flag}}}"#
        ))
        .expect("payment record")
    };

    assert!(!record(r#","transaction_successful":false"#).succeeded());
    assert!(record(r#","transaction_successful":true"#).succeeded());
    // Records without the flag (sources that only list successes) count as successful.
    assert!(record("").succeeded());
}

#[test]
fn test_new_format_overrides_when_both_present() {
    // When BOTH legacy and new fields are present, new format wins.
    let payment = Payment {
        transaction_successful: None,
        id: "3".into(),
        paging_token: "pt".into(),
        transaction_hash: "tx".into(),
        source_account: "GSRC".into(),
        destination: "GDEST_LEGACY".into(),
        asset_type: "credit_alphanum4".into(),
        asset_code: Some("OLD_CODE".into()),
        asset_issuer: Some("OLD_ISSUER".into()),
        amount: "111.0000000".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        operation_type: Some("payment".into()),
        source_asset_type: None,
        source_asset_code: None,
        source_asset_issuer: None,
        source_amount: None,
        from: Some("GSRC".into()),
        to: Some("GDEST_LEGACY".into()),
        asset_balance_changes: Some(vec![AssetBalanceChange {
            asset_type: "credit_alphanum4".into(),
            asset_code: Some("NEW_CODE".into()),
            asset_issuer: Some("NEW_ISSUER".into()),
            change_type: "transfer".into(),
            from: Some("GSRC".into()),
            to: Some("GDEST_NEW".into()),
            amount: "222.0000000".into(),
        }]),
    };

    // New format takes precedence
    assert_eq!(payment.get_destination(), Some("GDEST_NEW".to_string()));
    assert_eq!(payment.get_amount(), "222.0000000");
    assert_eq!(payment.get_asset_code(), Some("NEW_CODE".to_string()));
    assert_eq!(payment.get_asset_issuer(), Some("NEW_ISSUER".to_string()));
}

#[test]
fn test_native_asset_via_new_format() {
    let payment = Payment {
        transaction_successful: None,
        id: "4".into(),
        paging_token: "pt".into(),
        transaction_hash: "tx".into(),
        source_account: "GSRC".into(),
        destination: String::new(),
        asset_type: "native".into(),
        asset_code: None,
        asset_issuer: None,
        amount: String::new(),
        created_at: "2026-01-01T00:00:00Z".into(),
        operation_type: Some("payment".into()),
        source_asset_type: None,
        source_asset_code: None,
        source_asset_issuer: None,
        source_amount: None,
        from: None,
        to: None,
        asset_balance_changes: Some(vec![AssetBalanceChange {
            asset_type: "native".into(),
            asset_code: None,
            asset_issuer: None,
            change_type: "transfer".into(),
            from: Some("GSRC".into()),
            to: Some("GDEST".into()),
            amount: "50.0000000".into(),
        }]),
    };

    assert_eq!(payment.get_destination(), Some("GDEST".to_string()));
    assert_eq!(payment.get_amount(), "50.0000000");
    assert_eq!(payment.get_asset_code(), None);
    assert_eq!(payment.get_asset_issuer(), None);
}

#[test]
fn test_fallback_to_to_field_when_destination_empty() {
    // Legacy format where `destination` is empty but `to` is set.
    let payment = Payment {
        id: "5".into(),
        paging_token: "pt".into(),
        transaction_hash: "tx".into(),
        source_account: "GSRC".into(),
        destination: String::new(),
        asset_type: "credit_alphanum4".into(),
        asset_code: Some("USDC".into()),
        asset_issuer: Some("GISSUER".into()),
        amount: "100.0000000".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        operation_type: Some("payment".into()),
        source_asset_type: None,
        source_asset_code: None,
        source_asset_issuer: None,
        source_amount: None,
        from: Some("GSRC".into()),
        to: Some("GTO_FIELD".into()),
        asset_balance_changes: None,
        transaction_successful: None,
    };

    assert_eq!(payment.get_destination(), Some("GTO_FIELD".to_string()));
}

#[test]
fn test_deserialization_new_format() -> Result<()> {
    let json = r#"{
        "id": "op_new",
        "paging_token": "pt_new",
        "transaction_hash": "txhash_new",
        "source_account": "GSRC",
        "asset_type": "credit_alphanum4",
        "amount": "",
        "created_at": "2026-01-22T10:00:00Z",
        "asset_balance_changes": [
            {
                "asset_type": "credit_alphanum4",
                "asset_code": "USDC",
                "asset_issuer": "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
                "type": "transfer",
                "from": "GSRC",
                "to": "GDEST",
                "amount": "250.0000000"
            }
        ]
    }"#;

    let payment: Payment =
        serde_json::from_str(json).context("failed to deserialize new format payment")?;
    assert_eq!(payment.get_destination(), Some("GDEST".to_string()));
    assert_eq!(payment.get_amount(), "250.0000000");
    assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
    assert_eq!(
        payment.get_asset_issuer(),
        Some("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string())
    );
    Ok(())
}

#[test]
fn test_deserialization_legacy_format() -> Result<()> {
    let json = r#"{
        "id": "op_legacy",
        "paging_token": "pt_legacy",
        "transaction_hash": "txhash_legacy",
        "source_account": "GSRC",
        "destination": "GDEST_LEGACY",
        "asset_type": "credit_alphanum4",
        "asset_code": "USDC",
        "asset_issuer": "GISSUER",
        "amount": "100.0000000",
        "created_at": "2026-01-22T10:00:00Z",
        "type": "payment"
    }"#;

    let payment: Payment =
        serde_json::from_str(json).context("failed to deserialize legacy payment format")?;
    assert!(payment.asset_balance_changes.is_none());
    assert_eq!(payment.get_destination(), Some("GDEST_LEGACY".to_string()));
    assert_eq!(payment.get_amount(), "100.0000000");
    assert_eq!(payment.get_asset_code(), Some("USDC".to_string()));
    assert_eq!(payment.get_asset_issuer(), Some("GISSUER".to_string()));
    Ok(())
}

#[test]
fn test_mock_payments_include_new_format() {
    let payments = mock_stellar::mock_payments(10);
    assert_eq!(payments.len(), 10);

    // Even-indexed payments should have asset_balance_changes populated
    for (i, p) in payments.iter().enumerate() {
        if i % 2 == 0 {
            assert!(
                p.asset_balance_changes.is_some(),
                "payment[{}] should have asset_balance_changes",
                i
            );
            // Safe to unwrap here since we just asserted .is_some() above
            let changes = p
                .asset_balance_changes
                .as_ref()
                .expect("verified is_some above");
            assert_eq!(changes.len(), 1);
            assert_eq!(changes[0].change_type, "transfer");
            // Verify helper methods return the new-format values
            assert!(!p.get_amount().is_empty());
            assert!(p.get_destination().is_some());
        } else {
            assert!(
                p.asset_balance_changes.is_none(),
                "payment[{}] should NOT have asset_balance_changes",
                i
            );
            // Verify legacy fields still work
            assert!(!p.get_amount().is_empty());
            assert!(p.get_destination().is_some());
        }
    }
}
