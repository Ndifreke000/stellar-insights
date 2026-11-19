#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{
    emit_cancelled, emit_dispute_raised, emit_dispute_resolved, emit_escrow_created,
    emit_escrow_funded, emit_funds_released, emit_initialized, emit_paused, emit_refunded,
    emit_unpaused,
};
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, String,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
const ESCROW_TTL_THRESHOLD: u32 = 100_000;
const ESCROW_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

fn require_not_locked(env: &Env) -> Result<(), Error> {
    if env
        .storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Locked)
        .unwrap_or(false)
    {
        return Err(Error::Reentrancy);
    }
    Ok(())
}

fn set_locked(env: &Env) {
    env.storage().instance().set(&DataKey::Locked, &true);
}

fn set_unlocked(env: &Env) {
    env.storage().instance().set(&DataKey::Locked, &false);
}

// ============================================================================
// Data Types
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum EscrowState {
    /// Escrow created, awaiting deposit
    Created = 0,
    /// Funds deposited, awaiting release condition
    Funded = 1,
    /// Funds released to beneficiary
    Released = 2,
    /// Funds returned to depositor
    Refunded = 3,
    /// Dispute raised — awaiting arbitration by admin
    Disputed = 4,
    /// Cancelled before funding
    Cancelled = 5,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    /// Unique ID
    pub id: u64,
    /// Party depositing funds
    pub depositor: Address,
    /// Party receiving funds upon release
    pub beneficiary: Address,
    /// Token contract address
    pub token: Address,
    /// Amount locked in escrow
    pub amount: i128,
    /// Current state
    pub state: EscrowState,
    /// Unix timestamp after which depositor may claim a refund
    pub deadline: u64,
    /// Ledger timestamp when created
    pub created_at: u64,
}

// ============================================================================
// Storage Keys
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    EscrowCount,
    Paused,
    Version,
    Escrow(u64),
    Locked,
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct EscrowServiceContract;

#[contractimpl]
impl EscrowServiceContract {
    /// Initialize the escrow service with an admin address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::EscrowCount, &0u64);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);

