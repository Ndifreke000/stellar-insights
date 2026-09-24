use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("TLT_INI"),), admin);
}

pub fn emit_scheduled(
    env: &Env,
    tx_id: u64,
    sender: Address,
    recipient: Address,
    amount: i128,
    unlock_ledger: u32,
) {
    env.events().publish(
        (symbol_short!("TLT_SCH"),),
        (tx_id, sender, recipient, amount, unlock_ledger),
    );
}

pub fn emit_executed(env: &Env, tx_id: u64, recipient: Address, amount: i128) {
    env.events()
        .publish((symbol_short!("TLT_EXE"),), (tx_id, recipient, amount));
}

pub fn emit_cancelled(env: &Env, tx_id: u64, cancelled_by: Address) {
    env.events()
        .publish((symbol_short!("TLT_CAN"),), (tx_id, cancelled_by));
}

pub fn emit_paused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("TLT_PAU"),), admin);
}

pub fn emit_unpaused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("TLT_UNP"),), admin);
}
