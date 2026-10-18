use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};
use crate::AdminRole;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializedEvent {
    pub super_admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminAddedEvent {
    pub by: Address,
    pub new_admin: Address,
    pub role: AdminRole,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRemovedEvent {
    pub by: Address,
    pub removed: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleChangedEvent {
    pub by: Address,
    pub target: Address,
    pub new_role: AdminRole,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotSubmittedEvent {
    pub submitter: Address,
    pub epoch: u64,
    pub hash: BytesN<32>,
    pub timestamp: u64,
}

pub fn emit_initialized(env: &Env, super_admin: Address) {
    let event = InitializedEvent {
        super_admin,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("ma_init"),), event);
}

pub fn emit_admin_added(env: &Env, by: Address, new_admin: Address, role: AdminRole) {
    let event = AdminAddedEvent {
        by,
        new_admin,
        role,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("adm_add"),), event);
}

pub fn emit_admin_removed(env: &Env, by: Address, removed: Address) {
    let event = AdminRemovedEvent {
        by,
        removed,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("adm_rm"),), event);
}

pub fn emit_role_changed(env: &Env, by: Address, target: Address, new_role: AdminRole) {
    let event = RoleChangedEvent {
        by,
        target,
        new_role,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish((symbol_short!("role_chg"),), event);
}

pub fn emit_snapshot_submitted(
    env: &Env,
    submitter: Address,
    epoch: u64,
    hash: BytesN<32>,
    timestamp: u64,
) {
    let event = SnapshotSubmittedEvent {
        submitter,
        epoch,
        hash,
        timestamp,
    };
    env.events().publish((symbol_short!("snap_sub"),), event);
}
