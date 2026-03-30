use super::*;
use bolero::TypeGenerator;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

#[derive(TypeGenerator, Debug)]
struct FuzzInput {
    epoch: u64,
    hash: [u8; 32],
}

#[derive(TypeGenerator, Debug)]
struct TwoEpochInput {
    epoch_a: u64,
    epoch_b: u64,
    hash_a: [u8; 32],
    hash_b: [u8; 32],
}

// -----------------------------------------------------------------------
// 1. submit_snapshot – arbitrary inputs must never cause unexpected panics
// -----------------------------------------------------------------------
#[test]
fn fuzz_submit_snapshot() {
    bolero::check!().with_type::<FuzzInput>().for_each(|input| {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        env.mock_all_auths();
        client.initialize(&admin);

        let hash = BytesN::from_array(&env, &input.hash);

        // try_submit_snapshot returns Result – domain errors (epoch=0,
        // monotonicity violated) are acceptable; the contract must
        // never crash in an uncontrolled/unexpected way.
        let _ = client.try_submit_snapshot(&input.epoch, &hash, &admin);
    });
}

// -----------------------------------------------------------------------
// 2. get_snapshot – random epoch lookups must never panic
// -----------------------------------------------------------------------
#[test]
fn fuzz_get_snapshot() {
    bolero::check!().with_type::<u64>().for_each(|epoch| {
        let env = Env::default();
        let contract_id = env.register_contract(None, AnalyticsContract);
        let client = AnalyticsContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        env.mock_all_auths();
        client.initialize(&admin);

        // Seeding with one known snapshot so storage is non-empty
        let hash = BytesN::from_array(&env, &[42u8; 32]);
        let _ = client.try_submit_snapshot(&1u64, &hash, &admin);

        // Any arbitrary epoch lookup should return Some or None, never panic
        let _ = client.get_snapshot(epoch);
    });
}

// -----------------------------------------------------------------------
// 3. Sequential submits – strictly increasing epochs must always succeed
// -----------------------------------------------------------------------
#[test]
fn fuzz_sequential_submits() {
    bolero::check!()
        .with_type::<[u8; 32]>()
        .for_each(|hash_bytes| {
            let env = Env::default();
            let contract_id = env.register_contract(None, AnalyticsContract);
            let client = AnalyticsContractClient::new(&env, &contract_id);

            let admin = Address::generate(&env);
            env.mock_all_auths();
            client.initialize(&admin);

            // Submit three snapshots with guaranteed-increasing epochs
            let epochs = [1u64, 2u64, 3u64];
            for epoch in &epochs {
                let hash = BytesN::from_array(&env, hash_bytes);
                // Every call with a strictly greater epoch MUST succeed
                let result = client.try_submit_snapshot(epoch, &hash, &admin);
                assert!(result.is_ok(), "Expected Ok for epoch {epoch} but got Err");
            }

            // Monotonicity invariant: latest epoch equals the last submitted
            assert_eq!(client.get_latest_epoch(), 3u64);
        });
}

// -----------------------------------------------------------------------
// 4. Monotonicity invariant – lower/equal epoch must always be rejected
// -----------------------------------------------------------------------
#[test]
fn fuzz_monotonicity_invariant() {
    bolero::check!()
        .with_type::<TwoEpochInput>()
        .for_each(|input| {
            // Only test cases where epoch_a is non-zero and epoch_b < epoch_a
            // to exercise the rejection path
            if input.epoch_a == 0 || input.epoch_b == 0 {
                return;
            }
            if input.epoch_b >= input.epoch_a {
                return;
            }

            let env = Env::default();
            let contract_id = env.register_contract(None, AnalyticsContract);
            let client = AnalyticsContractClient::new(&env, &contract_id);

            let admin = Address::generate(&env);
            env.mock_all_auths();
            client.initialize(&admin);

            let hash_a = BytesN::from_array(&env, &input.hash_a);
            let hash_b = BytesN::from_array(&env, &input.hash_b);

            // Submit the higher epoch first – must succeed
            let first = client.try_submit_snapshot(&input.epoch_a, &hash_a, &admin);
            assert!(
                first.is_ok(),
                "First submit (epoch={}) should succeed",
                input.epoch_a
            );

            // Submit the lower epoch second – MUST be rejected
            let second = client.try_submit_snapshot(&input.epoch_b, &hash_b, &admin);
            assert!(
                second.is_err(),
                "Second submit (epoch={}) should have been rejected (epoch_a={})",
                input.epoch_b,
                input.epoch_a
            );

            // Latest epoch must remain unchanged (still epoch_a)
            assert_eq!(client.get_latest_epoch(), input.epoch_a);
        });
}
