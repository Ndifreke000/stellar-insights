use sqlx::SqlitePool;
use std::sync::Arc;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::liquidity_pool_analyzer::LiquidityPoolAnalyzer;

async fn setup_liquidity_pool_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS liquidity_pools (
            pool_id TEXT PRIMARY KEY,
            pool_type TEXT NOT NULL DEFAULT 'constant_product',
            fee_bp INTEGER NOT NULL DEFAULT 30,
            total_trustlines INTEGER NOT NULL DEFAULT 0,
            total_shares TEXT NOT NULL DEFAULT '0',
            reserve_a_asset_code TEXT NOT NULL,
            reserve_a_asset_issuer TEXT,
            reserve_a_amount REAL NOT NULL DEFAULT 0.0,
            reserve_b_asset_code TEXT NOT NULL,
            reserve_b_asset_issuer TEXT,
            reserve_b_amount REAL NOT NULL DEFAULT 0.0,
            total_value_usd REAL NOT NULL DEFAULT 0.0,
            volume_24h_usd REAL NOT NULL DEFAULT 0.0,
            fees_earned_24h_usd REAL NOT NULL DEFAULT 0.0,
            apy REAL NOT NULL DEFAULT 0.0,
            impermanent_loss_pct REAL NOT NULL DEFAULT 0.0,
            trade_count_24h INTEGER NOT NULL DEFAULT 0,
            last_synced_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS liquidity_pool_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pool_id TEXT NOT NULL,
            reserve_a_amount REAL NOT NULL,
            reserve_b_amount REAL NOT NULL,
            total_value_usd REAL NOT NULL DEFAULT 0.0,
            volume_usd REAL NOT NULL DEFAULT 0.0,
            fees_usd REAL NOT NULL DEFAULT 0.0,
            apy REAL NOT NULL DEFAULT 0.0,
            impermanent_loss_pct REAL NOT NULL DEFAULT 0.0,
            trade_count INTEGER NOT NULL DEFAULT 0,
            snapshot_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (pool_id) REFERENCES liquidity_pools(pool_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    for index_sql in [
        "CREATE INDEX IF NOT EXISTS idx_lp_apy ON liquidity_pools(apy DESC)",
        "CREATE INDEX IF NOT EXISTS idx_lp_volume ON liquidity_pools(volume_24h_usd DESC)",
        "CREATE INDEX IF NOT EXISTS idx_lp_total_value ON liquidity_pools(total_value_usd DESC)",
        "CREATE INDEX IF NOT EXISTS idx_lp_updated ON liquidity_pools(updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_lps_pool_id ON liquidity_pool_snapshots(pool_id)",
        "CREATE INDEX IF NOT EXISTS idx_lps_snapshot_at ON liquidity_pool_snapshots(snapshot_at)",
        "CREATE INDEX IF NOT EXISTS idx_lps_pool_time ON liquidity_pool_snapshots(pool_id, snapshot_at)",
    ] {
        sqlx::query(index_sql).execute(&pool).await.unwrap();
    }

    pool
}

#[tokio::test]
async fn test_liquidity_pool_sync_and_query() {
    let pool = setup_liquidity_pool_test_db().await;
    // Create a mock RPC client
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let analyzer = LiquidityPoolAnalyzer::new(pool.clone(), rpc_client);

    // Sync pools from mock Horizon data
    let count = analyzer.sync_pools().await.unwrap();
    assert_eq!(count, 5); // Mock returns 5 pools

    // Verify all pools are stored
    let pools = analyzer.get_all_pools().await.unwrap();
    assert_eq!(pools.len(), 5);

    // Verify first pool has correct data
    let first_pool = &pools[0];
    assert!(!first_pool.pool_id.is_empty());
    assert_eq!(first_pool.pool_type, "constant_product");
    assert_eq!(first_pool.fee_bp, 30);
    assert!(first_pool.reserve_a_amount > 0.0);
    assert!(first_pool.reserve_b_amount > 0.0);

    // Verify pool stats
    let stats = analyzer.get_pool_stats().await.unwrap();
    assert_eq!(stats.total_pools, 5);
    assert!(stats.total_value_locked_usd > 0.0);
}

#[tokio::test]
async fn test_liquidity_pool_rankings() {
    let pool = setup_liquidity_pool_test_db().await;
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let analyzer = LiquidityPoolAnalyzer::new(pool.clone(), rpc_client);

    // Sync first
    analyzer.sync_pools().await.unwrap();

    // Test different ranking sorts
    let by_apy = analyzer.get_pool_rankings("apy", 3).await.unwrap();
    assert_eq!(by_apy.len(), 3);

    let by_volume = analyzer.get_pool_rankings("volume", 5).await.unwrap();
    assert_eq!(by_volume.len(), 5);

    let by_tvl = analyzer.get_pool_rankings("tvl", 2).await.unwrap();
    assert_eq!(by_tvl.len(), 2);
    // TVL should be in descending order
    assert!(by_tvl[0].total_value_usd >= by_tvl[1].total_value_usd);
}

#[tokio::test]
async fn test_liquidity_pool_snapshots() {
    let pool = setup_liquidity_pool_test_db().await;
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let analyzer = LiquidityPoolAnalyzer::new(pool.clone(), rpc_client);

    // Sync pools first
    analyzer.sync_pools().await.unwrap();

    // Take snapshots
    let snap_count = analyzer.take_snapshots().await.unwrap();
    assert_eq!(snap_count, 5);

    // Retrieve snapshots for a pool
    let pools = analyzer.get_all_pools().await.unwrap();
    let pool_id = &pools[0].pool_id;
    let snapshots = analyzer.get_pool_snapshots(pool_id, 10).await.unwrap();
    assert_eq!(snapshots.len(), 1); // One snapshot taken

    assert_eq!(snapshots[0].pool_id, *pool_id);
    assert!(snapshots[0].total_value_usd > 0.0);
}

#[tokio::test]
async fn test_liquidity_pool_detail() {
    let pool = setup_liquidity_pool_test_db().await;
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let analyzer = LiquidityPoolAnalyzer::new(pool.clone(), rpc_client);

    // Sync and snapshot
    analyzer.sync_pools().await.unwrap();
    analyzer.take_snapshots().await.unwrap();

    let pools = analyzer.get_all_pools().await.unwrap();
    let pool_id = &pools[0].pool_id;

    // Get pool detail
    let (detail, snapshots) = analyzer.get_pool_detail(pool_id).await.unwrap();
    assert_eq!(detail.pool_id, *pool_id);
    assert_eq!(snapshots.len(), 1);
}

#[test]
fn test_impermanent_loss_computation() {
    // No price change => zero IL
    let il = LiquidityPoolAnalyzer::compute_impermanent_loss(100.0, 100.0, 100.0, 100.0);
    assert!((il - 0.0).abs() < 0.001);

    // 2x price change => ~5.72% IL
    let il = LiquidityPoolAnalyzer::compute_impermanent_loss(100.0, 100.0, 141.421, 70.710);
    assert!(il > 5.0 && il < 6.0, "IL was {} but expected ~5.72%", il);

    // 4x price change => ~20.0% IL
    let il = LiquidityPoolAnalyzer::compute_impermanent_loss(100.0, 100.0, 200.0, 50.0);
    assert!(il > 19.9 && il < 20.1, "IL was {} but expected ~20.0%", il);

    // Edge case: zero values
    let il = LiquidityPoolAnalyzer::compute_impermanent_loss(0.0, 100.0, 100.0, 100.0);
    assert_eq!(il, 0.0);
}
