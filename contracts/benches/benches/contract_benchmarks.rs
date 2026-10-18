//! Wall-clock benchmarks for the `payraider` contract, using Criterion.
//!
//! Informational only — timings vary with the machine, so CI gates on the
//! deterministic host-budget figures from `gas_benchmarks.rs` instead.
//!
//! Run with `cargo bench -p contract-benches --bench contract_benchmarks`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use payraider::{PayRaiderContract, PayRaiderContractClient};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

fn setup_payraider(env: &Env) -> (PayRaiderContractClient<'_>, Address) {
    let contract_id = env.register(PayRaiderContract, ());
    let client = PayRaiderContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn make_hash(env: &Env, epoch: u64) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&epoch.to_be_bytes());
    bytes[31] = 1; // never all-zero
    BytesN::from_array(env, &bytes)
}

fn bench_payraider_submit(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_payraider(&env);
    let mut epoch = 1u64;

    c.bench_function("payraider::submit_snapshot", |b| {
        b.iter(|| {
            let hash = make_hash(&env, epoch);
            client.submit_snapshot(black_box(&epoch), black_box(&hash), black_box(&admin));
            epoch += 1;
        })
    });
}

fn bench_payraider_get(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_payraider(&env);

    for epoch in 1u64..=100 {
        client.submit_snapshot(&epoch, &make_hash(&env, epoch), &admin);
    }

    c.bench_function("payraider::get_snapshot", |b| {
        b.iter(|| client.get_snapshot(black_box(&50u64)))
    });
}

fn bench_payraider_latest(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_payraider(&env);

    for epoch in 1u64..=50 {
        client.submit_snapshot(&epoch, &make_hash(&env, epoch), &admin);
    }

    c.bench_function("payraider::latest_snapshot", |b| {
        b.iter(|| client.latest_snapshot())
    });
}

criterion_group!(
    payraider_benches,
    bench_payraider_submit,
    bench_payraider_get,
    bench_payraider_latest,
);

criterion_main!(payraider_benches);
