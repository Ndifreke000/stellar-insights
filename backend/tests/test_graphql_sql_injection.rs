use sqlx::SqlitePool;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    
    // Create only the tables we need for testing
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS anchors (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            stellar_account TEXT NOT NULL UNIQUE,
            home_domain TEXT,
            total_transactions INTEGER DEFAULT 0,
            successful_transactions INTEGER DEFAULT 0,
            failed_transactions INTEGER DEFAULT 0,
            total_volume_usd REAL DEFAULT 0,
            avg_settlement_time_ms INTEGER DEFAULT 0,
            reliability_score REAL DEFAULT 0,
            status TEXT DEFAULT 'green',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();
    
    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS corridors (
            id TEXT PRIMARY KEY,
            source_asset_code TEXT NOT NULL,
            source_asset_issuer TEXT NOT NULL,
            destination_asset_code TEXT NOT NULL,
            destination_asset_issuer TEXT NOT NULL,
            reliability_score REAL DEFAULT 0,
            status TEXT DEFAULT 'active',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();
    
    pool
}

async fn setup_test_db_with_data() -> SqlitePool {
    let pool = setup_test_db().await;
    
    // Insert test anchor data
    sqlx::query(
        r"
        INSERT INTO anchors (
            id, name, stellar_account, home_domain,
            total_transactions, successful_transactions, failed_transactions,
            total_volume_usd, avg_settlement_time_ms, reliability_score,
            status, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        ",
    )
    .bind("anchor-1")
    .bind("Test Anchor")
    .bind("GTEST123")
    .bind("test.com")
    .bind(1000)
    .bind(950)
    .bind(50)
    .bind(100000.0)
    .bind(5000)
    .bind(95.0)
    .bind("green")
    .execute(&pool)
    .await
    .unwrap();
    
    // Insert test corridor data
    sqlx::query(
        r"
        INSERT INTO corridors (
            id, source_asset_code, source_asset_issuer,
            destination_asset_code, destination_asset_issuer,
            reliability_score, status, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        ",
    )
    .bind("corridor-1")
    .bind("USD")
    .bind("GISSUER1")
    .bind("EUR")
    .bind("GISSUER2")
    .bind(90.0)
    .bind("active")
    .execute(&pool)
    .await
    .unwrap();
    
    pool
}

#[tokio::test]
async fn test_sql_injection_in_corridors_query() {
    let pool = setup_test_db_with_data().await;
    
    // Test SQL injection attempt in source_asset_code
    let malicious_input = "USD' OR '1'='1";
    
    // This demonstrates what would happen with vulnerable string concatenation
    // (we don't actually execute this - just showing the vulnerability)
    let _vulnerable_query = format!(
        "SELECT * FROM corridors WHERE source_asset_code = '{}'",
        malicious_input
    );
    
    // This would be vulnerable if we used string concatenation
    // But our parameterized queries prevent this
    
    // Test with our safe QueryBuilder approach
    use sqlx::QueryBuilder;
    let mut builder = QueryBuilder::new("SELECT * FROM corridors WHERE source_asset_code = ");
    builder.push_bind(malicious_input);
    
    let result = builder
        .build()
        .fetch_all(&pool)
        .await;
    
    // Should return 0 results since no corridor has that exact source asset
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_sql_injection_in_anchors_query() {
    let pool = setup_test_db_with_data().await;
    
    // Test SQL injection attempt in status filter
    let malicious_status = "green' OR '1'='1";
    
    // Test with our safe QueryBuilder approach
    use sqlx::QueryBuilder;
    let mut builder = QueryBuilder::new("SELECT * FROM anchors WHERE status = ");
    builder.push_bind(malicious_status);
    
    let result = builder
        .build()
        .fetch_all(&pool)
        .await;
    
    // Should return 0 results since no anchor has that exact status
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
async fn test_parameterized_like_queries() {
    let pool = setup_test_db_with_data().await;
    
    // Test SQL injection attempt in LIKE query
    let malicious_search = "Test' OR '1'='1";
    let search_pattern = format!("%{}%", malicious_search);
    
    // Test with our safe QueryBuilder approach
    use sqlx::QueryBuilder;
    let mut builder = QueryBuilder::new("SELECT * FROM anchors WHERE name LIKE ");
    builder.push_bind(&search_pattern);
    
    let result = builder
        .build()
        .fetch_all(&pool)
        .await;
    
    // Should only match legitimate search results, not all records
    assert!(result.is_ok());
    let rows = result.unwrap();
    // Should return 0 since the malicious string doesn't match "Test Anchor"
    assert_eq!(rows.len(), 0);
    
    // Test legitimate search
    let legitimate_search = "Test";
    let search_pattern = format!("%{}%", legitimate_search);
    
    let mut builder = QueryBuilder::new("SELECT * FROM anchors WHERE name LIKE ");
    builder.push_bind(&search_pattern);
    
    let result = builder
        .build()
        .fetch_all(&pool)
        .await;
    
    assert!(result.is_ok());
    let rows = result.unwrap();
    // Should return 1 matching "Test Anchor"
    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn test_legitimate_parameterized_queries() {
    let pool = setup_test_db_with_data().await;
    
    // Test legitimate queries work correctly
    use sqlx::QueryBuilder;
    
    // Test 1: Legitimate status filter
    let mut builder = QueryBuilder::new("SELECT * FROM anchors WHERE status = ");
    builder.push_bind("green");
    
    let result = builder
        .build()
        .fetch_all(&pool)
        .await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
    
    // Test 2: Legitimate corridor filter
    let mut builder = QueryBuilder::new("SELECT * FROM corridors WHERE source_asset_code = ");
    builder.push_bind("USD");
    builder.push(" AND status = ");
    builder.push_bind("active");
    
    let result = builder
        .build()
        .fetch_all(&pool)
        .await;
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}
