use chrono::Utc;
use sqlx::SqlitePool;
use stellar_insights_backend::rpc::{FeeBumpTransactionInfo, HorizonTransaction, InnerTransaction};
use stellar_insights_backend::services::fee_bump_tracker::FeeBumpTrackerService;

async fn setup_fee_bump_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ledgers (
            sequence INTEGER PRIMARY KEY,
            hash TEXT NOT NULL,
            close_time TEXT NOT NULL,
            transaction_count INTEGER DEFAULT 0,
            operation_count INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fee_bump_transactions (
            transaction_hash TEXT PRIMARY KEY,
            ledger_sequence INTEGER NOT NULL,
            fee_source TEXT NOT NULL,
            fee_charged INTEGER NOT NULL,
            max_fee INTEGER NOT NULL,
            inner_transaction_hash TEXT NOT NULL,
            inner_max_fee INTEGER NOT NULL,
            signatures_count INTEGER NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (ledger_sequence) REFERENCES ledgers(sequence)
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    for index_sql in [
        "CREATE INDEX IF NOT EXISTS idx_fee_bump_ledger_sequence ON fee_bump_transactions(ledger_sequence)",
        "CREATE INDEX IF NOT EXISTS idx_fee_bump_fee_source ON fee_bump_transactions(fee_source)",
        "CREATE INDEX IF NOT EXISTS idx_fee_bump_created_at ON fee_bump_transactions(created_at)",
    ] {
        sqlx::query(index_sql).execute(&pool).await.unwrap();
    }

    pool
}

#[tokio::test]
async fn test_fee_bump_tracker_process_transactions() {
    let pool = setup_fee_bump_pool().await;
    // Initialize service
    let service = FeeBumpTrackerService::new(pool.clone());

    // Create mock transactions
    let tx1 = HorizonTransaction {
        id: "tx1".to_string(),
        hash: "hash1".to_string(),
        ledger: 100,
        created_at: Utc::now().to_rfc3339(),
        source_account: "src1".to_string(),
        fee_account: Some("fee_src1".to_string()),
        fee_charged: Some("100".to_string()),
        max_fee: Some("1000".to_string()),
        operation_count: 1,
        successful: true,
        paging_token: "pt1".to_string(),
        fee_bump_transaction: Some(FeeBumpTransactionInfo {
            hash: "fb_hash1".to_string(),
            signatures: vec!["sig1".to_string()],
        }),
        inner_transaction: Some(InnerTransaction {
            hash: "inner_hash1".to_string(),
            max_fee: Some("500".to_string()),
            signatures: vec!["sig1".to_string()],
        }),
    };

    let tx2 = HorizonTransaction {
        id: "tx2".to_string(),
        hash: "hash2".to_string(),
        ledger: 100,
        created_at: Utc::now().to_rfc3339(),
        source_account: "src2".to_string(),
        fee_account: None,
        fee_charged: Some("200".to_string()),
        max_fee: Some("2000".to_string()),
        operation_count: 1,
        successful: true,
        paging_token: "pt2".to_string(),
        fee_bump_transaction: None,
        inner_transaction: None,
    };

    let transactions = vec![tx1, tx2];

    // Insert mock ledger to satisfy foreign key constraint
    sqlx::query("INSERT INTO ledgers (sequence, hash, close_time, transaction_count, operation_count) VALUES (100, 'ledger_hash', '2026-01-01T00:00:00Z', 0, 0)")
        .execute(&pool)
        .await
        .expect("Failed to insert mock ledger");

    // Process transactions
    let count = service.process_transactions(&transactions).await.unwrap();
    assert_eq!(count, 1);

    // Verify stored data
    let stored = service.get_recent_fee_bumps(10).await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].transaction_hash, "hash1");
    assert_eq!(stored[0].fee_source, "fee_src1");
    assert_eq!(stored[0].fee_charged, 100);
    assert_eq!(stored[0].max_fee, 1000);
    assert_eq!(stored[0].inner_max_fee, 500);

    // Verify stats
    let stats = service.get_fee_bump_stats().await.unwrap();
    assert_eq!(stats.total_fee_bumps, 1);
    assert_eq!(stats.avg_fee_charged, 100.0);
    assert_eq!(stats.max_fee_charged, 100);
    assert_eq!(stats.min_fee_charged, 100);
    assert_eq!(stats.unique_fee_sources, 1);
}
