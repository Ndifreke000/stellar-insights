use soroban_sdk::{contracttype, symbol_short, Address, Env, String};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializedEvent {
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PausedEvent {
    pub triggered_by: Address,
    pub reason: String,
    pub timestamp: u64,
    pub ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpausedEvent {
    pub triggered_by: Address,
    pub reason: String,
    pub timestamp: u64,
    pub ledger: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardianAddedEvent {
    pub by: Address,
    pub guardian: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuardianRemovedEvent {
    pub by: Address,
    pub guardian: Address,
    pub timestamp: u64,
}

pub fn emit_initialized(env: &Env, admin: Address) {
    let event = InitializedEvent {
        admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("p_init"),), event);
}

pub fn emit_paused(env: &Env, triggered_by: Address, reason: String) {
    let event = PausedEvent {
        triggered_by,
        reason,
        timestamp: env.ledger().timestamp(),
        ledger: env.ledger().sequence(),
    };
    env.events().publish((symbol_short!("paused"),), event);
}

pub fn emit_unpaused(env: &Env, triggered_by: Address, reason: String) {
    let event = UnpausedEvent {
        triggered_by,
        reason,
        timestamp: env.ledger().timestamp(),
        ledger: env.ledger().sequence(),
    };
    env.events().publish((symbol_short!("unpaused"),), event);
}

pub fn emit_guardian_added(env: &Env, by: Address, guardian: Address) {
    let event = GuardianAddedEvent {
        by,
        guardian,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("g_add"),), event);
}

pub fn emit_guardian_removed(env: &Env, by: Address, guardian: Address) {
    let event = GuardianRemovedEvent {
        by,
        guardian,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("g_rm"),), event);
}
