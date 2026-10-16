//! Multi-network: ensure testnet and mainnet data never mix.
//!
//! `ledger_payments` (and the other ledger ingestion tables) used to have no
//! `network` column at all, so rows ingested from mainnet and testnet were
//! indistinguishable once in the database. Migration
//! 048_add_network_to_ledger_ingestion.sql adds the column; this test
//! verifies that filtering by network actually isolates the data instead of
//! silently returning everyone's rows (or everyone's data getting the same
//! default).

use sqlx::sqlite::SqlitePool;

async fn setup() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

async fn insert_payment(pool: &SqlitePool, network: &str, destination: &str) {
    // Note: `ledgers.sequence` is a plain (non-composite) primary key, so a
    // real dual-network deployment ingesting the same sequence number on
    // both networks would collide here (`ON CONFLICT DO NOTHING`) — this
    // test only exercises `ledger_payments`, which has an autoincrement id
    // and is therefore safe to tag per-network without that caveat.
    sqlx::query(
        r"
        INSERT INTO ledgers (sequence, hash, close_time, network)
        VALUES ($1, $2, CURRENT_TIMESTAMP, $3)
        ON CONFLICT (sequence) DO NOTHING
        ",
    )
    .bind(1i64)
    .bind(format!("hash-{network}"))
    .bind(network)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r"
        INSERT INTO ledger_payments (ledger_sequence, transaction_hash, operation_type, source_account, destination, amount, network)
        VALUES ($1, $2, 'payment', $3, $4, '100', $5)
        ",
    )
    .bind(1i64)
    .bind(format!("tx-{network}"))
    .bind("GA7QYNF7SOWQ3GLR2BGMZEHXAVIRZA4KVWLTJJFC7MGXUA74P7UJVSGZ")
    .bind(destination)
    .bind(network)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn cross_network_query_returns_nothing() {
    let pool = setup().await;

    insert_payment(&pool, "testnet", "GBTESTDESTACCOUNT00000000000000000000000000000000000TEST").await;

    // Querying mainnet for data that was only ever ingested on testnet must
    // come back empty — not fall back to some default, and not return the
    // testnet row.
    let mainnet_rows: Vec<(String,)> =
        sqlx::query_as("SELECT destination FROM ledger_payments WHERE network = 'mainnet'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(
        mainnet_rows.is_empty(),
        "expected no mainnet rows, got {mainnet_rows:?}"
    );

    let testnet_rows: Vec<(String,)> =
        sqlx::query_as("SELECT destination FROM ledger_payments WHERE network = 'testnet'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(testnet_rows.len(), 1);
}

#[tokio::test]
async fn mainnet_and_testnet_payments_do_not_mix() {
    let pool = setup().await;

    insert_payment(&pool, "mainnet", "GAMAINNETDEST000000000000000000000000000000000000000A").await;
    insert_payment(&pool, "testnet", "GATESTNETDEST000000000000000000000000000000000000000B").await;

    let mainnet_rows: Vec<(String,)> =
        sqlx::query_as("SELECT destination FROM ledger_payments WHERE network = 'mainnet'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(mainnet_rows.len(), 1);
    assert_eq!(
        mainnet_rows[0].0,
        "GAMAINNETDEST000000000000000000000000000000000000000A"
    );

    let testnet_rows: Vec<(String,)> =
        sqlx::query_as("SELECT destination FROM ledger_payments WHERE network = 'testnet'")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(testnet_rows.len(), 1);
    assert_eq!(
        testnet_rows[0].0,
        "GATESTNETDEST000000000000000000000000000000000000000B"
    );

    let all_rows: Vec<(String,)> = sqlx::query_as("SELECT destination FROM ledger_payments")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(all_rows.len(), 2, "both rows should still exist untouched");
}
