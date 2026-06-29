#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{
    emit_confirmation_revoked, emit_initialized, emit_tx_cancelled, emit_tx_confirmed,
    emit_tx_executed, emit_tx_proposed,
};
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String, Vec};

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
pub enum TxState {
    /// Pending confirmations
    Pending = 0,
    /// Successfully executed
    Executed = 1,
    /// Cancelled by proposer
    Cancelled = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    /// Unique ID
    pub id: u64,
    /// Owner who proposed the transaction
    pub proposer: Address,
    /// Recipient of the token transfer
    pub to: Address,
    /// Token contract address
    pub token: Address,
    /// Amount to transfer
    pub amount: i128,
    /// Number of owner confirmations collected
    pub confirmations: u32,
    /// Current state
    pub state: TxState,
    /// Ledger timestamp when proposed
    pub created_at: u64,
}

// ============================================================================
// Storage Keys
// ============================================================================

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Owners,
    Threshold,
    TxCount,
    Version,
    Tx(u64),
    Confirmers(u64),
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct MultiSigWalletContract;

#[contractimpl]
impl MultiSigWalletContract {
    /// Initialize the wallet with a set of owners and a confirmation threshold.
    pub fn initialize(
        env: Env,
        owners: Vec<Address>,
        threshold: u32,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Threshold) {
            return Err(Error::AlreadyInitialized);
        }

        let owner_count = owners.len();
        if owner_count == 0 {
            return Err(Error::InvalidOwners);
        }
        if threshold == 0 {
            return Err(Error::InvalidThreshold);
        }
        if threshold > owner_count {
            return Err(Error::ThresholdExceedsSigCount);
        }

        // Reject duplicate owners
        for i in 0..owner_count {
            let a = owners.get(i).ok_or(Error::InvalidOwners)?;
            for j in (i + 1)..owner_count {
                let b = owners.get(j).ok_or(Error::InvalidOwners)?;
                if a == b {
                    return Err(Error::DuplicateOwner);
                }
            }
        }

