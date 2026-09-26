//! # Multi-Admin Support for Contracts (Issue #2134)
//!
//! Provides role-based multi-admin management for snapshot contracts.
//! Supports multiple admin addresses with three permission tiers:
//!
//! | Role        | Capabilities                                           |
//! |-------------|--------------------------------------------------------|
//! | SuperAdmin  | Full control: add/remove admins, snapshots, pause      |
//! | Admin       | Submit snapshots, pause/unpause                        |
//! | Operator    | Submit snapshots only                                  |
//!
//! The first address supplied to `initialize` becomes the sole SuperAdmin.
//! There must always be at least one SuperAdmin in the system.

#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{emit_admin_added, emit_admin_removed, emit_initialized, emit_role_changed, emit_snapshot_submitted};
use soroban_sdk::{contract, contractimpl, contracttype, Address, BytesN, Env, Map, String, Vec};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ~30 days at 5 s/ledger
const LEDGERS_TO_EXTEND: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

// ---------------------------------------------------------------------------
// Role hierarchy
// ---------------------------------------------------------------------------

/// Permission tier for an admin address.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum AdminRole {
    /// Can submit snapshots only
    Operator = 1,
    /// Can submit snapshots and pause/unpause
    Admin = 2,
    /// Full control — add/remove admins, change roles, snapshots, pause
    SuperAdmin = 3,
}

