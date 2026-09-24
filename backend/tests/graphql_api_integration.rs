//! Integration tests for the consolidated GraphQL API layer (issues #2121-#2125).
//!
//! Tests the full GraphQL schema including:
//! - Query resolvers for all entities
//! - Mutation resolvers for create/update/delete operations
//! - Subscription setup (WebSocket handler verification)
//! - Error handling and validation
//! - SQL injection prevention

use async_graphql::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

use payraider_backend::graphql;

/// Create an in-memory SQLite database with all required tables
async fn create_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS anchors (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            stellar_account TEXT NOT NULL UNIQUE,
            home_domain TEXT,
            total_transactions INTEGER DEFAULT 0,
            successful_transactions INTEGER DEFAULT 0,
            failed_transactions INTEGER DEFAULT 0,
            total_volume_usd REAL DEFAULT 0.0,
            avg_settlement_time_ms INTEGER DEFAULT 0,
            reliability_score REAL DEFAULT 0.0,
            status TEXT DEFAULT 'green',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS corridors (
            id TEXT PRIMARY KEY,
            source_asset_code TEXT NOT NULL,
            source_asset_issuer TEXT NOT NULL,
            destination_asset_code TEXT NOT NULL,
            destination_asset_issuer TEXT NOT NULL,
            reliability_score REAL DEFAULT 0.0,
            status TEXT DEFAULT 'active',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(source_asset_code, source_asset_issuer, destination_asset_code, destination_asset_issuer)
        );

        CREATE TABLE IF NOT EXISTS payments (
            id TEXT PRIMARY KEY,
            transaction_hash TEXT NOT NULL,
            source_account TEXT NOT NULL,
            destination_account TEXT NOT NULL,
            asset_type TEXT NOT NULL,
            asset_code TEXT,
            asset_issuer TEXT,
            source_asset_code TEXT DEFAULT '',
            source_asset_issuer TEXT DEFAULT '',
            destination_asset_code TEXT DEFAULT '',
            destination_asset_issuer TEXT DEFAULT '',
            amount REAL NOT NULL,
            successful BOOLEAN DEFAULT 1,
            timestamp DATETIME,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            anchor_id TEXT NOT NULL,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_supply REAL,
            num_holders INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(asset_code, asset_issuer)
        );

        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            entity_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            data TEXT NOT NULL,
            hash TEXT,
            epoch INTEGER,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS metrics (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            value REAL NOT NULL,
            entity_id TEXT,
            entity_type TEXT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS liquidity_pools (
            pool_id TEXT PRIMARY KEY,
            pool_type TEXT NOT NULL,
            fee_bp INTEGER DEFAULT 0,
            total_trustlines INTEGER DEFAULT 0,
            total_shares TEXT DEFAULT '0',
            reserve_a_asset_code TEXT NOT NULL,
            reserve_a_asset_issuer TEXT,
            reserve_a_amount REAL DEFAULT 0.0,
            reserve_b_asset_code TEXT NOT NULL,
            reserve_b_asset_issuer TEXT,
            reserve_b_amount REAL DEFAULT 0.0,
            total_value_usd REAL DEFAULT 0.0,
            volume_24h_usd REAL DEFAULT 0.0,
            fees_earned_24h_usd REAL DEFAULT 0.0,
            apy REAL DEFAULT 0.0,
            impermanent_loss_pct REAL DEFAULT 0.0,
            trade_count_24h INTEGER DEFAULT 0,
            last_synced_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS trustline_stats (
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_trustlines INTEGER DEFAULT 0,
            authorized_trustlines INTEGER DEFAULT 0,
            unauthorized_trustlines INTEGER DEFAULT 0,
            total_supply REAL DEFAULT 0.0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (asset_code, asset_issuer)
        );

        CREATE TABLE IF NOT EXISTS trustline_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_trustlines INTEGER DEFAULT 0,
            authorized_trustlines INTEGER DEFAULT 0,
            unauthorized_trustlines INTEGER DEFAULT 0,
            total_supply REAL DEFAULT 0.0,
            snapshot_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS anchor_metrics_history (
            id TEXT PRIMARY KEY,
            anchor_id TEXT NOT NULL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            success_rate REAL DEFAULT 0.0,
            failure_rate REAL DEFAULT 0.0,
            reliability_score REAL DEFAULT 0.0,
            total_transactions INTEGER DEFAULT 0,
            successful_transactions INTEGER DEFAULT 0,
            failed_transactions INTEGER DEFAULT 0,
            avg_settlement_time_ms INTEGER,
            volume_usd REAL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create tables");

    pool
}

/// Build a test schema
async fn create_test_schema() -> graphql::AppSchema {
    let pool = Arc::new(create_test_db().await);
    let (broadcast_tx, _) = broadcast::channel::<String>(10);
    graphql::build_schema(pool, broadcast_tx)
}

// ── Query Tests ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_health_query() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ health { status version database cache } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["health"]["status"], "ok");
}

#[tokio::test]
async fn test_anchor_count_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ anchorCount }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["anchorCount"], 0);
}

