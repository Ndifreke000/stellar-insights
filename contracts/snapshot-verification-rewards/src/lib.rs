//! # Snapshot Verification Rewards Contract (Issue #2136)
//!
//! Reward mechanism for users who verify that snapshot hashes stored on-chain
//! match the backend analytics data.
//!
//! ## Flow
//! 1. Admin registers a snapshot epoch + expected hash via `register_snapshot`.
//! 2. A verifier calls `verify_snapshot` with the epoch and hash they computed
//!    off-chain from the backend data.
//! 3. If the submitted hash matches the on-chain record, the verifier earns
//!    reward points. Each (verifier, epoch) pair can only be submitted once.
//! 4. Reward points accumulate and can be claimed via `claim_reward`.

#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{emit_initialized, emit_reward_claimed, emit_snapshot_registered, emit_verified};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
/// Default reward points awarded per correct verification
const DEFAULT_REWARD_POINTS: u64 = 100;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

// ---------------------------------------------------------------------------
// Storage Keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Administrator address
    Admin,
    /// Contract pause state
    Paused,
    /// Map<epoch, RegisteredSnapshot> — snapshots available for verification
    RegisteredSnapshots,
    /// Map<epoch, Map<Address, bool>> — per-(epoch,verifier) dedup tracker
    EpochVerifiers,
    /// Map<Address, u64> — accumulated reward points per address
    RewardPoints,
    /// Points awarded per successful verification (admin-configurable)
    RewardPerVerification,
    /// Contract version string
    Version,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A registered snapshot epoch with its expected hash
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredSnapshot {
    /// Expected SHA-256 hash of the backend analytics data for this epoch
    pub expected_hash: BytesN<32>,
    /// Epoch identifier
    pub epoch: u64,
    /// Ledger timestamp when registered
    pub registered_at: u64,
    /// Whether this epoch is still open for verification
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct SnapshotVerificationRewardsContract;

#[contractimpl]
impl SnapshotVerificationRewardsContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialize the contract.
    ///
    /// # Arguments
    /// * `admin`             - Admin address
    /// * `reward_per_verify` - Points per correct verification; `0` uses the
    ///                         built-in default of 100.
    pub fn initialize(env: Env, admin: Address, reward_per_verify: u64) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        let points = if reward_per_verify == 0 {
            DEFAULT_REWARD_POINTS
        } else {
            reward_per_verify
        };

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::RewardPerVerification, &points);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        bump_instance(&env);

        emit_initialized(&env, admin, points);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn require_admin(env: &Env, caller: &Address) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != &admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    fn require_not_paused(env: &Env) -> Result<(), Error> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            Err(Error::ContractPaused)
        } else {
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Admin: Snapshot registration
    // -----------------------------------------------------------------------

    /// Register a snapshot epoch available for verification.
    ///
    /// Only the admin may call this. `expected_hash` is the SHA-256 hash of the
    /// backend analytics payload for that epoch.
    pub fn register_snapshot(
        env: Env,
        caller: Address,
        epoch: u64,
        expected_hash: BytesN<32>,
    ) -> Result<(), Error> {
        caller.require_auth();
        Self::require_not_paused(&env)?;
        Self::require_admin(&env, &caller)?;

        if epoch == 0 {
            return Err(Error::InvalidEpoch);
        }

        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        if expected_hash == zero_hash {
            return Err(Error::InvalidHash);
        }

        let mut snapshots: Map<u64, RegisteredSnapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::RegisteredSnapshots)
            .unwrap_or_else(|| Map::new(&env));

        if snapshots.contains_key(epoch) {
            return Err(Error::EpochAlreadyRegistered);
        }

        let record = RegisteredSnapshot {
            expected_hash: expected_hash.clone(),
            epoch,
            registered_at: env.ledger().timestamp(),
            active: true,
        };

        snapshots.set(epoch, record);
        env.storage()
            .persistent()
            .set(&DataKey::RegisteredSnapshots, &snapshots);
        env.storage().persistent().extend_ttl(
            &DataKey::RegisteredSnapshots,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        bump_instance(&env);

        emit_snapshot_registered(&env, caller, epoch, expected_hash);
        Ok(())
    }

    /// Deactivate an epoch so it can no longer be verified.
    pub fn deactivate_epoch(env: Env, caller: Address, epoch: u64) -> Result<(), Error> {
        caller.require_auth();
        Self::require_admin(&env, &caller)?;

        let mut snapshots: Map<u64, RegisteredSnapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::RegisteredSnapshots)
            .ok_or(Error::EpochNotFound)?;

        let mut record = snapshots.get(epoch).ok_or(Error::EpochNotFound)?;
        record.active = false;
        snapshots.set(epoch, record);

        env.storage()
            .persistent()
            .set(&DataKey::RegisteredSnapshots, &snapshots);
        env.storage().persistent().extend_ttl(
            &DataKey::RegisteredSnapshots,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        bump_instance(&env);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Verifier: Verify a snapshot
    // -----------------------------------------------------------------------

    /// Submit a verification attempt for an epoch.
    ///
    /// The verifier computes the SHA-256 hash of the backend analytics payload
    /// for `epoch` and passes it as `submitted_hash`. If it matches the
    /// on-chain registered hash, the verifier earns reward points.
    ///
    /// Each (verifier, epoch) pair can only be submitted once.
    ///
    /// # Returns
    /// * Points awarded — positive on hash match, `0` on mismatch.
    pub fn verify_snapshot(
        env: Env,
        verifier: Address,
        epoch: u64,
        submitted_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        verifier.require_auth();
        Self::require_not_paused(&env)?;

        if epoch == 0 {
            return Err(Error::InvalidEpoch);
        }

        // Load registered snapshot
        let snapshots: Map<u64, RegisteredSnapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::RegisteredSnapshots)
            .ok_or(Error::EpochNotFound)?;

        let record = snapshots.get(epoch).ok_or(Error::EpochNotFound)?;

        if !record.active {
            return Err(Error::EpochInactive);
        }

        // Prevent double-verification per (epoch, verifier) using a
        // nested Map<epoch -> Map<Address, bool>> in persistent storage.
        let mut outer_map: Map<u64, Map<Address, bool>> = env
            .storage()
            .persistent()
            .get(&DataKey::EpochVerifiers)
            .unwrap_or_else(|| Map::new(&env));

        let mut inner_map: Map<Address, bool> = outer_map
            .get(epoch)
            .unwrap_or_else(|| Map::new(&env));

        if inner_map.get(verifier.clone()).unwrap_or(false) {
            return Err(Error::AlreadyVerified);
        }

        // Compare submitted hash against the registered expected hash
        let matched = submitted_hash == record.expected_hash;
        let points_awarded: u64 = if matched {
            env.storage()
                .instance()
                .get(&DataKey::RewardPerVerification)
                .unwrap_or(DEFAULT_REWARD_POINTS)
        } else {
            0
        };

        // Mark this (epoch, verifier) as seen
        inner_map.set(verifier.clone(), true);
        outer_map.set(epoch, inner_map);
        env.storage()
            .persistent()
            .set(&DataKey::EpochVerifiers, &outer_map);
        env.storage().persistent().extend_ttl(
            &DataKey::EpochVerifiers,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );

        // Accumulate reward points on a successful match
        if points_awarded > 0 {
            let mut reward_map: Map<Address, u64> = env
                .storage()
                .persistent()
                .get(&DataKey::RewardPoints)
                .unwrap_or_else(|| Map::new(&env));

            let current = reward_map.get(verifier.clone()).unwrap_or(0);
            reward_map.set(verifier.clone(), current + points_awarded);

            env.storage()
                .persistent()
                .set(&DataKey::RewardPoints, &reward_map);
            env.storage().persistent().extend_ttl(
                &DataKey::RewardPoints,
                LEDGERS_TO_EXTEND,
                LEDGERS_TO_EXTEND,
            );
        }

        bump_instance(&env);
        emit_verified(&env, verifier, epoch, matched, points_awarded);

        Ok(points_awarded)
    }

    // -----------------------------------------------------------------------
    // Reward queries & claims
    // -----------------------------------------------------------------------

    /// Get the accumulated reward points for an address.
    pub fn get_reward_points(env: Env, verifier: Address) -> u64 {
        let reward_map: Map<Address, u64> = env
            .storage()
            .persistent()
            .get(&DataKey::RewardPoints)
            .unwrap_or_else(|| Map::new(&env));
        reward_map.get(verifier).unwrap_or(0)
    }

    /// Claim (reset) accumulated reward points.
    ///
    /// Emits a `RewardClaimed` event so off-chain systems can settle payouts.
    /// Points are zeroed on-chain; actual token payout is handled off-chain
    /// or by a separate token contract.
    ///
    /// # Returns
    /// * Total points claimed
    pub fn claim_reward(env: Env, verifier: Address) -> Result<u64, Error> {
        verifier.require_auth();
        Self::require_not_paused(&env)?;

        let mut reward_map: Map<Address, u64> = env
            .storage()
            .persistent()
            .get(&DataKey::RewardPoints)
            .unwrap_or_else(|| Map::new(&env));

        let points = reward_map.get(verifier.clone()).unwrap_or(0);
        if points == 0 {
            return Err(Error::NoRewardsToClaim);
        }

        reward_map.set(verifier.clone(), 0u64);
        env.storage()
            .persistent()
            .set(&DataKey::RewardPoints, &reward_map);
        env.storage().persistent().extend_ttl(
            &DataKey::RewardPoints,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        bump_instance(&env);

        emit_reward_claimed(&env, verifier, points);
        Ok(points)
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /// Get the registered snapshot record for an epoch.
    pub fn get_registered_snapshot(env: Env, epoch: u64) -> Result<RegisteredSnapshot, Error> {
        let snapshots: Map<u64, RegisteredSnapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::RegisteredSnapshots)
            .ok_or(Error::EpochNotFound)?;
        snapshots.get(epoch).ok_or(Error::EpochNotFound)
    }

    /// Check if a verifier has already submitted a verification for an epoch.
    pub fn has_verified(env: Env, verifier: Address, epoch: u64) -> bool {
        let outer_map: Map<u64, Map<Address, bool>> = env
            .storage()
            .persistent()
            .get(&DataKey::EpochVerifiers)
            .unwrap_or_else(|| Map::new(&env));
        let inner_map: Map<Address, bool> = outer_map
            .get(epoch)
            .unwrap_or_else(|| Map::new(&env));
        inner_map.get(verifier).unwrap_or(false)
    }

    /// Get current reward points setting.
    pub fn get_reward_per_verification(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::RewardPerVerification)
            .unwrap_or(DEFAULT_REWARD_POINTS)
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }

    /// Check whether the contract is paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Get contract version.
    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }

    // -----------------------------------------------------------------------
    // Admin: Pause / Unpause
    // -----------------------------------------------------------------------

    /// Emergency-pause the contract (admin only).
    pub fn pause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&DataKey::Paused, &true);
        bump_instance(&env);
        env.events()
            .publish((soroban_sdk::symbol_short!("paused"),), caller);
        Ok(())
    }

    /// Unpause the contract (admin only).
    pub fn unpause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&DataKey::Paused, &false);
        bump_instance(&env);
        env.events()
            .publish((soroban_sdk::symbol_short!("unpaused"),), caller);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Admin: Configure rewards
    // -----------------------------------------------------------------------

    /// Update the points awarded per successful verification (admin only).
    pub fn set_reward_per_verification(
        env: Env,
        caller: Address,
        new_points: u64,
    ) -> Result<(), Error> {
        caller.require_auth();
        Self::require_admin(&env, &caller)?;

        if new_points == 0 {
            return Err(Error::InvalidRewardAmount);
        }

        env.storage()
            .instance()
            .set(&DataKey::RewardPerVerification, &new_points);
        bump_instance(&env);
        Ok(())
    }

    /// Transfer admin rights to a new address (both must sign).
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();
        Self::require_admin(&env, &caller)?;
        new_admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);
        Ok(())
    }
}

mod test;