        emit_initialized(&env, admin);
        Ok(())
    }

    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }

    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false)
    }

    pub fn pause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Paused, &true);
        bump_instance(&env);
        emit_paused(&env, caller);
        Ok(())
    }

    pub fn unpause(env: Env, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Paused, &false);
        bump_instance(&env);
        emit_unpaused(&env, caller);
        Ok(())
    }

    /// Create a new escrow agreement.
    ///
    /// The depositor initiates the escrow by specifying the beneficiary, token,
    /// amount, and a deadline after which a refund can be claimed if funds
    /// have not been released.
    ///
    /// Returns the new escrow ID.
    pub fn create_escrow(
        env: Env,
        depositor: Address,
        beneficiary: Address,
        token: Address,
        amount: i128,
        deadline: u64,
    ) -> Result<u64, Error> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(Error::ContractPaused);
        }

        depositor.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if deadline <= now {
            return Err(Error::DeadlineExpired);
        }

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::EscrowCount)
            .unwrap_or(0);
        count = count.checked_add(1).ok_or(Error::EscrowIdOverflow)?;

        let escrow = Escrow {
            id: count,
            depositor: depositor.clone(),
            beneficiary: beneficiary.clone(),
            token,
            amount,
            state: EscrowState::Created,
            deadline,
            created_at: now,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Escrow(count), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(count),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        env.storage()
            .instance()
            .set(&DataKey::EscrowCount, &count);
        bump_instance(&env);

        emit_escrow_created(&env, count, depositor, beneficiary, amount);

        Ok(count)
    }

    /// Deposit funds into the escrow.
    ///
    /// The depositor transfers the agreed amount to this contract.
    /// Can only be called when the escrow is in the `Created` state.
    pub fn fund_escrow(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        require_not_locked(&env)?;

        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(Error::ContractPaused);
        }

        caller.require_auth();

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.state != EscrowState::Created {
            return Err(Error::InvalidState);
        }

        if caller != escrow.depositor {
            return Err(Error::Unauthorized);
        }

        let now = env.ledger().timestamp();
        if now >= escrow.deadline {
            return Err(Error::DeadlineExpired);
        }

        // Save values before mutating escrow
        let depositor = escrow.depositor.clone();
        let amount = escrow.amount;
        let token = escrow.token.clone();

        // Effect: update state before external call (CEI)
        escrow.state = EscrowState::Funded;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        // Interaction: external token call under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &token).transfer(&caller, &env.current_contract_address(), &amount);
        set_unlocked(&env);

        emit_escrow_funded(&env, escrow_id, depositor, amount);

        Ok(())
    }

    /// Release escrowed funds to the beneficiary.
    ///
    /// Only the depositor can release funds. The escrow must be in the `Funded`
    /// state (not disputed).
    pub fn release_funds(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        require_not_locked(&env)?;

        caller.require_auth();

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.state != EscrowState::Funded {
            return Err(Error::InvalidState);
        }

        if caller != escrow.depositor {
            return Err(Error::Unauthorized);
        }

        if escrow.amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token = escrow.token.clone();
        let beneficiary = escrow.beneficiary.clone();
        let amount = escrow.amount;

        // Effect: update state before external call (CEI)
        escrow.state = EscrowState::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        // Interaction: external token call under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &beneficiary,
            &amount,
        );
        set_unlocked(&env);

        emit_funds_released(&env, escrow_id, beneficiary, amount);

        Ok(())
    }

    /// Claim a refund after the deadline has passed without a release.
    ///
    /// Only the depositor can call this. The escrow must be `Funded` and the
    /// deadline must have passed.
    pub fn refund(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        require_not_locked(&env)?;

        caller.require_auth();

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.state != EscrowState::Funded {
            return Err(Error::InvalidState);
        }

        if caller != escrow.depositor {
            return Err(Error::Unauthorized);
        }

        if escrow.amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if now < escrow.deadline {
            return Err(Error::DeadlineNotExpired);
        }

        let token = escrow.token.clone();
        let depositor = escrow.depositor.clone();
        let amount = escrow.amount;

        // Effect: update state before external call (CEI)
        escrow.state = EscrowState::Refunded;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        // Interaction: external token call under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &depositor,
            &amount,
        );
        set_unlocked(&env);

        emit_refunded(&env, escrow_id, depositor, amount);

        Ok(())
    }

    /// Raise a dispute on a funded escrow.
    ///
    /// Either the depositor or beneficiary may raise a dispute.
    /// Moves the escrow into `Disputed` state — only the admin can resolve it.
    pub fn raise_dispute(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.state != EscrowState::Funded {
            return Err(Error::InvalidState);
        }

        if caller != escrow.depositor && caller != escrow.beneficiary {
            return Err(Error::Unauthorized);
        }

        escrow.state = EscrowState::Disputed;
        let raised_by = caller.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        emit_dispute_raised(&env, escrow_id, raised_by);

        Ok(())
    }

    /// Resolve a disputed escrow.
    ///
    /// Admin-only. `release_to_beneficiary` determines whether funds go to the
    /// beneficiary (`true`) or back to the depositor (`false`).
    pub fn resolve_dispute(
        env: Env,
        caller: Address,
        escrow_id: u64,
        release_to_beneficiary: bool,
    ) -> Result<(), Error> {
        require_not_locked(&env)?;

        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        if caller != admin {
            return Err(Error::Unauthorized);
        }

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.state != EscrowState::Disputed {
            return Err(Error::NotDisputed);
        }

        if escrow.amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let token = escrow.token.clone();
        let (recipient, new_state) = if release_to_beneficiary {
            (escrow.beneficiary.clone(), EscrowState::Released)
        } else {
            (escrow.depositor.clone(), EscrowState::Refunded)
        };
        let winner = recipient.clone();
        let amount = escrow.amount;

        // Effect: update state before external call (CEI)
        escrow.state = new_state;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        // Interaction: external token call under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &recipient,
            &amount,
        );
        set_unlocked(&env);

        emit_dispute_resolved(&env, escrow_id, winner, amount);

        Ok(())
    }

    /// Cancel an escrow that has not yet been funded.
    ///
    /// Only the depositor or admin can cancel a `Created` escrow.
    pub fn cancel_escrow(env: Env, caller: Address, escrow_id: u64) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        let mut escrow: Escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;

        if escrow.state != EscrowState::Created {
            return Err(Error::InvalidState);
        }

        if caller != escrow.depositor && caller != admin {
            return Err(Error::Unauthorized);
        }

        escrow.state = EscrowState::Cancelled;
        let cancelled_by = caller.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(escrow_id), &escrow);
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );

        emit_cancelled(&env, escrow_id, cancelled_by);

        Ok(())
    }

    // ========================================================================
    // Query Functions
    // ========================================================================

    pub fn get_escrow(env: Env, escrow_id: u64) -> Result<Escrow, Error> {
        let escrow = env
            .storage()
            .persistent()
            .get(&DataKey::Escrow(escrow_id))
            .ok_or(Error::EscrowNotFound)?;
        env.storage().persistent().extend_ttl(
            &DataKey::Escrow(escrow_id),
            ESCROW_TTL_THRESHOLD,
            ESCROW_TTL_EXTEND,
        );
        Ok(escrow)
    }

    pub fn get_escrow_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::EscrowCount)
            .unwrap_or(0)
    }

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
    }
}

mod test;
