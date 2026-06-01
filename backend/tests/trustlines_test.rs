use sqlx::SqlitePool;
use std::sync::Arc;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::trustline_analyzer::TrustlineAnalyzer;

#[sqlx::test]
async fn test_trustlines_sync_and_query(pool: SqlitePool) {
    // Create a mock RPC client
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let analyzer = TrustlineAnalyzer::new(pool.clone(), rpc_client);

    // Sync assets from mock Horizon data
    let count = analyzer.sync_assets().await.unwrap();
    assert_eq!(count, 4); // Mock data returns 4 assets

    // Verify metrics
    let stats = analyzer.get_metrics().await.unwrap();
    assert_eq!(stats.total_assets_tracked, 4);
    assert!(stats.total_trustlines_across_network > 0);

    // Verify rankings
    let rankings = analyzer.get_trustline_rankings(5).await.unwrap();
    assert_eq!(rankings.len(), 4);
    assert_eq!(rankings[0].asset_code, "USDC"); // USDC has the most trustlines in mock
}

#[sqlx::test]
async fn test_trustlines_snapshots(pool: SqlitePool) {
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let analyzer = TrustlineAnalyzer::new(pool.clone(), rpc_client);

    // Sync and snapshot
    analyzer.sync_assets().await.unwrap();
    let snap_count = analyzer.take_snapshots().await.unwrap();
    assert_eq!(snap_count, 4);

    let rankings = analyzer.get_trustline_rankings(5).await.unwrap();
    let asset = &rankings[0];

    let history = analyzer
        .get_asset_history(&asset.asset_code, &asset.asset_issuer, 10)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].asset_code, asset.asset_code);
    assert_eq!(history[0].total_trustlines, asset.total_trustlines);
}