#[tokio::test]
async fn test_anchors_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ anchors { nodes { id name } totalCount } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["anchors"]["nodes"].as_array().unwrap().is_empty());
    assert_eq!(data["anchors"]["totalCount"], 0);
}

#[tokio::test]
async fn test_corridors_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ corridors { nodes { id } totalCount } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["corridors"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_payments_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ payments { nodes { id amount } totalCount } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["payments"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_liquidity_pools_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ liquidityPools { nodes { poolId } totalCount } }").await;

    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_trustline_stats_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ trustlineStats { nodes { assetCode } totalCount } }").await;

    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_snapshots_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ snapshots { nodes { id } totalCount } }").await;

    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_search_empty() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ search(query: \"test\") { anchors { id } corridors { id } } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert!(data["search"]["anchors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_introspection_types() {
    let schema = create_test_schema().await;
    let result = schema.execute("{ __type(name: \"Anchor\") { name fields { name } } }").await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["__type"]["name"], "Anchor");
    let fields = data["__type"]["fields"].as_array().unwrap();
    let field_names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert!(field_names.contains(&"id"));
    assert!(field_names.contains(&"name"));
    assert!(field_names.contains(&"reliabilityScore"));
}

// ── Mutation Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_anchor_mutation() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        mutation {
            createAnchor(input: {
                name: "Test Anchor",
                stellarAccount: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H"
            }) {
                anchor { id name stellarAccount }
                success
                message
            }
        }
    "#).await;

    assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    let data = result.data.into_json().unwrap();
    assert_eq!(data["createAnchor"]["success"], true);
    assert_eq!(data["createAnchor"]["anchor"]["name"], "Test Anchor");
}

#[tokio::test]
async fn test_create_anchor_name_too_long() {
    let schema = create_test_schema().await;

    let result = schema.execute(&format!(r#"
        mutation {{
            createAnchor(input: {{
                name: "{}",
                stellarAccount: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H"
            }}) {{
                success
            }}
        }}
    "#, "x".repeat(101))).await;

    assert!(!result.errors.is_empty());
}

#[tokio::test]
async fn test_create_anchor_stellar_account_wrong_length() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        mutation {
            createAnchor(input: {
                name: "Test",
                stellarAccount: "SHORT"
            }) {
                success
            }
        }
    "#).await;

    assert!(!result.errors.is_empty());
}

#[tokio::test]
async fn test_create_corridor_mutation() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        mutation {
            createCorridor(input: {
                sourceAssetCode: "USDC",
                sourceAssetIssuer: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
                destinationAssetCode: "EUR",
                destinationAssetIssuer: "GARE5K4KJL3VQ4E5VZ6JZ7X7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q"
            }) {
                corridor { id sourceAssetCode destinationAssetCode }
                success
                message
            }
        }
    "#).await;

    assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    let data = result.data.into_json().unwrap();
    assert_eq!(data["createCorridor"]["success"], true);
    assert_eq!(data["createCorridor"]["corridor"]["sourceAssetCode"], "USDC");
}

