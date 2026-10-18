use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_initialized(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("TSW_INI"),), admin);
}

pub fn emit_offer_created(
    env: &Env,
    offer_id: u64,
    maker: Address,
    offer_amount: i128,
    want_amount: i128,
) {
    env.events().publish(
        (symbol_short!("TSW_CRT"),),
        (offer_id, maker, offer_amount, want_amount),
    );
}

pub fn emit_offer_accepted(env: &Env, offer_id: u64, taker: Address) {
    env.events()
        .publish((symbol_short!("TSW_ACC"),), (offer_id, taker));
}

pub fn emit_offer_cancelled(env: &Env, offer_id: u64, cancelled_by: Address) {
    env.events()
        .publish((symbol_short!("TSW_CAN"),), (offer_id, cancelled_by));
}

pub fn emit_paused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("TSW_PAU"),), admin);
}

pub fn emit_unpaused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("TSW_UNP"),), admin);
}
