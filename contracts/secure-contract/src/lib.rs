#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, symbol_short};

mod acl {
    soroban_sdk::contractimport!(
        file = "../access-control/target/wasm32-unknown-unknown/release/access_control.wasm"
    );
}

#[contract]
pub struct SecureContract;

#[contractimpl]
impl SecureContract {
    pub fn initialize(env: Env, admin: Address, acl_contract: Address) {
        admin.require_auth();
        env.storage().instance().set(&symbol_short!("acl"), &acl_contract);
        env.storage().instance().set(&symbol_short!("admin"), &admin);
    }

    pub fn protected_function(env: Env, caller: Address) -> u32 {
        caller.require_auth();
        Self::check_permission(&env, &caller, symbol_short!("protected"));
        42
    }

    pub fn admin_only(env: Env, caller: Address) -> bool {
        caller.require_auth();
        Self::check_permission(&env, &caller, symbol_short!("admin"));
        true
    }

    fn check_permission(env: &Env, user: &Address, function: Symbol) {
        let acl_addr: Address = env.storage().instance()
            .get(&symbol_short!("acl"))
            .expect("ACL not initialized");
        
        let acl_client = acl::Client::new(env, &acl_addr);
        
        if !acl_client.check_permission(user, &function) {
            panic!("Access denied");
        }
    }
}
