use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};
use payraider::{PayRaiderContract, PayRaiderContractClient};

fn setup_payraider(env: &Env) -> (PayRaiderContractClient, Address) {
    let contract_id = env.register_contract(None, PayRaiderContract);
    let client = PayRaiderContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    env.mock_all_auths();
    client.initialize(&admin);
    (client, admin)
}

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn bench_payraider_submit(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_payraider(&env);
    let mut epoch = 1u64;

    c.bench_function("payraider::submit_snapshot", |b| {
        b.iter(|| {
            let hash = make_hash(&env, (epoch % 255) as u8);
            client
                .submit_snapshot(black_box(&epoch), black_box(&hash), black_box(&admin))
                .unwrap();
            epoch += 1;
        })
    });
}

fn bench_payraider_get(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_payraider(&env);

    for epoch in 1u64..=100 {
        let hash = make_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin).unwrap();
    }

    c.bench_function("payraider::get_snapshot", |b| {
        b.iter(|| client.get_snapshot(black_box(&50u64)).unwrap())
    });
}

fn bench_payraider_latest(c: &mut Criterion) {
    let env = Env::default();
    let (client, admin) = setup_payraider(&env);

    for epoch in 1u64..=50 {
        let hash = make_hash(&env, (epoch % 255) as u8);
        client.submit_snapshot(&epoch, &hash, &admin).unwrap();
    }

    c.bench_function("payraider::latest_snapshot", |b| {
        b.iter(|| client.latest_snapshot().unwrap())
    });
}

criterion_group!(
    payraider_benches,
    bench_payraider_submit,
    bench_payraider_get,
    bench_payraider_latest,
);

criterion_main!(payraider_benches);
