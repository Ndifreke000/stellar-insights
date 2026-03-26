#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, BytesN, Map};

#[contract]
pub struct AnalyticsContract;

pub struct AnalyticsData {
    pub admin: Address,
    pub snapshots: Map<u64, BytesN<32>>,
}

#[contractimpl]
impl AnalyticsContract {
    pub fn initialize(env: Env, admin: Address) {
        let key = "admin";
        env.storage().instance().set(&key, &admin);
    }

    pub fn get_admin(env: Env) -> Address {
        let key = "admin";
        env.storage().instance().get(&key).unwrap()
    }

    pub fn submit_snapshot(
        env: Env,
        snapshot_id: u64,
        hash: BytesN<32>,
        _submitter: Address,
    ) -> Result<(), &'static str> {
        let key = ("snapshot", snapshot_id);
        env.storage().instance().set(&key, &hash);
        Ok(())
    }

    pub fn get_snapshot(env: Env, snapshot_id: u64) -> Option<BytesN<32>> {
        let key = ("snapshot", snapshot_id);
        env.storage().instance().get(&key)
    }
}
