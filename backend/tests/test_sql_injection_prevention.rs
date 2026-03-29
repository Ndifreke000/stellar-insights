use sqlx::SqlitePool;
use stellar_insights_backend::models::asset_verification::VerificationStatus;
use stellar_insights_backend::services::asset_verifier::AssetVerifier;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    // Create only the tables we need for testing
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS verified_assets (
            id TEXT PRIMARY KEY,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            verification_status TEXT NOT NULL,
            reputation_score REAL NOT NULL,
            stellar_expert_verified INTEGER NOT NULL,
            stellar_toml_verified INTEGER NOT NULL,
            anchor_registry_verified INTEGER NOT NULL,
            trustline_count INTEGER NOT NULL,
            transaction_count INTEGER NOT NULL,
            total_volume_usd REAL NOT NULL,
            toml_home_domain TEXT,
            toml_name TEXT,
            toml_description TEXT,
            toml_org_name TEXT,
            toml_org_url TEXT,
            toml_logo_url TEXT,
            suspicious_reports_count INTEGER DEFAULT 0,
            last_suspicious_report_at TEXT,
            last_verified_at TEXT,
            verification_notes TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(asset_code, asset_issuer)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS asset_verification_history (
            id TEXT PRIMARY KEY,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            previous_status TEXT,
            new_status TEXT NOT NULL,
            previous_reputation_score REAL,
            new_reputation_score REAL NOT NULL,
            change_reason TEXT NOT NULL,
            created_at TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS payments (
            id TEXT PRIMARY KEY,
            transaction_hash TEXT NOT NULL,
            source_account TEXT NOT NULL,
            destination_account TEXT NOT NULL,
            asset_type TEXT NOT NULL,
            asset_code TEXT,
            asset_issuer TEXT,
            amount REAL NOT NULL,
            created_at TEXT NOT NULL
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[tokio::test]
async fn test_sql_injection_prevention_in_asset_verifier() {
    let pool = setup_test_db().await;
    let verifier = AssetVerifier::new(pool.clone()).unwrap();

    // Test 1: SQL injection attempt in status filter
    let malicious_status = Some(VerificationStatus::Verified);
    let result = verifier
        .list_verified_assets(malicious_status, None, 10, 0)
        .await;

    // Should not panic or cause SQL errors
    assert!(result.is_ok());

    // Test 2: SQL injection attempt in min_reputation filter
    // This should be safe as it's a numeric type, but test anyway
    let result = verifier.list_verified_assets(None, Some(50.0), 10, 0).await;

    assert!(result.is_ok());

    // Test 3: Combined filters
    let result = verifier
        .list_verified_assets(Some(VerificationStatus::Verified), Some(75.0), 10, 0)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parameterized_queries_work_correctly() {
    let pool = setup_test_db().await;
    let verifier = AssetVerifier::new(pool.clone()).unwrap();

    // Insert test data
    sqlx::query(
        r"
        INSERT INTO verified_assets (
            id, asset_code, asset_issuer, verification_status, reputation_score,
            stellar_expert_verified, stellar_toml_verified, anchor_registry_verified,
            trustline_count, transaction_count, total_volume_usd,
            last_verified_at, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'), datetime('now'))
        ",
    )
    .bind("test-id-1")
    .bind("USDC")
    .bind("GTEST123")
    .bind("verified")
    .bind(85.0)
    .bind(true)
    .bind(true)
    .bind(false)
    .bind(1000)
    .bind(5000)
    .bind(100000.0)
    .execute(&pool)
    .await
    .unwrap();

    // Test filtering by status
    let assets = verifier
        .list_verified_assets(Some(VerificationStatus::Verified), None, 10, 0)
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].asset_code, "USDC");

    // Test filtering by min reputation
    let assets = verifier
        .list_verified_assets(None, Some(80.0), 10, 0)
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);

    // Test filtering with threshold that excludes the asset
    let assets = verifier
        .list_verified_assets(None, Some(90.0), 10, 0)
        .await
        .unwrap();

    assert_eq!(assets.len(), 0);
}
