#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    let mut bytes = [seed; 32];
    bytes[0] = seed.wrapping_add(1);
    BytesN::from_array(env, &bytes)
}

fn setup() -> (Env, MultiAdminContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, MultiAdminContract);
    let client = MultiAdminContractClient::new(&env, &id);
    let super_admin = Address::generate(&env);
    client.initialize(&super_admin);
    (env, client, super_admin)
}

#[test]
fn test_initialize() {
    let (env, client, super_admin) = setup();
    assert!(client.is_admin(&super_admin));
    assert_eq!(client.get_role(&super_admin), Some(AdminRole::SuperAdmin));
    assert_eq!(client.list_admins().len(), 1);
    let _ = env;
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_double_initialize() {
    let (env, client, super_admin) = setup();
    client.initialize(&super_admin);
    let _ = env;
}

#[test]
fn test_add_admin_roles() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);
    let admin = Address::generate(&env);

    client.add_admin(&super_admin, &operator, &AdminRole::Operator);
    client.add_admin(&super_admin, &admin, &AdminRole::Admin);

    assert_eq!(client.get_role(&operator), Some(AdminRole::Operator));
    assert_eq!(client.get_role(&admin), Some(AdminRole::Admin));
    assert_eq!(client.list_admins().len(), 3);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_add_duplicate_admin() {
    let (env, client, super_admin) = setup();
    let new_admin = Address::generate(&env);
    client.add_admin(&super_admin, &new_admin, &AdminRole::Admin);
    client.add_admin(&super_admin, &new_admin, &AdminRole::Operator);
    let _ = env;
}

#[test]
fn test_remove_admin() {
    let (env, client, super_admin) = setup();
    let new_admin = Address::generate(&env);
    client.add_admin(&super_admin, &new_admin, &AdminRole::Admin);
    assert!(client.is_admin(&new_admin));

    client.remove_admin(&super_admin, &new_admin);
    assert!(!client.is_admin(&new_admin));
    assert_eq!(client.list_admins().len(), 1);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_cannot_remove_last_super_admin() {
    let (env, client, super_admin) = setup();
    client.remove_admin(&super_admin, &super_admin);
    let _ = env;
}

#[test]
fn test_change_role() {
    let (env, client, super_admin) = setup();
    let new_admin = Address::generate(&env);
    client.add_admin(&super_admin, &new_admin, &AdminRole::Operator);
    assert_eq!(client.get_role(&new_admin), Some(AdminRole::Operator));

    client.change_role(&super_admin, &new_admin, &AdminRole::Admin);
    assert_eq!(client.get_role(&new_admin), Some(AdminRole::Admin));
}

#[test]
fn test_operator_can_submit_snapshot() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);
    client.add_admin(&super_admin, &operator, &AdminRole::Operator);

    let hash = make_hash(&env, 1);
    client.submit_snapshot(&operator, &1u64, &hash);
    assert_eq!(client.get_latest_epoch(), 1u64);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_non_admin_cannot_submit() {
    let (env, client, _super_admin) = setup();
    let stranger = Address::generate(&env);
    let hash = make_hash(&env, 1);
    client.submit_snapshot(&stranger, &1u64, &hash);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_duplicate_epoch_rejected() {
    let (env, client, super_admin) = setup();
    let hash = make_hash(&env, 1);
    client.submit_snapshot(&super_admin, &1u64, &hash);
    client.submit_snapshot(&super_admin, &1u64, &hash);
    let _ = env;
}

#[test]
#[should_panic(expected = "Error(Contract, #11)")]
fn test_monotonicity_enforced() {
    let (env, client, super_admin) = setup();
    client.submit_snapshot(&super_admin, &5u64, &make_hash(&env, 5));
    client.submit_snapshot(&super_admin, &3u64, &make_hash(&env, 3)); // should fail
}

#[test]
fn test_pause_by_admin_role() {
    let (env, client, super_admin) = setup();
    let admin = Address::generate(&env);
    client.add_admin(&super_admin, &admin, &AdminRole::Admin);

    client.pause(&admin);
    assert!(client.is_paused());
    client.unpause(&admin);
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_operator_cannot_pause() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);
    client.add_admin(&super_admin, &operator, &AdminRole::Operator);
    client.pause(&operator);
    let _ = env;
}

#[test]
fn test_list_admin_entries() {
    let (env, client, super_admin) = setup();
    let operator = Address::generate(&env);
    client.add_admin(&super_admin, &operator, &AdminRole::Operator);

    let entries = client.list_admin_entries();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_get_snapshot() {
    let (env, client, super_admin) = setup();
    let hash = make_hash(&env, 77);
    client.submit_snapshot(&super_admin, &1u64, &hash);

    let snap = client.get_snapshot(&1u64);
    assert_eq!(snap.hash, hash);
    assert_eq!(snap.epoch, 1u64);
    assert_eq!(snap.submitter, super_admin);
}