        env.storage().instance().set(&DataKey::Owners, &owners);
        env.storage().instance().set(&DataKey::Threshold, &threshold);
        env.storage().instance().set(&DataKey::TxCount, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::Version, &String::from_str(&env, VERSION));
        env.storage()
            .instance()
            .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND);

        emit_initialized(&env, threshold);
        Ok(())
    }

    pub fn get_version(env: Env) -> String {
        env.storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or_else(|| String::from_str(&env, VERSION))
    }

    pub fn get_owners(env: Env) -> Result<Vec<Address>, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Owners)
            .ok_or(Error::NotInitialized)
    }

    pub fn get_threshold(env: Env) -> Result<u32, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Threshold)
            .ok_or(Error::NotInitialized)
    }

    pub fn get_tx_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TxCount)
            .unwrap_or(0)
    }

    pub fn get_tx(env: Env, tx_id: u64) -> Result<Transaction, Error> {
        let tx = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .ok_or(Error::TransactionNotFound)?;
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Tx(tx_id), TX_TTL_THRESHOLD, TX_TTL_EXTEND);
        Ok(tx)
    }

    pub fn get_confirmers(env: Env, tx_id: u64) -> Result<Vec<Address>, Error> {
        let confirmers = env
            .storage()
            .persistent()
            .get(&DataKey::Confirmers(tx_id))
            .ok_or(Error::TransactionNotFound)?;
        env.storage().persistent().extend_ttl(
            &DataKey::Confirmers(tx_id),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );
        Ok(confirmers)
    }

    // ========================================================================
    // Owner Verification
    // ========================================================================

    fn require_owner(env: &Env, addr: &Address) -> Result<(), Error> {
        let owners: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Owners)
            .ok_or(Error::NotInitialized)?;
        for i in 0..owners.len() {
            let o = owners.get(i).ok_or(Error::NotInitialized)?;
            if &o == addr {
                return Ok(());
            }
        }
        Err(Error::Unauthorized)
    }

    // ========================================================================
    // Transaction Lifecycle
    // ========================================================================

    /// Propose a new token transfer. Proposer's confirmation is automatically added.
    ///
    /// Returns the new transaction ID.
    pub fn propose_tx(
        env: Env,
        proposer: Address,
        to: Address,
        token: Address,
        amount: i128,
    ) -> Result<u64, Error> {
        proposer.require_auth();
        Self::require_owner(&env, &proposer)?;

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::TxCount)
            .unwrap_or(0);
        count = count.checked_add(1).ok_or(Error::TxIdOverflow)?;

        let tx = Transaction {
            id: count,
            proposer: proposer.clone(),
            to,
            token,
            amount,
            confirmations: 1,
            state: TxState::Pending,
            created_at: env.ledger().timestamp(),
        };

        // Auto-confirm for the proposer
        let mut confirmers: Vec<Address> = Vec::new(&env);
        confirmers.push_back(proposer.clone());

        env.storage().persistent().set(&DataKey::Tx(count), &tx);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Tx(count), TX_TTL_THRESHOLD, TX_TTL_EXTEND);
        env.storage()
            .persistent()
            .set(&DataKey::Confirmers(count), &confirmers);
        env.storage().persistent().extend_ttl(
            &DataKey::Confirmers(count),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );

        env.storage().instance().set(&DataKey::TxCount, &count);
        bump_instance(&env);

        emit_tx_proposed(&env, count, proposer, amount);
        Ok(count)
    }

    /// Confirm a pending transaction.
    pub fn confirm_tx(env: Env, signer: Address, tx_id: u64) -> Result<(), Error> {
        signer.require_auth();
        Self::require_owner(&env, &signer)?;

        let mut tx: Transaction = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .ok_or(Error::TransactionNotFound)?;

        if tx.state == TxState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if tx.state == TxState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        let mut confirmers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Confirmers(tx_id))
            .ok_or(Error::TransactionNotFound)?;

        for i in 0..confirmers.len() {
            let c = confirmers.get(i).ok_or(Error::TransactionNotFound)?;
            if c == signer {
                return Err(Error::AlreadyConfirmed);
            }
        }

        confirmers.push_back(signer.clone());
        tx.confirmations = confirmers.len();

        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Tx(tx_id), TX_TTL_THRESHOLD, TX_TTL_EXTEND);
        env.storage()
            .persistent()
            .set(&DataKey::Confirmers(tx_id), &confirmers);
        env.storage().persistent().extend_ttl(
            &DataKey::Confirmers(tx_id),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );

        emit_tx_confirmed(&env, tx_id, signer);
        Ok(())
    }

    /// Revoke a previously given confirmation.
    pub fn revoke_confirmation(env: Env, signer: Address, tx_id: u64) -> Result<(), Error> {
        signer.require_auth();
        Self::require_owner(&env, &signer)?;

        let mut tx: Transaction = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .ok_or(Error::TransactionNotFound)?;

        if tx.state == TxState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if tx.state == TxState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        let confirmers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::Confirmers(tx_id))
            .ok_or(Error::TransactionNotFound)?;

        let mut new_confirmers: Vec<Address> = Vec::new(&env);
        let mut found = false;
        for i in 0..confirmers.len() {
            let c = confirmers.get(i).ok_or(Error::TransactionNotFound)?;
            if c == signer {
                found = true;
            } else {
                new_confirmers.push_back(c);
            }
        }

        if !found {
            return Err(Error::NotConfirmed);
        }

        tx.confirmations = new_confirmers.len();

        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Tx(tx_id), TX_TTL_THRESHOLD, TX_TTL_EXTEND);
        env.storage()
            .persistent()
            .set(&DataKey::Confirmers(tx_id), &new_confirmers);
        env.storage().persistent().extend_ttl(
            &DataKey::Confirmers(tx_id),
            TX_TTL_THRESHOLD,
            TX_TTL_EXTEND,
        );

        emit_confirmation_revoked(&env, tx_id, signer);
        Ok(())
    }

    /// Execute a transaction once the threshold is met.
    ///
    /// Transfers tokens held by this contract to the recipient.
    pub fn execute_tx(env: Env, caller: Address, tx_id: u64) -> Result<(), Error> {
        caller.require_auth();
        Self::require_owner(&env, &caller)?;

        let mut tx: Transaction = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .ok_or(Error::TransactionNotFound)?;

        if tx.state == TxState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if tx.state == TxState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        let threshold: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Threshold)
            .ok_or(Error::NotInitialized)?;

        if tx.confirmations < threshold {
            return Err(Error::ThresholdNotMet);
        }

        let token_client = token::Client::new(&env, &tx.token);
        token_client.transfer(&env.current_contract_address(), &tx.to, &tx.amount);

        let amount = tx.amount;
        tx.state = TxState::Executed;
        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Tx(tx_id), TX_TTL_THRESHOLD, TX_TTL_EXTEND);

        bump_instance(&env);
        emit_tx_executed(&env, tx_id, caller, amount);
        Ok(())
    }

    /// Cancel a pending transaction. Only the proposer may cancel.
    pub fn cancel_tx(env: Env, caller: Address, tx_id: u64) -> Result<(), Error> {
        caller.require_auth();
        Self::require_owner(&env, &caller)?;

        let mut tx: Transaction = env
            .storage()
            .persistent()
            .get(&DataKey::Tx(tx_id))
            .ok_or(Error::TransactionNotFound)?;

        if tx.state == TxState::Executed {
            return Err(Error::AlreadyExecuted);
        }
        if tx.state == TxState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        if caller != tx.proposer {
            return Err(Error::Unauthorized);
        }

        tx.state = TxState::Cancelled;
        env.storage().persistent().set(&DataKey::Tx(tx_id), &tx);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Tx(tx_id), TX_TTL_THRESHOLD, TX_TTL_EXTEND);

        bump_instance(&env);
        emit_tx_cancelled(&env, tx_id, caller);
        Ok(())
    }
}

mod test;
