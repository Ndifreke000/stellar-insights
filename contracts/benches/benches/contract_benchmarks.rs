use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use analytics::{AnalyticsContract, AnalyticsContractClient};
use stellar_insights::{StellarInsightsContract, StellarInsightsContractClient};

// ============================================================================
// Helpers
// ============================================================================

fn setup_analytics(env: &Env) -> (AnalyticsContractClient, Address) {
    let contract_id = env.register_contract(None, AnalyticsContract);
    let client = AnalyticsContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin).unwrap();
    (client, admin)
}

fn setup_stellar_insights(env: &Env) -> (StellarInsightsContractClient, Address) {
    let contract_id = env.register_contract(None, StellarInsightsContract);
    let client = StellarInsightsContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

// ============================================================================
// bench_submit_snapshot
// Measures cost of submitting a single snapshot to AnalyticsContract.
// Each iteration uses a fresh, strictly-increasing epoch.
// ============================================================================

fn bench_submit_snapshot(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_analytics(&env);
    let mut epoch = 1u64;

    c.bench_function("analytics::submit_snapshot", |b| {
        b.iter(|| {
            let hash = make_hash(&env, (epoch % 255) as u8);
            client
                .submit_snapshot(black_box(&epoch), black_box(&hash), black_box(&admin))
                .unwrap();
            epoch += 1;
        })
    });
}

// ============================================================================
// bench_get_snapshot
// Measures retrieval cost after pre-populating 100 snapshots.
// ============================================================================

fn bench_get_snapshot(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_analytics(&env);

    for epoch in 1u64..=100 {
        let hash = make_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin).unwrap();
    }

    c.bench_function("analytics::get_snapshot", |b| {
        b.iter(|| client.get_snapshot(black_box(&50u64)))
    });
}

// ============================================================================
// bench_get_latest_snapshot
// Measures retrieval cost of the latest snapshot pointer.
// ============================================================================

fn bench_get_latest_snapshot(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_analytics(&env);

    for epoch in 1u64..=50 {
        let hash = make_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin).unwrap();
    }

    c.bench_function("analytics::get_latest_snapshot", |b| {
        b.iter(|| client.get_latest_snapshot())
    });
}

// ============================================================================
// bench_batch_operations
// Measures cost of submitting N snapshots in sequence.
// Parameterised over batch sizes: 5, 10, 25, 50.
// A fresh Env is created per iteration to keep epochs valid.
// ============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("analytics::batch_submit");

    for batch_size in [5u64, 10, 25, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    let env = Env::default();
                    let (client, admin) = setup_analytics(&env);
                    for epoch in 1..=size {
                        let hash = make_hash(&env, (epoch % 255) as u8);
                        client
                            .submit_snapshot(black_box(&epoch), black_box(&hash), black_box(&admin))
                            .unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// bench_snapshot_history_growth
// Measures get_snapshot_history read cost as the map grows.
// ============================================================================

fn bench_snapshot_history_growth(c: &mut Criterion) {
    let mut group = c.benchmark_group("analytics::history_read");

    for size in [10u64, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let env = Env::default();
            let (client, admin) = setup_analytics(&env);
            for epoch in 1..=n {
                let hash = make_hash(&env, (epoch % 255) as u8);
                client.submit_snapshot(&epoch, &hash, &admin).unwrap();
            }
            b.iter(|| client.get_snapshot_history())
        });
    }

    group.finish();
}

// ============================================================================
// bench_stellar_insights_submit
// Submit benchmark against the stellar_insights contract variant.
// Result is unwrapped — a panic here indicates a contract regression.
// ============================================================================

fn bench_stellar_insights_submit(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_stellar_insights(&env);
    let mut epoch = 1u64;

    c.bench_function("stellar_insights::submit_snapshot", |b| {
        b.iter(|| {
            let hash = make_hash(&env, (epoch % 255) as u8);
            client
                .submit_snapshot(black_box(&epoch), black_box(&hash), black_box(&admin))
                .unwrap();
            epoch += 1;
        })
    });
}

// ============================================================================
// bench_stellar_insights_get
// Measures get_snapshot cost on the stellar_insights contract.
// ============================================================================

fn bench_stellar_insights_get(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_stellar_insights(&env);

    for epoch in 1u64..=100 {
        let hash = make_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin).unwrap();
    }

    c.bench_function("stellar_insights::get_snapshot", |b| {
        b.iter(|| client.get_snapshot(black_box(&50u64)).unwrap())
    });
}

// ============================================================================
// bench_stellar_insights_latest
// Measures latest_snapshot cost on the stellar_insights contract.
// ============================================================================

fn bench_stellar_insights_latest(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_stellar_insights(&env);

    for epoch in 1u64..=50 {
        let hash = make_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin).unwrap();
    }

    c.bench_function("stellar_insights::latest_snapshot", |b| {
        b.iter(|| client.latest_snapshot().unwrap())
    });
}

// ============================================================================
// Registration
// ============================================================================

criterion_group!(
    analytics_benches,
    bench_submit_snapshot,
    bench_get_snapshot,
    bench_get_latest_snapshot,
    bench_batch_operations,
    bench_snapshot_history_growth,
);

criterion_group!(
    stellar_insights_benches,
    bench_stellar_insights_submit,
    bench_stellar_insights_get,
    bench_stellar_insights_latest,
);

criterion_main!(analytics_benches, stellar_insights_benches);
