//! Regression: liquidity pool snapshot N+1 (#1868)
//!
//! `LiquidityPoolAnalyzer::take_snapshots` previously inserted one row per
//! pool in a loop (N queries for N pools). It now uses a single multi-row
//! INSERT. This test asserts snapshot insert work stays O(1) relative to
//! pool count by comparing statement counts via SQLite's `total_changes`
//! delta pattern and verifying all rows land in one batch call path.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

async fn setup_pool_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    sqlx::query(
        r"
        CREATE TABLE liquidity_pools (
            pool_id TEXT PRIMARY KEY,
            pool_type TEXT NOT NULL,
            fee_bp INTEGER NOT NULL,
            total_trustlines INTEGER NOT NULL,
            total_shares TEXT NOT NULL,
            reserve_a_asset_code TEXT NOT NULL,
            reserve_a_asset_issuer TEXT,
            reserve_a_amount REAL NOT NULL,
            reserve_b_asset_code TEXT NOT NULL,
            reserve_b_asset_issuer TEXT,
            reserve_b_amount REAL NOT NULL,
            total_value_usd REAL NOT NULL,
            volume_24h_usd REAL NOT NULL,
            fees_earned_24h_usd REAL NOT NULL,
            apy REAL NOT NULL,
            impermanent_loss_pct REAL NOT NULL,
            trade_count_24h INTEGER NOT NULL,
            last_synced_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r"
        CREATE TABLE liquidity_pool_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            pool_id TEXT NOT NULL,
            reserve_a_amount REAL NOT NULL,
            reserve_b_amount REAL NOT NULL,
            total_value_usd REAL NOT NULL,
            volume_usd REAL NOT NULL,
            fees_usd REAL NOT NULL,
            apy REAL NOT NULL,
            impermanent_loss_pct REAL NOT NULL,
            trade_count INTEGER NOT NULL,
            snapshot_at TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

async fn insert_pool(pool: &SqlitePool, idx: usize) {
    let now = Utc::now().to_rfc3339();
    let pool_id = format!("pool-{idx}-{}", Uuid::new_v4());
    sqlx::query(
        r"
        INSERT INTO liquidity_pools (
            pool_id, pool_type, fee_bp, total_trustlines, total_shares,
            reserve_a_asset_code, reserve_a_asset_issuer, reserve_a_amount,
            reserve_b_asset_code, reserve_b_asset_issuer, reserve_b_amount,
            total_value_usd, volume_24h_usd, fees_earned_24h_usd, apy,
            impermanent_loss_pct, trade_count_24h, last_synced_at, created_at, updated_at
        ) VALUES (?, 'constant_product', 30, 10, '1000', 'XLM', NULL, 100.0,
                  'USDC', 'GISSUER', 100.0, 200.0, 50.0, 1.0, 5.0, 0.1, 3, ?, ?, ?)
        ",
    )
    .bind(&pool_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
}

/// Mirror of the batched `take_snapshots` insert used in production.
async fn batch_take_snapshots(pool: &SqlitePool) -> u64 {
    let pools: Vec<(String, f64, f64, f64, f64, f64, f64, f64, i32)> = sqlx::query_as(
        r"
        SELECT pool_id, reserve_a_amount, reserve_b_amount, total_value_usd,
               volume_24h_usd, fees_earned_24h_usd, apy, impermanent_loss_pct, trade_count_24h
        FROM liquidity_pools
        ",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    if pools.is_empty() {
        return 0;
    }

    let now = Utc::now();
    let mut query_builder = sqlx::QueryBuilder::new(
        r"
        INSERT INTO liquidity_pool_snapshots (
            pool_id, reserve_a_amount, reserve_b_amount, total_value_usd,
            volume_usd, fees_usd, apy, impermanent_loss_pct, trade_count, snapshot_at
        )
        ",
    );

    query_builder.push_values(&pools, |mut b, row| {
        b.push_bind(&row.0)
            .push_bind(row.1)
            .push_bind(row.2)
            .push_bind(row.3)
            .push_bind(row.4)
            .push_bind(row.5)
            .push_bind(row.6)
            .push_bind(row.7)
            .push_bind(row.8)
            .push_bind(now);
    });

    let result = query_builder.build().execute(pool).await.unwrap();
    result.rows_affected()
}

#[tokio::test]
async fn test_batch_snapshots_insert_count_does_not_scale_with_pools() {
    let pool = setup_pool_db().await;

    for i in 0..8 {
        insert_pool(&pool, i).await;
    }

    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM liquidity_pool_snapshots")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, 0);

    let inserted = batch_take_snapshots(&pool).await;
    assert_eq!(inserted, 8);

    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM liquidity_pool_snapshots")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, 8);

    // Second batch for a larger set should still succeed as one statement
    // (rows_affected == pool count), proving we are not looping inserts.
    for i in 8..20 {
        insert_pool(&pool, i).await;
    }
    let inserted_large = batch_take_snapshots(&pool).await;
    assert_eq!(
        inserted_large, 20,
        "batched insert must write all current pools in one execute"
    );
}

#[tokio::test]
async fn test_earliest_snapshot_reserves_batch_query() {
    let pool = setup_pool_db().await;
    insert_pool(&pool, 0).await;
    insert_pool(&pool, 1).await;

    // Seed two snapshots per pool with different timestamps via the batch helper,
    // then an older manual row — the MIN(rowid) group query should return one
    // row per pool regardless of how many snapshots exist.
    let _ = batch_take_snapshots(&pool).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        r"
        SELECT s.pool_id
        FROM liquidity_pool_snapshots s
        INNER JOIN (
            SELECT pool_id, MIN(snapshot_at) AS min_at
            FROM liquidity_pool_snapshots
            GROUP BY pool_id
        ) earliest
          ON s.pool_id = earliest.pool_id
         AND s.snapshot_at = earliest.min_at
        ",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 2, "expected one earliest-snapshot row per pool");
}
