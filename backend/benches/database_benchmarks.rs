//! Database Benchmarks
//!
//! This benchmark suite measures performance of database operations:
//! - Connection pool management
//! - Query execution
//! - Batch operations
//! - Transaction handling
//!
//! # Usage
//! ```bash
//! # Run all database benchmarks
//! cargo bench --bench database_benchmarks
//!
//! # Run specific benchmark
//! cargo bench --bench database_benchmarks -- db_connection
//!
//! # Note: These benchmarks use an in-memory SQLite database
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sqlx::SqlitePool;
use tokio::runtime::Runtime;

/// Setup an in-memory SQLite database for benchmarking
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create test database pool");

    // Create test tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS corridors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            corridor_key TEXT NOT NULL UNIQUE,
            asset_a_code TEXT NOT NULL,
            asset_a_issuer TEXT NOT NULL,
            asset_b_code TEXT NOT NULL,
            asset_b_issuer TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create corridors table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS corridor_metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            corridor_key TEXT NOT NULL,
            date DATE NOT NULL,
            total_transactions INTEGER NOT NULL,
            successful_transactions INTEGER NOT NULL,
            failed_transactions INTEGER NOT NULL,
            success_rate REAL NOT NULL,
            volume_usd REAL NOT NULL,
            avg_settlement_latency_ms INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (corridor_key) REFERENCES corridors(corridor_key)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create corridor_metrics table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS anchors (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            stellar_account TEXT NOT NULL,
            home_domain TEXT NOT NULL,
            status TEXT NOT NULL,
            reliability_score REAL NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create anchors table");

    // Seed with test data
    seed_test_data(&pool).await;

    pool
}

/// Seed test data for benchmarks
async fn seed_test_data(pool: &SqlitePool) {
    // Insert test corridors
    for i in 0..100 {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO corridors 
            (corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(format!("corridor_{}", i))
        .bind(format!("ASSET_{}", i % 10))
        .bind(format!("ISSUER_{}", i % 5))
        .bind(format!("ASSET_{}", (i + 1) % 10))
        .bind(format!("ISSUER_{}", (i + 2) % 5))
        .execute(pool)
        .await
        .unwrap();
    }

    // Insert test metrics
    for i in 0..1000 {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO corridor_metrics 
            (corridor_key, date, total_transactions, successful_transactions, 
             failed_transactions, success_rate, volume_usd, avg_settlement_latency_ms)
            VALUES (?, date('now'), ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(format!("corridor_{}", i % 100))
        .bind(1000 + (i % 100))
        .bind(950 + (i % 50))
        .bind(50 - (i % 50))
        .bind(0.95)
        .bind(100_000.0 + (i as f64 * 100.0))
        .bind(Some(150 + (i % 100)))
        .execute(pool)
        .await
        .unwrap();
    }

    // Insert test anchors
    for i in 0..50 {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO anchors 
            (id, name, stellar_account, home_domain, status, reliability_score)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(format!("anchor_{}", i))
        .bind(format!("Anchor {}", i))
        .bind(format!("G{}XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX", i))
        .bind(format!("anchor{}.example.com", i))
        .bind("active")
        .bind(0.95 + (i as f64 * 0.001))
        .execute(pool)
        .await
        .unwrap();
    }
}

fn bench_db_connection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("db_connection");
    group.throughput(Throughput::Elements(1));

    group.bench_function("create_pool", |b| {
        b.iter(|| rt.block_on(async { SqlitePool::connect("sqlite::memory:").await.unwrap() }));
    });

    group.bench_function("pool_ping", |b| {
        let pool = rt.block_on(setup_test_db());
        b.iter(|| rt.block_on(async { sqlx::query("SELECT 1").fetch_one(&pool).await.unwrap() }));
    });

    group.finish();
}

fn bench_db_read_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_db());

    let mut group = c.benchmark_group("db_read");
    group.throughput(Throughput::Elements(1));

    // Single row lookup by primary key
    group.bench_function("select_by_id", |b| {
        b.to_async(&rt).iter(|| async {
            sqlx::query("SELECT * FROM corridors WHERE id = ?")
                .bind(black_box(1))
                .fetch_one(&pool)
                .await
                .unwrap()
        });
    });

    // Lookup by indexed column
    group.bench_function("select_by_corridor_key", |b| {
        b.to_async(&rt).iter(|| async {
            sqlx::query("SELECT * FROM corridors WHERE corridor_key = ?")
                .bind(black_box("corridor_50"))
                .fetch_one(&pool)
                .await
                .unwrap()
        });
    });

    // Aggregation query
    group.bench_function("aggregate_count", |b| {
        b.to_async(&rt).iter(|| async {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM corridor_metrics")
                .fetch_one(&pool)
                .await
                .unwrap()
        });
    });

    // Aggregation with grouping
    group.bench_function("aggregate_group_by", |b| {
        b.to_async(&rt).iter(|| async {
            sqlx::query(
                "SELECT corridor_key, COUNT(*), AVG(success_rate) 
                 FROM corridor_metrics 
                 GROUP BY corridor_key",
            )
            .fetch_all(&pool)
            .await
            .unwrap()
        });
    });

    group.finish();
}

