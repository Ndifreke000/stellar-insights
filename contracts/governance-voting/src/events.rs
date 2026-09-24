use soroban_sdk::{symbol_short, Address, Env, String};

pub fn emit_initialized(env: &Env, admin: Address, quorum: u64, voting_period: u64) {
    env.events().publish(
        (symbol_short!("GV_INIT"),),
        (admin, quorum, voting_period),
    );
}

pub fn emit_voter_registered(env: &Env, voter: Address, weight: u64) {
    env.events().publish(
        (symbol_short!("GV_REG"),),
        (voter, weight),
    );
}

pub fn emit_proposal_created(env: &Env, proposal_id: u64, proposer: Address, title: String, voting_ends_at: u64) {
    env.events().publish(
        (symbol_short!("GV_PROP"),),
        (proposal_id, proposer, title, voting_ends_at),
    );
}

pub fn emit_vote_cast(env: &Env, proposal_id: u64, voter: Address, choice: u32, weight: u64) {
    env.events().publish(
        (symbol_short!("GV_VOTE"),),
        (proposal_id, voter, choice, weight),
    );
}

pub fn emit_proposal_finalized(env: &Env, proposal_id: u64, status: u32, votes_for: u64, votes_against: u64) {
    env.events().publish(
        (symbol_short!("GV_FIN"),),
        (proposal_id, status, votes_for, votes_against),
    );
}

pub fn emit_admin_changed(env: &Env, old_admin: Address, new_admin: Address) {
    env.events().publish(
        (symbol_short!("GV_ADM"),),
        (old_admin, new_admin),
    );
}
