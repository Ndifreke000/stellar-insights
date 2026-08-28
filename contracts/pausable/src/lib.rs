//! # Contract Pause/Unpause Functionality (Issue #2135)
//!
//! A reusable emergency-pause module for Soroban contracts.
//!
//! Provides:
//! - `pause(caller)` / `unpause(caller)` — admin-only state toggle
//! - `is_paused()` — public read
//! - `get_pause_history()` — on-chain audit trail of pause events
//! - Role-based pause authority: any listed `PauseGuardian` OR the primary
//!   admin may pause; only the primary admin may unpause.
//!
//! ## Design rationale
//! Separating pause authority from unpause authority follows the principle of
//! least privilege: a broader set of trusted addresses can trigger an emergency
//! stop, but only the highest-trust admin can resume operations.

#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{emit_guardian_added, emit_guardian_removed, emit_initialized, emit_paused, emit_unpaused};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
/// Maximum number of pause history records kept on-chain
const MAX_PAUSE_HISTORY: u32 = 50;

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
    /// Primary admin address (only admin may unpause)
    Admin,
    /// Current pause state
    Paused,
    /// Vec<Address> — pause guardians who may trigger an emergency pause
    PauseGuardians,
    /// Vec<PauseRecord> — audit log of pause/unpause events
    PauseHistory,
    /// Contract version
    Version,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// A single entry in the pause audit trail
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseRecord {
    /// Whether this is a pause (true) or unpause (false) event
    pub paused: bool,
    /// Address that triggered the change
    pub triggered_by: Address,
    /// Reason string (optional — empty string if not provided)
    pub reason: String,
    /// Ledger timestamp
    pub timestamp: u64,
    /// Ledger sequence number
    pub ledger: u32,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct PausableContract;

#[contractimpl]
impl PausableContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialize the contract.
    ///
    /// # Arguments
    /// * `admin` - Primary admin address; the only address that can unpause.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::PauseGuardians, &Vec::<Address>::new(&env));
        env.storage()
            .instance()
            .set(&DataKey::PauseHistory, &Vec::<PauseRecord>::new(&env));
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        bump_instance(&env);

        emit_initialized(&env, admin);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn get_admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }

    fn is_guardian(env: &Env, addr: &Address) -> bool {
        let guardians: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::PauseGuardians)
            .unwrap_or_else(|| Vec::new(env));
        for g in guardians.iter() {
            if &g == addr {
                return true;
            }
        }
        false
    }

    fn append_history(env: &Env, record: PauseRecord) {
        let mut history: Vec<PauseRecord> = env
            .storage()
            .instance()
            .get(&DataKey::PauseHistory)
            .unwrap_or_else(|| Vec::new(env));

        // Trim to keep only the latest MAX_PAUSE_HISTORY entries
        while history.len() >= MAX_PAUSE_HISTORY {
            history.remove(0);
        }
        history.push_back(record);
        env.storage()
            .instance()
            .set(&DataKey::PauseHistory, &history);
    }

    // -----------------------------------------------------------------------
    // Pause / Unpause
    // -----------------------------------------------------------------------

    /// Emergency-pause the contract.
    ///
    /// Can be called by the admin OR any registered pause guardian.
    ///
    /// # Arguments
    /// * `caller` - Address triggering the pause
    /// * `reason` - Optional human-readable reason (pass empty string for none)
    pub fn pause(env: Env, caller: Address, reason: String) -> Result<(), Error> {
        caller.require_auth();

        let admin = Self::get_admin(&env)?;
        let is_admin = caller == admin;
        let is_guardian = Self::is_guardian(&env, &caller);

        if !is_admin && !is_guardian {
            return Err(Error::Unauthorized);
        }

        let already_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if already_paused {
            return Err(Error::AlreadyPaused);
        }

        env.storage().instance().set(&DataKey::Paused, &true);

        let record = PauseRecord {
            paused: true,
            triggered_by: caller.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
            ledger: env.ledger().sequence(),
        };
        Self::append_history(&env, record);
        bump_instance(&env);

        emit_paused(&env, caller, reason);
        Ok(())
    }

    /// Resume normal operations.
    ///
    /// Only the primary admin may unpause.
    ///
    /// # Arguments
    /// * `caller` - Must be the primary admin
    /// * `reason` - Optional human-readable reason
    pub fn unpause(env: Env, caller: Address, reason: String) -> Result<(), Error> {
        caller.require_auth();

        let admin = Self::get_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }

        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if !is_paused {
            return Err(Error::NotPaused);
        }

        env.storage().instance().set(&DataKey::Paused, &false);

        let record = PauseRecord {
            paused: false,
            triggered_by: caller.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
            ledger: env.ledger().sequence(),
        };
        Self::append_history(&env, record);
        bump_instance(&env);

        emit_unpaused(&env, caller, reason);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Guardian management
    // -----------------------------------------------------------------------

    /// Add a pause guardian (admin only).
    ///
    /// Guardians can trigger emergency pauses but cannot unpause.
    pub fn add_guardian(env: Env, caller: Address, guardian: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin = Self::get_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }

        let mut guardians: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::PauseGuardians)
            .unwrap_or_else(|| Vec::new(&env));

        // Prevent duplicates
        for g in guardians.iter() {
            if g == guardian {
                return Err(Error::GuardianAlreadyExists);
            }
        }

        guardians.push_back(guardian.clone());
        env.storage()
            .instance()
            .set(&DataKey::PauseGuardians, &guardians);
        bump_instance(&env);

        emit_guardian_added(&env, caller, guardian);
        Ok(())
    }

    /// Remove a pause guardian (admin only).
    pub fn remove_guardian(env: Env, caller: Address, guardian: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin = Self::get_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }

        let guardians: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::PauseGuardians)
            .unwrap_or_else(|| Vec::new(&env));

        let mut new_guardians: Vec<Address> = Vec::new(&env);
        let mut found = false;
        for g in guardians.iter() {
            if g == guardian {
                found = true;
            } else {
                new_guardians.push_back(g);
            }
        }

        if !found {
            return Err(Error::GuardianNotFound);
        }

        env.storage()
            .instance()
            .set(&DataKey::PauseGuardians, &new_guardians);
        bump_instance(&env);

        emit_guardian_removed(&env, caller, guardian);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Admin transfer
    // -----------------------------------------------------------------------

    /// Transfer the admin role (both parties must sign).
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin = Self::get_admin(&env)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        new_admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        bump_instance(&env);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /// Check if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Get the primary admin address.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }

    /// List all pause guardians.
    pub fn list_guardians(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::PauseGuardians)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get the on-chain pause audit history (most recent last).
    pub fn get_pause_history(env: Env) -> Vec<PauseRecord> {
        env.storage()
            .instance()
            .get(&DataKey::PauseHistory)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Check if an address is a pause guardian.
    pub fn is_guardian(env: Env, addr: Address) -> bool {
        Self::is_guardian(&env, &addr)
    }

    /// Get contract version.
    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }
}

mod test;
