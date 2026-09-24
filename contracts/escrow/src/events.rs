use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("ESC_INI"),), admin);
}

pub fn emit_escrow_created(env: &Env, escrow_id: u64, depositor: Address, beneficiary: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("ESC_CRT"),),
        (escrow_id, depositor, beneficiary, amount),
    );
}

pub fn emit_escrow_funded(env: &Env, escrow_id: u64, depositor: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("ESC_FND"),),
        (escrow_id, depositor, amount),
    );
}

pub fn emit_funds_released(env: &Env, escrow_id: u64, beneficiary: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("ESC_REL"),),
        (escrow_id, beneficiary, amount),
    );
}

pub fn emit_refunded(env: &Env, escrow_id: u64, depositor: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("ESC_RFD"),),
        (escrow_id, depositor, amount),
    );
}

pub fn emit_dispute_raised(env: &Env, escrow_id: u64, raised_by: Address) {
    env.events().publish(
        (symbol_short!("ESC_DIS"),),
        (escrow_id, raised_by),
    );
}

pub fn emit_dispute_resolved(env: &Env, escrow_id: u64, winner: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("ESC_RSV"),),
        (escrow_id, winner, amount),
    );
}

pub fn emit_cancelled(env: &Env, escrow_id: u64, cancelled_by: Address) {
    env.events().publish(
        (symbol_short!("ESC_CAN"),),
        (escrow_id, cancelled_by),
    );
}

pub fn emit_paused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("ESC_PAU"),), admin);
}

pub fn emit_unpaused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("ESC_UNP"),), admin);
}
