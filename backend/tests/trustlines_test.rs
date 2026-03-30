use sqlx::SqlitePool;
use std::sync::Arc;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::trustline_analyzer::TrustlineAnalyzer;

async fn setup_trustline_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trustline_stats (
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_trustlines INTEGER NOT NULL DEFAULT 0,
            authorized_trustlines INTEGER NOT NULL DEFAULT 0,
            unauthorized_trustlines INTEGER NOT NULL DEFAULT 0,
            total_supply REAL NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (asset_code, asset_issuer)
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trustline_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_trustlines INTEGER NOT NULL,
            authorized_trustlines INTEGER NOT NULL,
            unauthorized_trustlines INTEGER NOT NULL,
            total_supply REAL NOT NULL,
            snapshot_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_trustline_snapshots_asset_time ON trustline_snapshots(asset_code, asset_issuer, snapshot_at DESC)",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[tokio::test]
async fn test_trustlines_sync_and_query() {
    let pool = setup_trustline_test_db().await;
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

#[tokio::test]
async fn test_trustlines_snapshots() {
    let pool = setup_trustline_test_db().await;
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