// ---------------------------------------------------------------------------
// Storage Keys
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Map<Address, AdminRole> — all authorized admins and their roles
    AdminRoles,
    /// Vec<Address> — ordered list of all admins (for enumeration)
    AdminList,
    /// Map<epoch, Snapshot>  — submitted snapshots
    Snapshots,
    /// Latest epoch number
    LatestEpoch,
    /// Emergency pause flag
    Paused,
    /// Contract version string
    Version,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub hash: BytesN<32>,
    pub epoch: u64,
    pub timestamp: u64,
    pub submitter: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminEntry {
    pub address: Address,
    pub role: AdminRole,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct MultiAdminContract;

#[contractimpl]
impl MultiAdminContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialize the contract with a SuperAdmin.
    pub fn initialize(env: Env, super_admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::AdminRoles) {
            return Err(Error::AlreadyInitialized);
        }

        let mut roles: Map<Address, AdminRole> = Map::new(&env);
        roles.set(super_admin.clone(), AdminRole::SuperAdmin);

        let mut list: Vec<Address> = Vec::new(&env);
        list.push_back(super_admin.clone());

        env.storage().instance().set(&DataKey::AdminRoles, &roles);
        env.storage().instance().set(&DataKey::AdminList, &list);
        env.storage().instance().set(&DataKey::LatestEpoch, &0u64);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        bump_instance(&env);

        emit_initialized(&env, super_admin);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn get_role(env: &Env, addr: &Address) -> Option<AdminRole> {
        let roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .unwrap_or_else(|| Map::new(env));
        roles.get(addr.clone())
    }

    fn require_role(env: &Env, caller: &Address, min_role: AdminRole) -> Result<(), Error> {
        match Self::get_role(env, caller) {
            None => Err(Error::Unauthorized),
            Some(role) => {
                if (role as u32) >= (min_role as u32) {
                    Ok(())
                } else {
                    Err(Error::InsufficientRole)
                }
            }
        }
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

    fn count_super_admins(env: &Env) -> u32 {
        let roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .unwrap_or_else(|| Map::new(env));
        let list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(env));

        let mut count: u32 = 0;
        for addr in list.iter() {
            if roles.get(addr).map(|r| r == AdminRole::SuperAdmin).unwrap_or(false) {
                count += 1;
            }
        }
        count
    }

    // -----------------------------------------------------------------------
    // Admin management
    // -----------------------------------------------------------------------

    /// Add a new admin with the specified role.
    ///
    /// Only a SuperAdmin may call this.
    pub fn add_admin(
        env: Env,
        caller: Address,
        new_admin: Address,
        role: AdminRole,
    ) -> Result<(), Error> {
        caller.require_auth();
        Self::require_not_paused(&env)?;
        Self::require_role(&env, &caller, AdminRole::SuperAdmin)?;

        let mut roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .unwrap_or_else(|| Map::new(&env));

        if roles.contains_key(new_admin.clone()) {
            return Err(Error::AdminAlreadyExists);
        }

        roles.set(new_admin.clone(), role.clone());

        let mut list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env));
        list.push_back(new_admin.clone());

        env.storage().instance().set(&DataKey::AdminRoles, &roles);
        env.storage().instance().set(&DataKey::AdminList, &list);
        bump_instance(&env);

        emit_admin_added(&env, caller, new_admin, role);
        Ok(())
    }

    /// Remove an admin.
    ///
    /// Only a SuperAdmin may call this. The last SuperAdmin cannot be removed.
    pub fn remove_admin(env: Env, caller: Address, target: Address) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, AdminRole::SuperAdmin)?;

        let mut roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .ok_or(Error::AdminNotFound)?;

        if !roles.contains_key(target.clone()) {
            return Err(Error::AdminNotFound);
        }

        // Guard: prevent removing the last SuperAdmin
        if roles.get(target.clone()) == Some(AdminRole::SuperAdmin)
            && Self::count_super_admins(&env) <= 1
        {
            return Err(Error::CannotRemoveLastSuperAdmin);
        }

        roles.remove(target.clone());

        // Remove from list
        let list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut new_list: Vec<Address> = Vec::new(&env);
        for addr in list.iter() {
            if addr != target {
                new_list.push_back(addr);
            }
        }

        env.storage().instance().set(&DataKey::AdminRoles, &roles);
        env.storage().instance().set(&DataKey::AdminList, &new_list);
        bump_instance(&env);

        emit_admin_removed(&env, caller, target);
        Ok(())
    }

    /// Change the role of an existing admin.
    ///
    /// Only a SuperAdmin may call this. Cannot demote the last SuperAdmin.
    pub fn change_role(
        env: Env,
        caller: Address,
        target: Address,
        new_role: AdminRole,
    ) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, AdminRole::SuperAdmin)?;

        let mut roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .ok_or(Error::AdminNotFound)?;

        if !roles.contains_key(target.clone()) {
            return Err(Error::AdminNotFound);
        }

        // Guard: prevent demoting the last SuperAdmin
        if roles.get(target.clone()) == Some(AdminRole::SuperAdmin)
            && new_role != AdminRole::SuperAdmin
            && Self::count_super_admins(&env) <= 1
        {
            return Err(Error::CannotRemoveLastSuperAdmin);
        }

        roles.set(target.clone(), new_role.clone());
        env.storage().instance().set(&DataKey::AdminRoles, &roles);
        bump_instance(&env);

        emit_role_changed(&env, caller, target, new_role);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Snapshot submission
    // -----------------------------------------------------------------------

    /// Submit a snapshot hash.
    ///
    /// Any Admin or Operator may call this.
    pub fn submit_snapshot(
        env: Env,
        caller: Address,
        epoch: u64,
        hash: BytesN<32>,
    ) -> Result<u64, Error> {
        caller.require_auth();
        Self::require_not_paused(&env)?;
        Self::require_role(&env, &caller, AdminRole::Operator)?;

        if epoch == 0 {
            return Err(Error::InvalidEpoch);
        }

        let zero_hash = BytesN::from_array(&env, &[0u8; 32]);
        if hash == zero_hash {
            return Err(Error::InvalidHash);
        }

        let mut snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .unwrap_or_else(|| Map::new(&env));

        if snapshots.contains_key(epoch) {
            return Err(Error::DuplicateEpoch);
        }

        let latest: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0);
        if epoch <= latest {
            return Err(Error::EpochMonotonicityViolated);
        }

        let timestamp = env.ledger().timestamp();
        let snapshot = Snapshot {
            hash: hash.clone(),
            epoch,
            timestamp,
            submitter: caller.clone(),
        };

        snapshots.set(epoch, snapshot);
        env.storage()
            .persistent()
            .set(&DataKey::Snapshots, &snapshots);
        env.storage().persistent().extend_ttl(
            &DataKey::Snapshots,
            LEDGERS_TO_EXTEND,
            LEDGERS_TO_EXTEND,
        );
        env.storage().instance().set(&DataKey::LatestEpoch, &epoch);
        bump_instance(&env);

        emit_snapshot_submitted(&env, caller, epoch, hash, timestamp);
        Ok(timestamp)
    }

    // -----------------------------------------------------------------------
    // Pause / Unpause
    // -----------------------------------------------------------------------

    /// Pause the contract. Requires Admin or SuperAdmin role.
    pub fn pause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, AdminRole::Admin)?;
        env.storage().instance().set(&DataKey::Paused, &true);
        bump_instance(&env);
        env.events()
            .publish((soroban_sdk::symbol_short!("paused"),), caller);
        Ok(())
    }

    /// Unpause the contract. Requires Admin or SuperAdmin role.
    pub fn unpause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        Self::require_role(&env, &caller, AdminRole::Admin)?;
        env.storage().instance().set(&DataKey::Paused, &false);
        bump_instance(&env);
        env.events()
            .publish((soroban_sdk::symbol_short!("unpaused"),), caller);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /// Get the role of an address, or None if not an admin.
    pub fn get_role(env: Env, addr: Address) -> Option<AdminRole> {
        let roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .unwrap_or_else(|| Map::new(&env));
        roles.get(addr)
    }

    /// Check whether an address is an admin.
    pub fn is_admin(env: Env, addr: Address) -> bool {
        let roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .unwrap_or_else(|| Map::new(&env));
        roles.contains_key(addr)
    }

    /// Get the snapshot for a given epoch.
    pub fn get_snapshot(env: Env, epoch: u64) -> Result<Snapshot, Error> {
        let snapshots: Map<u64, Snapshot> = env
            .storage()
            .persistent()
            .get(&DataKey::Snapshots)
            .ok_or(Error::SnapshotNotFound)?;
        snapshots.get(epoch).ok_or(Error::SnapshotNotFound)
    }

    /// Get the latest epoch.
    pub fn get_latest_epoch(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LatestEpoch)
            .unwrap_or(0)
    }

    /// Check if contract is paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// List all admin addresses.
    pub fn list_admins(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get the list of admin entries (address + role).
    pub fn list_admin_entries(env: Env) -> Vec<AdminEntry> {
        let roles: Map<Address, AdminRole> = env
            .storage()
            .instance()
            .get(&DataKey::AdminRoles)
            .unwrap_or_else(|| Map::new(&env));
        let list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut entries: Vec<AdminEntry> = Vec::new(&env);
        for addr in list.iter() {
            if let Some(role) = roles.get(addr.clone()) {
                entries.push_back(AdminEntry {
                    address: addr,
                    role,
                });
            }
        }
        entries
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
