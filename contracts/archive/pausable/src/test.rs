#![cfg(test)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn empty_reason(env: &Env) -> String {
    String::from_str(env, "")
}

fn reason(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn setup() -> (Env, PausableContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register_contract(None, PausableContract);
    let client = PausableContractClient::new(&env, &id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn test_initialize() {
    let (env, client, admin) = setup();
    assert_eq!(client.get_admin(), admin);
    assert!(!client.is_paused());
    assert_eq!(client.list_guardians().len(), 0);
    let _ = env;
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_double_initialize() {
    let (env, client, admin) = setup();
    client.initialize(&admin);
    let _ = env;
}

#[test]
fn test_admin_can_pause_and_unpause() {
    let (env, client, admin) = setup();
    client.pause(&admin, &reason(&env, "security incident"));
    assert!(client.is_paused());

    client.unpause(&admin, &reason(&env, "incident resolved"));
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_double_pause_rejected() {
    let (env, client, admin) = setup();
    client.pause(&admin, &empty_reason(&env));
    client.pause(&admin, &empty_reason(&env));
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_unpause_when_not_paused_rejected() {
    let (env, client, admin) = setup();
    client.unpause(&admin, &empty_reason(&env));
}

#[test]
fn test_guardian_can_pause_but_not_unpause() {
    let (env, client, admin) = setup();
    let guardian = Address::generate(&env);
    client.add_guardian(&admin, &guardian);
    assert!(client.is_guardian(&guardian));

    // Guardian can pause
    client.pause(&guardian, &reason(&env, "guardian triggered"));
    assert!(client.is_paused());

    // Guardian cannot unpause — check via try_unpause
    let result = client.try_unpause(&guardian, &empty_reason(&env));
    assert!(result.is_err());

    // Admin can unpause
    client.unpause(&admin, &empty_reason(&env));
    assert!(!client.is_paused());
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_stranger_cannot_pause() {
    let (env, client, _admin) = setup();
    let stranger = Address::generate(&env);
    client.pause(&stranger, &empty_reason(&env));
}

#[test]
fn test_add_and_remove_guardian() {
    let (env, client, admin) = setup();
    let guardian = Address::generate(&env);

    client.add_guardian(&admin, &guardian);
    assert_eq!(client.list_guardians().len(), 1);
    assert!(client.is_guardian(&guardian));

    client.remove_guardian(&admin, &guardian);
    assert_eq!(client.list_guardians().len(), 0);
    assert!(!client.is_guardian(&guardian));
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_add_duplicate_guardian() {
    let (env, client, admin) = setup();
    let guardian = Address::generate(&env);
    client.add_guardian(&admin, &guardian);
    client.add_guardian(&admin, &guardian);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_remove_non_existent_guardian() {
    let (env, client, admin) = setup();
    let guardian = Address::generate(&env);
    client.remove_guardian(&admin, &guardian);
}

#[test]
fn test_pause_history_recorded() {
    let (env, client, admin) = setup();

    client.pause(&admin, &reason(&env, "test pause"));
    client.unpause(&admin, &reason(&env, "test unpause"));

    let history = client.get_pause_history();
    assert_eq!(history.len(), 2);

    let pause_record = history.get(0).unwrap();
    assert!(pause_record.paused);

    let unpause_record = history.get(1).unwrap();
    assert!(!unpause_record.paused);
}

#[test]
fn test_multiple_guardians() {
    let (env, client, admin) = setup();
    let g1 = Address::generate(&env);
    let g2 = Address::generate(&env);
    let g3 = Address::generate(&env);

    client.add_guardian(&admin, &g1);
    client.add_guardian(&admin, &g2);
    client.add_guardian(&admin, &g3);

    assert_eq!(client.list_guardians().len(), 3);

    // g2 pauses
    client.pause(&g2, &reason(&env, "g2 triggered"));
    assert!(client.is_paused());

    // admin unpauses
    client.unpause(&admin, &empty_reason(&env));
    assert!(!client.is_paused());
}
