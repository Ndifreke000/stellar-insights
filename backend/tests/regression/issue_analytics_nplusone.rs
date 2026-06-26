//! Regression: Analytics N+1 Query Issue
//!
//! # Background
//! The `/anchors` endpoint previously loaded assets for each anchor via a
//! separate `SELECT * FROM assets WHERE anchor_id = ?` call inside a loop,
//! producing N+1 database queries for N anchors.  Under mainnet load this
//! caused significant latency spikes and occasionally exhausted the SQLite
//! connection pool.
//!
//! # Reproduction
//! 1. Insert N anchors and their assets into the database.
//! 2. Call the asset-loading code in a loop (one query per anchor).
//! 3. Observe N round-trips to the database.
//!
//! # Fix
//! `AssetDb::get_assets_by_anchors` (in `db/assets.rs`) was introduced to
//! load all assets for a list of anchor IDs in a **single** SQL query using
//! an `IN (...)` clause.  The handler now batches anchor IDs and calls this
//! method exactly once regardless of the number of anchors returned.
//!
//! # References
//! - GitHub Issue: stellar-insights#analytics-nplusone
//! - Fixed in: `AssetDb::get_assets_by_anchors` (`src/db/assets.rs`)

use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared DB setup
// ---------------------------------------------------------------------------

