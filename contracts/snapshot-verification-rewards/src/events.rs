use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

// ---------------------------------------------------------------------------
// Event structures
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializedEvent {
    pub admin: Address,
    pub reward_per_verification: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotRegisteredEvent {
    pub registrar: Address,
    pub epoch: u64,
    pub expected_hash: BytesN<32>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationEvent {
    pub verifier: Address,
    pub epoch: u64,
    pub matched: bool,
    pub points_awarded: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardClaimedEvent {
    pub verifier: Address,
    pub points: u64,
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Event helper functions
// ---------------------------------------------------------------------------

pub fn emit_initialized(env: &Env, admin: Address, reward_per_verification: u64) {
    let event = InitializedEvent {
        admin,
        reward_per_verification,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("svr_init"),), event);
}

pub fn emit_snapshot_registered(
    env: &Env,
    registrar: Address,
    epoch: u64,
    expected_hash: BytesN<32>,
) {
    let event = SnapshotRegisteredEvent {
        registrar,
        epoch,
        expected_hash,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("snap_reg"),), event);
}

pub fn emit_verified(env: &Env, verifier: Address, epoch: u64, matched: bool, points_awarded: u64) {
    let event = VerificationEvent {
        verifier,
        epoch,
        matched,
        points_awarded,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("verified"),), event);
}

pub fn emit_reward_claimed(env: &Env, verifier: Address, points: u64) {
    let event = RewardClaimedEvent {
        verifier,
        points,
        timestamp: env.ledger().timestamp(),
    };
    env.events()
        .publish((symbol_short!("rwd_claim"),), event);
}
