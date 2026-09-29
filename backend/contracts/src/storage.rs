use soroban_sdk::{contracttype, Address, Env};

/// Number of ledgers to extend the TTL by on each persistent storage write.
/// At ~5s per ledger this is roughly 30 days.
pub const LEDGERS_TO_EXTEND: u32 = 518_400;

/// Threshold (in ledgers) below which the TTL is refreshed on access/write.
/// At ~5s per ledger this is roughly 7 days.
pub const LEDGERS_TO_EXTEND_THRESHOLD: u32 = 120_960;

#[contracttype]
#[derive(Clone)]
pub struct StorageKey {
    pub owner: Address,
    pub id: u32,
}

/// Persist a value and extend its TTL so it does not expire unexpectedly.
pub fn set_persistent<T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>>(
    env: &Env,
    key: &StorageKey,
    value: &T,
) {
    env.storage().persistent().set(key, value);
    env.storage()
        .persistent()
        .extend_ttl(key, LEDGERS_TO_EXTEND_THRESHOLD, LEDGERS_TO_EXTEND);
}

/// Read a persistent value, refreshing its TTL when it is still present.
pub fn get_persistent<T: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>>(
    env: &Env,
    key: &StorageKey,
) -> Option<T> {
    let value = env.storage().persistent().get(key);
    if value.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(key, LEDGERS_TO_EXTEND_THRESHOLD, LEDGERS_TO_EXTEND);
    }
    value
}

/// Remove a persistent value from storage.
pub fn remove_persistent(env: &Env, key: &StorageKey) {
    env.storage().persistent().remove(key);
}
