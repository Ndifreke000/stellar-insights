#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{emit_cancelled, emit_executed, emit_initialized, emit_paused, emit_scheduled, emit_unpaused};
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
const TX_TTL_THRESHOLD: u32 = 100_000;
const TX_TTL_EXTEND: u32 = 518_400;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);
}

// ============================================================================
// Data Types
// ============================================================================

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TransferState {
    /// Funds locked, awaiting unlock ledger
    Pending = 0,
    /// Funds released to recipient
    Executed = 1,
    /// Cancelled and refunded to sender
    Cancelled = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledTransfer {
    /// Unique ID
    pub id: u64,
    /// Address that locked the funds
    pub sender: Address,
    /// Address that receives the funds after unlock
    pub recipient: Address,
    /// Token contract address
    pub token: Address,
    /// Locked amount
    pub amount: i128,
    /// Ledger sequence after which execution is permitted
    pub unlock_ledger: u32,
    /// Current state
    pub state: TransferState,
    /// Ledger timestamp when scheduled
    pub created_at: u64,
}

// ============================================================================
// Storage Keys
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    TxCount,
    Paused,
    Version,
    Transfer(u64),
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct TimeLockedTransactionsContract;

#[contractimpl]
impl TimeLockedTransactionsContract {
    /// Initialize the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::TxCount, &0u64);
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

    pub fn get_admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)
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

    pub fn get_transfer_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TxCount)
            .unwrap_or(0)
    }

    pub fn get_transfer(env: Env, tx_id: u64) -> Result<ScheduledTransfer, Error> {
        let transfer = env
            .storage()
            .persistent()
            .get(&DataKey::Transfer(tx_id))
            .ok_or(Error::TransferNotFound)?;
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Transfer(tx_id), TX_TTL_THRESHOLD, TX_TTL_EXTEND);
        Ok(transfer)
    }

    // ========================================================================
    // Transfer Lifecycle
    // ========================================================================

    /// Lock tokens for time-delayed delivery to a recipient.
    ///
    /// The sender's tokens are transferred to this contract immediately.
    /// Returns the new transfer ID.
    pub fn schedule_transfer(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        amount: i128,
        unlock_ledger: u32,
    ) -> Result<u64, Error> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(Error::ContractPaused);
        }

        sender.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let current_ledger = env.ledger().sequence();
        if unlock_ledger <= current_ledger {
            return Err(Error::InvalidUnlockTime);
        }

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TxCount)
            .unwrap_or(0);
        count = count.checked_add(1).ok_or(Error::TxIdOverflow)?;

        // Pull tokens from sender into this contract now
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        let transfer = ScheduledTransfer {
            id: count,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token,
            amount,
            unlock_ledger,
            state: TransferState::Pending,
            created_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Transfer(count), &transfer);
        env.storage().persistent().extend_ttl(
            &DataKey::Transfer(count),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );

        env.storage().instance().set(&DataKey::TxCount, &count);
        bump_instance(&env);

        emit_scheduled(&env, count, sender, recipient, amount, unlock_ledger);
        Ok(count)
    }

    /// Release locked funds to the recipient once the unlock ledger has been reached.
    ///
    /// Either the sender or the recipient may trigger execution.
    pub fn execute_transfer(env: Env, caller: Address, tx_id: u64) -> Result<(), Error> {
        caller.require_auth();

        let mut transfer: ScheduledTransfer = env
            .storage()
            .persistent()
            .get(&DataKey::Transfer(tx_id))
            .ok_or(Error::TransferNotFound)?;

        if transfer.state == TransferState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if transfer.state == TransferState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        if caller != transfer.sender && caller != transfer.recipient {
            return Err(Error::Unauthorized);
        }

        let current_ledger = env.ledger().sequence();
        if current_ledger < transfer.unlock_ledger {
            return Err(Error::NotUnlockedYet);
        }

        let token_client = token::Client::new(&env, &transfer.token);
        token_client.transfer(
            &env.current_contract_address(),
            &transfer.recipient,
            &transfer.amount,
        );

        let recipient = transfer.recipient.clone();
        let amount = transfer.amount;
        transfer.state = TransferState::Executed;
        env.storage()
            .persistent()
            .set(&DataKey::Transfer(tx_id), &transfer);
        env.storage().persistent().extend_ttl(
            &DataKey::Transfer(tx_id),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );

        bump_instance(&env);
        emit_executed(&env, tx_id, recipient, amount);
        Ok(())
    }

    /// Cancel a pending transfer and return funds to the sender.
    ///
    /// Only the original sender or the admin may cancel.
    pub fn cancel_transfer(env: Env, caller: Address, tx_id: u64) -> Result<(), Error> {
        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        let mut transfer: ScheduledTransfer = env
            .storage()
            .persistent()
            .get(&DataKey::Transfer(tx_id))
            .ok_or(Error::TransferNotFound)?;

        if transfer.state == TransferState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if transfer.state == TransferState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        if caller != transfer.sender && caller != admin {
            return Err(Error::Unauthorized);
        }

        let token_client = token::Client::new(&env, &transfer.token);
        token_client.transfer(
            &env.current_contract_address(),
            &transfer.sender,
            &transfer.amount,
        );

        transfer.state = TransferState::Cancelled;
        env.storage()
            .persistent()
            .set(&DataKey::Transfer(tx_id), &transfer);
        env.storage().persistent().extend_ttl(
            &DataKey::Transfer(tx_id),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );

        bump_instance(&env);
        emit_cancelled(&env, tx_id, caller);
        Ok(())
    }
}

mod test;
