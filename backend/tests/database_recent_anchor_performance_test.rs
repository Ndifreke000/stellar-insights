use chrono::Utc;
use payraider_backend::database::Database;
use payraider_backend::models::PaymentRow;
use sqlx::SqlitePool;
use std::sync::Arc;

async fn setup_test_db() -> Arc<Database> {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    Arc::new(Database::new(pool))
}

#[tokio::test]
async fn test_get_recent_anchor_performance_from_payments_without_successful_column() {
    let db = setup_test_db().await;
    let anchor_account = "GCFX5UTRGXX5TQ6W2GNFX3YV7EP3RWMJZ3CXPQ7W7V4LETTTXX2TEST";
    let now = Utc::now();

    db.save_payments(vec![
        PaymentRow {
            id: "payment_1".to_string(),
            transaction_hash: "tx_1".to_string(),
            source_account: anchor_account.to_string(),
            destination_account: "GDESTINATIONACCOUNT000000000000000000000000000000000000001"
                .to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some(
                "GISSUERACCOUNT000000000000000000000000000000000000000000001".to_string(),
            ),
            source_asset_code: String::new(),
            source_asset_issuer: String::new(),
            destination_asset_code: String::new(),
            destination_asset_issuer: String::new(),
            amount: 10.0,
            successful: true,
            timestamp: None,
            submission_time: None,
            confirmation_time: None,
            created_at: now,
        },
        PaymentRow {
            id: "payment_2".to_string(),
            transaction_hash: "tx_2".to_string(),
            source_account: "GSOURCEACCOUNT000000000000000000000000000000000000000000001"
                .to_string(),
            destination_account: anchor_account.to_string(),
            asset_type: "credit_alphanum4".to_string(),
            asset_code: Some("USDC".to_string()),
            asset_issuer: Some(
                "GISSUERACCOUNT000000000000000000000000000000000000000000001".to_string(),
            ),
            source_asset_code: String::new(),
            source_asset_issuer: String::new(),
            destination_asset_code: String::new(),
            destination_asset_issuer: String::new(),
            amount: 12.5,
            successful: true,
            timestamp: None,
            submission_time: None,
            confirmation_time: None,
            created_at: now,
        },
    ])
    .await
    .unwrap();

    let metrics = db
        .get_recent_anchor_performance(anchor_account, 60)
        .await
        .unwrap();

    assert_eq!(metrics.total_transactions, 2);
    assert_eq!(metrics.successful_transactions, 2);
    assert_eq!(metrics.failed_transactions, 0);
    assert!(metrics.failure_rate.abs() < f64::EPSILON);
    assert!((metrics.success_rate - 100.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn test_anchor_health_scoring_and_stages() {
    use payraider_backend::models::AnchorMetrics;
    use payraider_backend::services::anchor_monitor::AnchorMonitor;

    let db = setup_test_db().await;

    // 1. Health scoring model
    let metrics = AnchorMetrics {
        success_rate: 99.0,
        failure_rate: 1.0,
        reliability_score: 98.0,
        total_transactions: 1000,
        successful_transactions: 990,
        failed_transactions: 10,
        avg_settlement_time_ms: Some(2500),
        status: payraider_backend::models::AnchorStatus::Green,
    };

    let score = AnchorMonitor::compute_health_score("anchor-1", &metrics, true, 1.0);
    assert_eq!(score.status, "Green");
    assert!(score.composite_score >= 80.0);
    assert_eq!(score.components.len(), 5);

    // 2. Stage latency recording and percentile extraction
    AnchorMonitor::record_stage_latency_for_db(
        &db,
        "anchor-1",
        "tx-sep-1",
        "SEP-24",
        Some(3000),
        Some(1200),
        Some(14000),
        "completed",
    )
    .await
    .unwrap();

    AnchorMonitor::record_stage_latency_for_db(
        &db,
        "anchor-1",
        "tx-sep-2",
        "SEP-24",
        Some(3500),
        Some(1500),
        Some(18000),
        "completed",
    )
    .await
    .unwrap();

    let stages = AnchorMonitor::get_settlement_stages_for_db(&db, "anchor-1")
        .await
        .unwrap();

    assert_eq!(stages.total_sample_count, 2);
    assert_eq!(stages.stages.len(), 3);
    assert!(stages.overall_p50_ms > 0.0);
}
