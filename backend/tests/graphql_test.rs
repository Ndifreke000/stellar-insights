//! Integration coverage for the GraphQL API surface (issues #1856, #2121-#2125).
//!
//! Tests the consolidated GraphQL schema including queries, mutations, and
//! the health endpoint. Uses an in-memory SQLite database for testing.

use async_graphql::*;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

// Import the old feature-level API for backward compatibility tests
use stellar_insights_backend::features::graphql_api::{GraphQLAPI, GraphQLAPIConfig};
use stellar_insights_backend::models::graphql_api::GraphQLRequest;

fn old_request(query: &str) -> GraphQLRequest {
    GraphQLRequest {
        query: query.to_string(),
        variables: None,
        operation_name: None,
    }
}

// ── Legacy Feature-Level API Tests (backward compatibility) ───────────────────

#[tokio::test]
async fn legacy_health_query_returns_ok_status() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 0);

    let response = api
        .execute(old_request("{ health { status version } }"))
        .await
        .expect("health query should execute");

    assert!(response.success, "expected successful response");
    let data = response.data.expect("health query should return data");
    assert_eq!(data["health"]["status"], "ok");
    assert_eq!(data["health"]["version"], "1.0.0");
}

#[tokio::test]
async fn legacy_anchor_count_query_reflects_seeded_value() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 42);

    let response = api
        .execute(old_request("{ anchorCount { count } }"))
        .await
        .expect("anchorCount query should execute");

    assert!(response.success);
    let data = response.data.expect("anchorCount should return data");
    assert_eq!(data["anchorCount"]["count"], 42);
}

#[tokio::test]
async fn legacy_invalid_query_is_rejected_without_panicking() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 0);

    let result = api.execute(old_request("")).await;

    assert!(result.is_err(), "empty query should be rejected");
}

#[tokio::test]
async fn legacy_disabled_api_rejects_queries() {
    let config = GraphQLAPIConfig {
        enabled: false,
        ..GraphQLAPIConfig::default()
    };
    let api = GraphQLAPI::new(config, 0);

    let result = api.execute(old_request("{ health { status } }")).await;

    assert!(result.is_err(), "disabled API should not execute queries");
}

#[tokio::test]
async fn legacy_health_status_is_exposed_for_health_aggregation() {
    let api = GraphQLAPI::new(GraphQLAPIConfig::default(), 0);

    let status = api.health_status();

    assert!(status.enabled);
    assert_eq!(status.endpoint, "/graphql");
    assert!(!status.version.is_empty());
}

// ── Consolidated Schema Tests ────────────────────────────────────────────────

/// Create an in-memory SQLite database for testing
async fn create_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create in-memory SQLite pool");

    // Run migrations
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

/// Build a test schema with in-memory database
async fn create_test_schema() -> stellar_insights_backend::graphql::AppSchema {
    let pool = Arc::new(create_test_db().await);
    let (broadcast_tx, _) = broadcast::channel::<String>(10);
    stellar_insights_backend::graphql::build_schema(pool, broadcast_tx)
}

#[tokio::test]
async fn consolidated_health_query_works() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ health { status version database cache } }").await;

    assert!(result.errors.is_empty(), "health query should have no errors");
    let data = result.data.into_json().unwrap();
    assert_eq!(data["health"]["status"], "ok");
    assert!(!data["health"]["version"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_anchor_count_query_works() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ anchorCount }").await;

    assert!(result.errors.is_empty(), "anchorCount query should have no errors");
    let data = result.data.into_json().unwrap();
    assert_eq!(data["anchorCount"], 0);
}

#[tokio::test]
async fn consolidated_anchors_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ anchors { nodes { id name } totalCount hasNextPage } }").await;

    assert!(result.errors.is_empty(), "anchors query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["anchors"]["nodes"].as_array().unwrap().is_empty());
    assert_eq!(data["anchors"]["totalCount"], 0);
    assert_eq!(data["anchors"]["hasNextPage"], false);
}

