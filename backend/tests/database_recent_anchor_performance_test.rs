use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::models::PaymentRecord;

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
        PaymentRecord {
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
        PaymentRecord {
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
