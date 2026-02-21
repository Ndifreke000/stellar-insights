use sqlx::SqlitePool;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use stellar_insights_backend::rpc::StellarRpcClient;
use stellar_insights_backend::services::account_merge_detector::AccountMergeDetector;
use tower::util::ServiceExt;

#[sqlx::test]
async fn test_account_merge_detector_process_and_stats(pool: SqlitePool) {

    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let detector = AccountMergeDetector::new(pool.clone(), rpc_client);

    sqlx::query(
        "INSERT INTO ledgers (sequence, hash, close_time, transaction_count, operation_count) VALUES (200, 'ledger_hash_200', '2026-01-22T10:30:00Z', 0, 0)",
    )
    .execute(&pool)
    .await
    .expect("failed to insert ledger row");

    let detected = detector
        .process_ledger_operations(200)
        .await
        .expect("failed to process account merges");
    assert_eq!(detected, 2);

    let recent = detector.get_recent_merges(10).await.unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].ledger_sequence, 200);

    let stats = detector.get_merge_stats().await.unwrap();
    assert_eq!(stats.total_merges, 2);
    assert!((stats.total_merged_balance - 136.0).abs() < f64::EPSILON);
    assert_eq!(stats.unique_sources, 2);
    assert_eq!(stats.unique_destinations, 2);

    let patterns = detector.get_destination_patterns(10).await.unwrap();
    assert_eq!(patterns.len(), 2);
    assert_eq!(patterns[0].merge_count, 1);
}

#[sqlx::test]
async fn test_account_merge_detector_is_idempotent(pool: SqlitePool) {

    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let detector = AccountMergeDetector::new(pool.clone(), rpc_client);

    sqlx::query(
        "INSERT INTO ledgers (sequence, hash, close_time, transaction_count, operation_count) VALUES (201, 'ledger_hash_201', '2026-01-22T10:31:00Z', 0, 0)",
    )
    .execute(&pool)
    .await
    .expect("failed to insert ledger row");

    let first = detector.process_ledger_operations(201).await.unwrap();
    let second = detector.process_ledger_operations(201).await.unwrap();

    assert_eq!(first, 2);
    assert_eq!(second, 0);

    let stats = detector.get_merge_stats().await.unwrap();
    assert_eq!(stats.total_merges, 2);
}

#[sqlx::test]
async fn test_account_merge_routes(pool: SqlitePool) {

    let rpc_client = Arc::new(StellarRpcClient::new_with_defaults(true));
    let detector = Arc::new(AccountMergeDetector::new(pool.clone(), rpc_client));

    sqlx::query(
        "INSERT INTO ledgers (sequence, hash, close_time, transaction_count, operation_count) VALUES (202, 'ledger_hash_202', '2026-01-22T10:32:00Z', 0, 0)",
    )
    .execute(&pool)
    .await
    .expect("failed to insert ledger row");

    detector.process_ledger_operations(202).await.unwrap();

    let app = stellar_insights_backend::api::account_merges::routes(detector);

    let stats_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stats_res.status(), StatusCode::OK);
    let stats_body = to_bytes(stats_res.into_body(), usize::MAX).await.unwrap();
    let stats: serde_json::Value = serde_json::from_slice(&stats_body).unwrap();
    assert_eq!(stats["total_merges"], 2);

    let recent_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/recent?limit=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(recent_res.status(), StatusCode::OK);
    let recent_body = to_bytes(recent_res.into_body(), usize::MAX).await.unwrap();
    let recent: Vec<serde_json::Value> = serde_json::from_slice(&recent_body).unwrap();
    assert_eq!(recent.len(), 1);

    let destinations_res = app
        .oneshot(
            Request::builder()
                .uri("/destinations?limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(destinations_res.status(), StatusCode::OK);
    let destinations_body = to_bytes(destinations_res.into_body(), usize::MAX)
        .await
        .unwrap();
    let destinations: Vec<serde_json::Value> = serde_json::from_slice(&destinations_body).unwrap();
    assert_eq!(destinations.len(), 2);
}