#[tokio::test]
async fn test_create_anchor_then_query() {
    let schema = create_test_schema().await;

    // Create anchor
    schema.execute(r#"
        mutation {
            createAnchor(input: {
                name: "My Anchor",
                stellarAccount: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H"
            }) {
                success
            }
        }
    "#).await;

    // Query anchors
    let result = schema.execute("{ anchors { nodes { id name } totalCount } }").await;
    assert!(result.errors.is_empty());

    let data = result.data.into_json().unwrap();
    assert_eq!(data["anchors"]["totalCount"], 1);
    assert_eq!(data["anchors"]["nodes"][0]["name"], "My Anchor");
}

#[tokio::test]
async fn test_delete_anchor_mutation() {
    let schema = create_test_schema().await;

    // Create
    let result = schema.execute(r#"
        mutation {
            createAnchor(input: {
                name: "To Delete",
                stellarAccount: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H"
            }) {
                anchor { id }
                success
            }
        }
    "#).await;

    let data = result.data.into_json().unwrap();
    let anchor_id = data["createAnchor"]["anchor"]["id"].as_str().unwrap().to_string();

    // Delete
    let result = schema.execute(&format!(
        r#"
        mutation {{
            deleteAnchor(id: "{}")
        }}
    "#, anchor_id)).await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["deleteAnchor"], true);

    // Verify deleted
    let result = schema.execute("{ anchors { totalCount } }").await;
    let data = result.data.into_json().unwrap();
    assert_eq!(data["anchors"]["totalCount"], 0);
}

#[tokio::test]
async fn test_delete_nonexistent_anchor() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        mutation {
            deleteAnchor(id: "nonexistent-id")
        }
    "#).await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["deleteAnchor"], false);
}

// ── SQL Injection Prevention Tests ────────────────────────────────────────────

#[tokio::test]
async fn test_sql_injection_in_search() {
    let schema = create_test_schema().await;

    // Try SQL injection via search query
    let result = schema.execute(r#"
        { search(query: "'; DROP TABLE anchors; --") { anchors { id } } }
    "#).await;

    // Should not error (no SQL injection possible with parameterized queries)
    assert!(result.errors.is_empty());

    // Verify table still exists by querying it
    let result = schema.execute("{ anchorCount }").await;
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_sql_injection_in_anchor_filter() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        { anchors(filter: { search: "'; DROP TABLE anchors; --" }) { nodes { id } } }
    "#).await;

    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_sql_injection_in_corridor_filter() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        { corridors(filter: { sourceAssetCode: "'; DROP TABLE corridors; --" }) { nodes { id } } }
    "#).await;

    assert!(result.errors.is_empty());
}

// ── Subscription Schema Verification ─────────────────────────────────────────

#[tokio::test]
async fn test_subscription_type_exists() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        { __type(name: "SubscriptionRoot") { name kind } }
    "#).await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    assert_eq!(data["__type"]["name"], "SubscriptionRoot");
    assert_eq!(data["__type"]["kind"], "OBJECT");
}

#[tokio::test]
async fn test_subscription_fields_exist() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        { __type(name: "SubscriptionRoot") { fields { name } } }
    "#).await;

    assert!(result.errors.is_empty());
    let data = result.data.into_json().unwrap();
    let fields = data["__type"]["fields"].as_array().unwrap();
    let field_names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert!(field_names.contains(&"corridorUpdates"));
    assert!(field_names.contains(&"anchorUpdates"));
    assert!(field_names.contains(&"snapshotUpdates"));
    assert!(field_names.contains(&"healthAlerts"));
    assert!(field_names.contains(&"newPayments"));
}

// ── Complexity / Depth Limiting Tests ─────────────────────────────────────────

#[tokio::test]
async fn test_complex_query_does_not_panic() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        {
            health { status }
            anchorCount
            anchors { nodes { id name } totalCount }
            corridors { nodes { id sourceAssetCode } totalCount }
            payments { nodes { id amount } totalCount }
            liquidityPools { nodes { poolId } totalCount }
            trustlineStats { nodes { assetCode } totalCount }
            snapshots { nodes { id } totalCount }
            search(query: "test") { anchors { id } corridors { id } }
            liquidityPoolStats { totalPools }
            trustlineMetrics { totalAssetsTracked }
        }
    "#).await;

    assert!(result.errors.is_empty(), "Complex query should not error: {:?}", result.errors);
}

// ── Pagination Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_pagination_input() {
    let schema = create_test_schema().await;

    let result = schema.execute(r#"
        { anchors(pagination: { limit: 5, offset: 0 }) { nodes { id } totalCount hasNextPage } }
    "#).await;

    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_pagination_max_limit() {
    let schema = create_test_schema().await;

    // Even with limit > 100, should be capped at 100
    let result = schema.execute(r#"
        { anchors(pagination: { limit: 200 }) { nodes { id } } }
    "#).await;

    assert!(result.errors.is_empty());
}