#[tokio::test]
async fn consolidated_corridors_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ corridors { nodes { id sourceAssetCode } totalCount } }").await;

    assert!(result.errors.is_empty(), "corridors query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["corridors"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_payments_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ payments { nodes { id amount } totalCount } }").await;

    assert!(result.errors.is_empty(), "payments query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["payments"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_liquidity_pools_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ liquidityPools { nodes { poolId } totalCount } }").await;

    assert!(result.errors.is_empty(), "liquidityPools query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["liquidityPools"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_trustline_stats_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ trustlineStats { nodes { assetCode } totalCount } }").await;

    assert!(result.errors.is_empty(), "trustlineStats query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["trustlineStats"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_search_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ search(query: \"test\") { anchors { id } corridors { id } } }").await;

    assert!(result.errors.is_empty(), "search query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["search"]["anchors"].as_array().unwrap().is_empty());
    assert!(data["search"]["corridors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_create_anchor_mutation_works() {
    let schema = create_test_schema().await;

    let mutation = r#"
        mutation {
            createAnchor(input: {
                name: "Test Anchor",
                stellarAccount: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
                homeDomain: Some("example.com")
            }) {
                anchor {
                    id
                    name
                    stellarAccount
                    homeDomain
                }
                success
                message
            }
        }
    "#;

    let result = schema.execute(mutation).await;

    if !result.errors.is_empty() {
        eprintln!("Errors: {:?}", result.errors);
    }
    assert!(result.errors.is_empty(), "createAnchor mutation should have no errors: {:?}", result.errors);
    let data = result.data.into_json().unwrap();
    assert_eq!(data["createAnchor"]["success"], true);
    assert_eq!(data["createAnchor"]["anchor"]["name"], "Test Anchor");
}

#[tokio::test]
async fn consolidated_create_corridor_mutation_works() {
    let schema = create_test_schema().await;

    let mutation = r#"
        mutation {
            createCorridor(input: {
                sourceAssetCode: "USDC",
                sourceAssetIssuer: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
                destinationAssetCode: "EUR",
                destinationAssetIssuer: "GARE5K4KJL3VQ4E5VZ6JZ7X7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q7Q"
            }) {
                corridor {
                    id
                    sourceAssetCode
                    destinationAssetCode
                }
                success
                message
            }
        }
    "#;

    let result = schema.execute(mutation).await;

    if !result.errors.is_empty() {
        eprintln!("Errors: {:?}", result.errors);
    }
    assert!(result.errors.is_empty(), "createCorridor mutation should have no errors: {:?}", result.errors);
    let data = result.data.into_json().unwrap();
    assert_eq!(data["createCorridor"]["success"], true);
    assert_eq!(data["createCorridor"]["corridor"]["sourceAssetCode"], "USDC");
}

#[tokio::test]
async fn consolidated_create_anchor_validation_error() {
    let schema = create_test_schema().await;

    // Name too long
    let mutation = format!(
        r#"
        mutation {{
            createAnchor(input: {{
                name: "{}",
                stellarAccount: "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H"
            }}) {{
                success
            }}
        }}
        "#,
        "x".repeat(101)
    );

    let result = schema.execute(&mutation).await;

    assert!(!result.errors.is_empty(), "should return validation error for long name");
}

#[tokio::test]
async fn consolidated_liquidity_pool_stats_query() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ liquidityPoolStats { totalPools totalLiquidityUsd } }").await;

    assert!(result.errors.is_empty(), "liquidityPoolStats query should have no errors");
    let data = result.data.into_json().unwrap();
    assert_eq!(data["liquidityPoolStats"]["totalPools"], 0);
}

#[tokio::test]
async fn consolidated_trustline_metrics_query() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ trustlineMetrics { totalAssetsTracked activeAssets } }").await;

    assert!(result.errors.is_empty(), "trustlineMetrics query should have no errors");
    let data = result.data.into_json().unwrap();
    assert_eq!(data["trustlineMetrics"]["totalAssetsTracked"], 0);
}

#[tokio::test]
async fn consolidated_snapshots_query_empty() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ snapshots { nodes { id } totalCount } }").await;

    assert!(result.errors.is_empty(), "snapshots query should have no errors");
    let data = result.data.into_json().unwrap();
    assert!(data["snapshots"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn consolidated_introspection_works() {
    let schema = create_test_schema().await;

    let result = schema.execute("{ __schema { queryType { name } mutationType { name } subscriptionType { name } } }").await;

    assert!(result.errors.is_empty(), "introspection should have no errors");
    let data = result.data.into_json().unwrap();
    assert_eq!(data["__schema"]["queryType"]["name"], "QueryRoot");
    assert_eq!(data["__schema"]["mutationType"]["name"], "MutationRoot");
    assert_eq!(data["__schema"]["subscriptionType"]["name"], "SubscriptionRoot");
}
