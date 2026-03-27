#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

#[derive(Clone)]
pub enum Role {
    Admin,
    Submitter,
    Viewer,
}

#[contract]
pub struct AccessControlContract;

#[contractimpl]
impl AccessControlContract {
    pub fn initialize(env: Env, admin: Address) {
        let key = "admin";
        env.storage().instance().set(&key, &admin);
    }

    pub fn get_admin(env: Env) -> Address {
        let key = "admin";
        env.storage().instance().get(&key).unwrap()
    }

    pub fn grant_role(env: Env, _admin: Address, user: Address, _role: Symbol) {
        let key = ("role", user.clone());
        env.storage().instance().set(&key, &user);
    }

    pub fn has_role(env: Env, user: Address, _role: Symbol) -> bool {
        let key = ("role", user.clone());
        env.storage().instance().get::<_, Address>(&key).is_ok()
    }

    pub fn revoke_role(env: Env, _admin: Address, user: Address) {
        let key = ("role", user.clone());
        env.storage().instance().remove(&key);
    }
}
