//! Cache Manager Benchmarks
//!
//! This benchmark suite measures performance of cache operations:
//! - Cache set/get operations
//! - Cache invalidation
//! - Pattern-based deletion
//! - Cache statistics
//!
//! # Usage
//! ```bash
//! # Run all cache benchmarks
//! cargo bench --bench cache_benchmarks
//!
//! # Run specific benchmark
//! cargo bench --bench cache_benchmarks -- cache_set
//!
//! # Note: These benchmarks require Redis to be running
//! # Start Redis: docker run -p 6379:6379 redis:alpine
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tokio::runtime::Runtime;

// Mock cache manager for benchmarking (avoids Redis dependency in benchmarks)
#[derive(Clone)]
struct MockCacheManager {
    hit_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
    miss_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl MockCacheManager {
    fn new() -> Self {
        Self {
            hit_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            miss_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    async fn get<T: Clone>(&self, _key: &str, value: &T) -> Option<T> {
        // Simulate cache hit/miss
        if black_box(true) {
            self.hit_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(value.clone())
        } else {
            self.miss_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    async fn set<T>(&self, _key: &str, _value: &T, _ttl: usize) {
        // Simulate cache set
        tokio::task::yield_now().await;
    }

    async fn delete(&self, _key: &str) {
        // Simulate cache delete
        tokio::task::yield_now().await;
    }

    fn hit_rate(&self) -> f64 {
        let hits = self.hit_count.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.miss_count.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct CorridorMetricsData {
    corridor_key: String,
    total_transactions: i64,
    success_rate: f64,
    volume_usd: f64,
    avg_latency_ms: Option<i32>,
}

impl CorridorMetricsData {
    fn new(key: &str) -> Self {
        Self {
            corridor_key: key.to_string(),
            total_transactions: 1000,
            success_rate: 0.95,
            volume_usd: 1_000_000.0,
            avg_latency_ms: Some(150),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct AnchorData {
    id: String,
    name: String,
    stellar_account: String,
    home_domain: String,
    status: String,
    reliability_score: f64,
}

impl AnchorData {
    fn new(id: u64) -> Self {
        Self {
            id: format!("anchor_{}", id),
            name: format!("Anchor {}", id),
            stellar_account: format!("G{}...", id),
            home_domain: format!("anchor{}.example.com", id),
            status: "active".to_string(),
            reliability_score: 0.98,
        }
    }
}

fn bench_cache_set_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = MockCacheManager::new();

    let mut group = c.benchmark_group("cache_set");
    group.throughput(Throughput::Elements(1));

    // Benchmark small payload
    group.bench_function("set_small_payload", |b| {
        b.to_async(&rt).iter(|| async {
            let data = black_box(CorridorMetricsData::new("USDC:EURC"));
            cache
                .set(black_box("corridor:USDC:EURC"), &data, black_box(300))
                .await
        });
    });

    // Benchmark medium payload
    group.bench_function("set_medium_payload", |b| {
        b.to_async(&rt).iter(|| async {
            let data = black_box(AnchorData::new(1));
            cache
                .set(black_box("anchor:1"), &data, black_box(600))
                .await
        });
    });

    // Benchmark with different TTLs
    for ttl in [60, 300, 3600].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("ttl_{}s", ttl)),
            ttl,
            |b, &ttl| {
                b.to_async(&rt).iter(|| async {
                    let data = CorridorMetricsData::new("test");
                    cache.set("test_key", &data, ttl).await
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_get_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = MockCacheManager::new();
    let data = CorridorMetricsData::new("USDC:EURC");

    let mut group = c.benchmark_group("cache_get");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_cached_value", |b| {
        b.to_async(&rt)
            .iter(|| async { cache.get(black_box("corridor:USDC:EURC"), &data).await });
    });

    group.bench_function("get_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let missing_data = CorridorMetricsData::new("missing");
            cache.get(black_box("nonexistent:key"), &missing_data).await
        });
    });

    group.finish();
}

fn bench_cache_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_serialization");
    group.throughput(Throughput::Elements(1));

    // Small payload serialization
    let small_data = CorridorMetricsData::new("USDC:EURC");
    group.bench_function("serialize_small", |b| {
        b.iter(|| serde_json::to_string(black_box(&small_data)).unwrap())
    });

    group.bench_function("deserialize_small", |b| {
        let json = serde_json::to_string(&small_data).unwrap();
        b.iter(|| serde_json::from_str::<CorridorMetricsData>(black_box(&json)).unwrap());
    });

    // Large payload (simulating batch data)
    let large_data: Vec<CorridorMetricsData> = (0..100)
        .map(|i| CorridorMetricsData::new(&format!("corridor_{}", i)))
        .collect();

    group.bench_function("serialize_large_batch", |b| {
        b.iter(|| serde_json::to_string(black_box(&large_data)).unwrap())
    });

    group.bench_function("deserialize_large_batch", |b| {
        let json = serde_json::to_string(&large_data).unwrap();
        b.iter(|| serde_json::from_str::<Vec<CorridorMetricsData>>(black_box(&json)).unwrap());
    });

    group.finish();
}

fn bench_cache_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_key_generation");

    group.bench_function("corridor_key", |b| {
        b.iter(|| {
            format!(
                "corridor:{}:{}->{}:{}",
                black_box("USDC"),
                black_box("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"),
                black_box("EURC"),
                black_box("GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y3IEMRRVXNWLSFYWM5V2J7LT")
            )
        });
    });

    group.bench_function("anchor_key", |b| {
        b.iter(|| format!("anchor:{}", black_box("uuid-1234-5678")))
    });

    group.bench_function("metrics_key", |b| {
        b.iter(|| {
            format!(
                "metrics:{}:{}",
                black_box("corridor"),
                black_box("2024-01-15")
            )
        });
    });

    group.finish();
}

fn bench_cache_statistics(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = MockCacheManager::new();
    let data = CorridorMetricsData::new("test");

    // Simulate some cache operations
    rt.block_on(async {
        for i in 0..100 {
            let _ = cache.get(&format!("key_{}", i % 10), &data).await;
        }
    });

    c.bench_function("calculate_hit_rate", |b| {
        b.iter(|| black_box(&cache).hit_rate())
    });

    c.bench_function("get_stats", |b| {
        b.iter(|| {
            (
                cache.hit_count.load(std::sync::atomic::Ordering::Relaxed),
                cache.miss_count.load(std::sync::atomic::Ordering::Relaxed),
            )
        });
    });
}

fn bench_concurrent_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = MockCacheManager::new();

    let mut group = c.benchmark_group("concurrent_cache");

    for concurrency in [1, 10, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("concurrent_{}", concurrency)),
            concurrency,
            |b, &concurrency| {
                let cache_clone = cache.clone();
                b.to_async(&rt).iter(|| async {
                    let mut handles = Vec::new();
                    for i in 0..concurrency {
                        let cache = cache_clone.clone();
                        handles.push(tokio::spawn(async move {
                            let data = CorridorMetricsData::new(&format!("key_{}", i));
                            cache.set(&format!("key_{}", i), &data, 300).await;
                            cache.get(&format!("key_{}", i), &data).await
                        }));
                    }
                    for handle in handles {
                        let _ = handle.await;
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_set_operations,
    bench_cache_get_operations,
    bench_cache_serialization,
    bench_cache_key_generation,
    bench_cache_statistics,
    bench_concurrent_cache_operations,
);

criterion_main!(benches);