fn bench_db_write_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_db());

    let mut group = c.benchmark_group("db_write");
    group.throughput(Throughput::Elements(1));

    // Single insert
    group.bench_function("insert_single", |b| {
        b.to_async(&rt).iter(|| async {
            let i = black_box(1000);
            sqlx::query(
                "INSERT INTO corridors (corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(format!("test_corridor_{}", i))
            .bind("USDC")
            .bind("TEST_ISSUER")
            .bind("EURC")
            .bind("TEST_ISSUER_2")
            .execute(&pool)
            .await
            .unwrap()
        });
    });

    // Single update
    group.bench_function("update_single", |b| {
        b.to_async(&rt).iter(|| async {
            sqlx::query("UPDATE corridors SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(black_box(1))
                .execute(&pool)
                .await
                .unwrap()
        });
    });

    // Single delete
    group.bench_function("delete_single", |b| {
        b.to_async(&rt).iter(|| async {
            sqlx::query("DELETE FROM corridors WHERE corridor_key LIKE ?")
                .bind(black_box("test_corridor_%"))
                .execute(&pool)
                .await
                .unwrap()
        });
    });

    group.finish();
}

fn bench_db_batch_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_db());

    let mut group = c.benchmark_group("db_batch");

    for batch_size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_insert_{}", batch_size)),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let mut tx = pool.begin().await.unwrap();
                    for i in 0..batch_size {
                        sqlx::query(
                            "INSERT INTO corridors (corridor_key, asset_a_code, asset_a_issuer, asset_b_code, asset_b_issuer)
                             VALUES (?, ?, ?, ?, ?)"
                        )
                        .bind(format!("batch_corridor_{}_{}", batch_size, i))
                        .bind("USDC")
                        .bind("BATCH_ISSUER")
                        .bind("EURC")
                        .bind("BATCH_ISSUER_2")
                        .execute(&mut *tx)
                        .await
                        .unwrap();
                    }
                    tx.commit().await.unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_db_transaction_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_db());

    let mut group = c.benchmark_group("db_transaction");
    group.throughput(Throughput::Elements(1));

    group.bench_function("transaction_read_only", |b| {
        b.to_async(&rt).iter(|| async {
            let mut tx = pool.begin().await.unwrap();
            let result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM corridors")
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            tx.rollback().await.unwrap();
            result
        });
    });

    group.bench_function("transaction_write", |b| {
        b.to_async(&rt).iter(|| async {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("UPDATE anchors SET reliability_score = ? WHERE id = ?")
                .bind(black_box(0.99))
                .bind(black_box("anchor_1"))
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        });
    });

    group.finish();
}

fn bench_db_pool_metrics(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_db());

    let mut group = c.benchmark_group("db_pool_metrics");

    group.bench_function("get_pool_size", |b| b.iter(|| black_box(&pool).size()));

    group.bench_function("get_pool_idle", |b| b.iter(|| black_box(&pool).num_idle()));

    group.finish();
}

criterion_group!(
    benches,
    bench_db_connection,
    bench_db_read_operations,
    bench_db_write_operations,
    bench_db_batch_operations,
    bench_db_transaction_operations,
    bench_db_pool_metrics,
);

criterion_main!(benches);