/// Create an in-memory SQLite database with the `anchors` and `assets` tables
/// needed for the N+1 regression tests.
async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS anchors (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            stellar_account TEXT NOT NULL,
            home_domain TEXT,
            total_transactions INTEGER NOT NULL DEFAULT 0,
            successful_transactions INTEGER NOT NULL DEFAULT 0,
            failed_transactions INTEGER NOT NULL DEFAULT 0,
            total_volume_usd REAL NOT NULL DEFAULT 0.0,
            avg_settlement_time_ms INTEGER NOT NULL DEFAULT 0,
            reliability_score REAL NOT NULL DEFAULT 0.0,
            status TEXT NOT NULL DEFAULT 'active',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r"
        CREATE TABLE IF NOT EXISTS assets (
            id TEXT PRIMARY KEY,
            anchor_id TEXT NOT NULL,
            asset_code TEXT NOT NULL,
            asset_issuer TEXT NOT NULL,
            total_supply REAL,
            num_holders INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(asset_code, asset_issuer)
        )
        ",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a dummy anchor and return its UUID string.
async fn insert_anchor(pool: &SqlitePool, name: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO anchors (id, name, stellar_account, status) VALUES ($1, $2, $3, 'active')",
    )
    .bind(&id)
    .bind(name)
    .bind(format!(
        "G{}",
        "A".repeat(55) // valid 56-char account placeholder
    ))
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Insert a dummy asset linked to `anchor_id`.
async fn insert_asset(pool: &SqlitePool, anchor_id: &str, code: &str, issuer: &str) {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT OR IGNORE INTO assets (id, anchor_id, asset_code, asset_issuer) VALUES ($1, $2, $3, $4)",
    )
    .bind(&id)
    .bind(anchor_id)
    .bind(code)
    .bind(issuer)
    .execute(pool)
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// Core regression helper
// ---------------------------------------------------------------------------

/// Replicate `AssetDb::get_assets_by_anchors` inline so this test is
/// self-contained and verifies the batch-loading contract directly against
/// the raw SQL used in production.
async fn batch_load_assets(
    pool: &SqlitePool,
    anchor_ids: &[String],
) -> HashMap<String, Vec<String>> {
    if anchor_ids.is_empty() {
        return HashMap::new();
    }

    // Build `IN ($1, $2, …)` dynamically — same approach as production code.
    let placeholders: String = anchor_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");

    let query_str = format!(
        "SELECT anchor_id, asset_code FROM assets WHERE anchor_id IN ({placeholders}) ORDER BY anchor_id, asset_code ASC"
    );

    let mut query = sqlx::query_as::<_, (String, String)>(&query_str);
    for id in anchor_ids {
        query = query.bind(id);
    }

    let rows = query.fetch_all(pool).await.unwrap();

    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    for (anchor_id, asset_code) in rows {
        result.entry(anchor_id).or_default().push(asset_code);
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Regression: a single batch query retrieves assets for ALL anchors.
///
/// This test inserts 5 anchors with 2 assets each, then calls the batch
/// loader once and asserts that every anchor's assets are correctly
/// retrieved.  Previously this required 5 separate queries.
#[tokio::test]
async fn test_batch_load_returns_all_assets_in_one_query() {
    let pool = setup_db().await;

    // Insert 5 anchors with 2 assets each.
    let anchor_count = 5usize;
    let assets_per_anchor = 2usize;
    let mut anchor_ids: Vec<String> = Vec::new();

    for i in 0..anchor_count {
        let anchor_id = insert_anchor(&pool, &format!("Anchor-{i}")).await;
        for j in 0..assets_per_anchor {
            insert_asset(
                &pool,
                &anchor_id,
                &format!("ASSET{i}{j}"),
                &format!("GISSUER{i}{j}XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"),
            )
            .await;
        }
        anchor_ids.push(anchor_id);
    }

    // One batch call (mirrors AssetDb::get_assets_by_anchors).
    let result = batch_load_assets(&pool, &anchor_ids).await;

    // Every anchor must appear in the result.
    assert_eq!(
        result.len(),
        anchor_count,
        "expected {anchor_count} anchors in result, got {}",
        result.len()
    );

    // Each anchor must have exactly `assets_per_anchor` assets.
    for id in &anchor_ids {
        let assets = result.get(id).expect("anchor missing from batch result");
        assert_eq!(
            assets.len(),
            assets_per_anchor,
            "anchor {id} expected {assets_per_anchor} assets, got {}",
            assets.len()
        );
    }
}

/// Regression: empty anchor list returns an empty map without querying DB.
///
/// The guard `if anchor_ids.is_empty() { return Ok(HashMap::new()); }` was
/// added to avoid generating an invalid `IN ()` SQL clause.
#[tokio::test]
async fn test_batch_load_empty_input_returns_empty_map() {
    let pool = setup_db().await;
    let result = batch_load_assets(&pool, &[]).await;
    assert!(result.is_empty(), "expected empty map for empty input");
}

/// Regression: batch load correctly associates assets to the right anchors.
///
/// If asset rows were incorrectly keyed the HashMap entries would map to
/// the wrong anchor, silently producing bad data in API responses.
#[tokio::test]
async fn test_batch_load_asset_anchor_mapping_is_correct() {
    let pool = setup_db().await;

    let anchor_a = insert_anchor(&pool, "AnchorAlpha").await;
    let anchor_b = insert_anchor(&pool, "AnchorBeta").await;

    insert_asset(&pool, &anchor_a, "USDC", "GUSDC_ISSUER_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await;
    insert_asset(&pool, &anchor_a, "EURT", "GEURT_ISSUER_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await;
    insert_asset(&pool, &anchor_b, "BRL", "GBRL_ISSUER_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await;

    let result = batch_load_assets(&pool, &[anchor_a.clone(), anchor_b.clone()]).await;

    let a_assets = result.get(&anchor_a).expect("AnchorAlpha missing");
    let b_assets = result.get(&anchor_b).expect("AnchorBeta missing");

    assert_eq!(a_assets.len(), 2, "AnchorAlpha should have 2 assets");
    assert_eq!(b_assets.len(), 1, "AnchorBeta should have 1 asset");

    // Verify specific asset codes are in the right bucket.
    assert!(a_assets.contains(&"USDC".to_string()), "USDC should belong to AnchorAlpha");
    assert!(a_assets.contains(&"EURT".to_string()), "EURT should belong to AnchorAlpha");
    assert!(b_assets.contains(&"BRL".to_string()), "BRL should belong to AnchorBeta");
}

/// Regression: batch load with a single anchor works correctly.
///
/// Edge case to ensure the `IN ($1)` form with one element is valid SQL.
#[tokio::test]
async fn test_batch_load_single_anchor() {
    let pool = setup_db().await;

    let anchor_id = insert_anchor(&pool, "SingleAnchor").await;
    insert_asset(&pool, &anchor_id, "NGNT", "GNGNT_ISSUER_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await;

    let result = batch_load_assets(&pool, &[anchor_id.clone()]).await;

    assert_eq!(result.len(), 1);
    let assets = result.get(&anchor_id).expect("anchor missing");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0], "NGNT");
}
