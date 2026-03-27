//! Corridor and Analytics Benchmarks
//!
//! This benchmark suite measures performance of corridor-related operations:
//! - Corridor creation and normalization
//! - Corridor key generation
//! - Payment record processing
//! - Median computation for settlement latencies
//!
//! # Usage
//! ```bash
//! # Run all benchmarks
//! cargo bench --bench corridor_benchmarks
//!
//! # Run specific benchmark
//! cargo bench --bench corridor_benchmarks -- corridor_creation
//!
//! # Save baseline for comparison
//! cargo bench --bench corridor_benchmarks -- --save-baseline main
//!
//! # Compare against baseline
//! cargo bench --bench corridor_benchmarks -- --baseline main
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use stellar_insights_backend::models::corridor::{compute_median, Corridor, PaymentRecord};
use uuid::Uuid;

/// Benchmark corridor creation with normalization
fn bench_corridor_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("corridor_creation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("new_corridor_normalized", |b| {
        b.iter(|| {
            Corridor::new(
                black_box("USDC".to_string()),
                black_box("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string()),
                black_box("EURC".to_string()),
                black_box("GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y3IEMRRVXNWLSFYWM5V2J7LT".to_string()),
            )
        });
    });

    group.bench_function("new_corridor_same_asset", |b| {
        b.iter(|| {
            Corridor::new(
                black_box("XLM".to_string()),
                black_box("native".to_string()),
                black_box("XLM".to_string()),
                black_box("native".to_string()),
            )
        });
    });

    group.finish();
}

/// Benchmark corridor key generation
fn bench_corridor_key_generation(c: &mut Criterion) {
    let corridor = Corridor::new(
        "USDC".to_string(),
        "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        "EURC".to_string(),
        "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y3IEMRRVXNWLSFYWM5V2J7LT".to_string(),
    );

    c.bench_function("corridor_to_string_key", |b| {
        b.iter(|| black_box(&corridor).to_string_key())
    });
}

/// Benchmark payment record creation and corridor extraction
fn bench_payment_record_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("payment_record");
    group.throughput(Throughput::Elements(1));

    let source_asset = "USDC".to_string();
    let source_issuer = "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string();
    let dest_asset = "EURC".to_string();
    let dest_issuer = "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y3IEMRRVXNWLSFYWM5V2J7LT".to_string();

    group.bench_function("create_payment_record", |b| {
        b.iter(|| {
            PaymentRecord {
                id: Uuid::new_v4(),
                source_asset_code: black_box(source_asset.clone()),
                source_asset_issuer: black_box(source_issuer.clone()),
                destination_asset_code: black_box(dest_asset.clone()),
                destination_asset_issuer: black_box(dest_issuer.clone()),
                amount: black_box(100.0),
                successful: black_box(true),
                timestamp: chrono::Utc::now(),
                submission_time: None,
                confirmation_time: None,
            }
        });
    });

    let payment = PaymentRecord {
        id: Uuid::new_v4(),
        source_asset_code: source_asset.clone(),
        source_asset_issuer: source_issuer.clone(),
        destination_asset_code: dest_asset.clone(),
        destination_asset_issuer: dest_issuer.clone(),
        amount: 100.0,
        successful: true,
        timestamp: chrono::Utc::now(),
        submission_time: Some(chrono::Utc::now() - chrono::Duration::seconds(5)),
        confirmation_time: Some(chrono::Utc::now()),
    };

    group.bench_function("payment_get_corridor", |b| {
        b.iter(|| black_box(&payment).get_corridor())
    });

    group.bench_function("payment_settlement_latency", |b| {
        b.iter(|| black_box(&payment).settlement_latency_ms())
    });

    group.finish();
}

/// Benchmark median computation for settlement latencies
fn bench_median_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("median_computation");

    // Test with different dataset sizes
    for size in [10, 100, 1000, 10000].iter() {
        let mut values: Vec<i64> = (0..*size)
            .map(|i| (i % 5000) as i64)
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("size_{}", size)),
            &mut values,
            |b, values| {
                b.iter(|| {
                    let mut test_values = values.clone();
                    compute_median(black_box(&mut test_values))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark corridor comparison operations
fn bench_corridor_comparison(c: &mut Criterion) {
    let corridor1 = Corridor::new(
        "USDC".to_string(),
        "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
        "EURC".to_string(),
        "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y3IEMRRVXNWLSFYWM5V2J7LT".to_string(),
    );

    let corridor2 = Corridor::new(
        "EURC".to_string(),
        "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y3IEMRRVXNWLSFYWM5V2J7LT".to_string(),
        "USDC".to_string(),
        "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN".to_string(),
    );

    let mut group = c.benchmark_group("corridor_comparison");

    group.bench_function("corridor_equality_check", |b| {
        b.iter(|| black_box(&corridor1) == black_box(&corridor2))
    });

    group.bench_function("corridor_clone", |b| {
        b.iter(|| black_box(&corridor1).clone())
    });

    group.finish();
}

/// Benchmark batch corridor operations
fn bench_batch_corridor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_corridor");

    for batch_size in [10, 100, 1000].iter() {
        let corridors: Vec<Corridor> = (0..*batch_size)
            .map(|i| {
                Corridor::new(
                    format!("ASSET_{}", i % 10),
                    format!("ISSUER_{}", i % 5),
                    format!("ASSET_{}", (i + 1) % 10),
                    format!("ISSUER_{}", (i + 2) % 5),
                )
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            &corridors,
            |b, corridors| {
                b.iter(|| {
                    for corridor in black_box(corridors) {
                        let _ = corridor.to_string_key();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_corridor_creation,
    bench_corridor_key_generation,
    bench_payment_record_operations,
    bench_median_computation,
    bench_corridor_comparison,
    bench_batch_corridor_operations,
);

criterion_main!(benches);
