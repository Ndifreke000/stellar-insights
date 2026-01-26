use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;
use tower::util::ServiceExt;

// Use correct handlers from the updated API
use backend::api::corridors::{get_corridor_detail, list_corridors};
use backend::database::Database;
use backend::models::corridor::CorridorMetrics;

async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:password@localhost:5432/stellar_insights_test".to_string()
    });

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Clean up existing test data
    sqlx::query("DELETE FROM corridor_metrics")
        .execute(&pool)
        .await
        .unwrap();

    pool
}

async fn create_test_corridor_metrics(db: &Database) -> Vec<CorridorMetrics> {
    let mut corridors = Vec::new();

    // Create test corridor 1: EURC->USDC
    let corridor1 = db
        .create_or_update_corridor_metrics(
            "EURC:issuer2->USDC:issuer1",
            "EURC",
            "issuer2",
            "USDC",
            "issuer1",
            1000,
            950,
            50,
            1000000.0,
        )
        .await
        .unwrap();
    corridors.push(corridor1);

    // Create test corridor 2: BTC->ETH
    let corridor2 = db
        .create_or_update_corridor_metrics(
            "BTC:issuer3->ETH:issuer4",
            "BTC",
            "issuer3",
            "ETH",
            "issuer4",
            500,
            400,
            100,
            2000000.0,
        )
        .await
        .unwrap();
    corridors.push(corridor2);

    corridors
}

fn create_test_router(db: Arc<Database>) -> Router {
    Router::new()
        .route("/api/corridors", axum::routing::get(list_corridors))
        .route(
            "/api/corridors/:corridor_key",
            axum::routing::get(get_corridor_detail),
        )
        .with_state(db)
}

#[tokio::test]
async fn test_list_corridors_success() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    create_test_corridor_metrics(&db).await;

    let app = create_test_router(db);

    let request = Request::builder()
        .uri("/api/corridors")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.is_array());
    let corridors = json.as_array().unwrap();
    assert_eq!(corridors.len(), 2);

    // Verify response structure matches new CorridorResponse
    let first_corridor = &corridors[0];
    assert!(first_corridor["id"].is_string());
    assert!(first_corridor["source_asset"].is_string());
    assert!(first_corridor["destination_asset"].is_string());
    assert!(first_corridor["success_rate"].is_number());
    assert!(first_corridor["total_attempts"].is_number());
    assert!(first_corridor["health_score"].is_number());

    // Check values match injected data (EURC->USDC)
    // Note: ordering might vary, so find the EURC one
    let eurc_corridor = corridors
        .iter()
        .find(|c| c["source_asset"].as_str() == Some("EURC"))
        .expect("EURC corridor not found");

    assert_eq!(eurc_corridor["destination_asset"], "USDC");
    assert_eq!(eurc_corridor["total_attempts"].as_i64(), Some(1000));
    assert_eq!(eurc_corridor["successful_payments"].as_i64(), Some(950));
}

#[tokio::test]
async fn test_get_corridor_detail_success() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    create_test_corridor_metrics(&db).await;

    let app = create_test_router(db);

    let corridor_key = "EURC:issuer2->USDC:issuer1";
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", corridor_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Verify detail response structure
    assert!(json["corridor"].is_object());
    assert!(json["historical_success_rate"].is_array());
    assert!(json["latency_distribution"].is_array());

    let corridor = &json["corridor"];
    assert_eq!(corridor["id"], corridor_key);
    assert_eq!(corridor["source_asset"], "EURC");
    assert_eq!(corridor["success_rate"].as_f64(), Some(95.0)); // 950/1000 * 100
}

#[tokio::test]
async fn test_get_corridor_detail_not_found() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    // No data created

    let app = create_test_router(db);

    let corridor_key = "NONEXISTENT:issuer->FAKE:issuer";
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", corridor_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_corridor_detail_invalid_format() {
    let pool = setup_test_db().await;
    let db = Arc::new(Database::new(pool));
    let app = create_test_router(db);

    let invalid_key = "INVALID_FORMAT";
    let request = Request::builder()
        .uri(&format!("/api/corridors/{}", invalid_key))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Handler should return BadRequest for invalid format
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
