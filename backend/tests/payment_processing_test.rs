use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;
use stellar_insights_backend::database::Database;
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::aggregation::{AggregationConfig, AggregationService};
use stellar_insights_backend::services::indexing::IndexingService;

async fn setup_test_db() -> Arc<Database> {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    Arc::new(Database::new(pool))
}

#[tokio::test]
async fn test_payment_ingestion_pipeline_end_to_end() {
    let db = setup_test_db().await;
    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));

    // 1) Ingest mock payments from RPC into DB
    let indexing = IndexingService::new(Arc::clone(&rpc_client), Arc::clone(&db));
    indexing.run_payment_ingestion().await.unwrap();

    let stored_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payments")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert!(
        stored_count > 0,
        "expected ingested payments to be persisted"
    );

    // 2) Verify ingestion cursor is advanced after successful ingestion
    let cursor = db.get_ingestion_cursor("payment_ingestion").await.unwrap();
    assert!(cursor.is_some(), "expected ingestion cursor to be stored");

    // 3) Aggregate ingested data into hourly corridor metrics
    let aggregation = AggregationService::new(
        Arc::clone(&db),
        AggregationConfig {
            interval_hours: 1,
            lookback_hours: 24 * 365 * 10,
            batch_size: 10_000,
        },
    );
    aggregation.run_hourly_aggregation().await.unwrap();

    // 4) Verify aggregated corridor metrics were created
    let start = Utc::now() - Duration::days(3650);
    let end = Utc::now() + Duration::days(3650);
    let metrics = db
        .fetch_hourly_metrics_by_timerange(start, end)
        .await
        .unwrap();
    assert!(
        !metrics.is_empty(),
        "expected hourly corridor metrics after aggregation"
    );

    for metric in metrics {
        assert!(
            (0.0..=100.0).contains(&metric.success_rate),
            "success_rate must stay in [0, 100]"
        );
        assert!(
            metric.total_transactions >= 0,
            "total_transactions must be non-negative"
        );
    }
}
