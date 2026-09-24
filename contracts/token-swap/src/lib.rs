#![no_std]

mod errors;
mod events;

use errors::Error;
use events::{
    emit_initialized, emit_offer_accepted, emit_offer_cancelled, emit_offer_created, emit_paused,
    emit_unpaused,
};
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANCE_TTL_THRESHOLD: u32 = 100_000;
const INSTANCE_TTL_EXTEND: u32 = 518_400;
const OFFER_TTL_THRESHOLD: u32 = 100_000;
const OFFER_TTL_EXTEND: u32 = 518_400;

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
pub enum OfferState {
    /// Open and waiting for a taker
    Open = 0,
    /// Swap completed
    Filled = 1,
    /// Cancelled, offer tokens returned to maker
    Cancelled = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Offer {
    /// Unique ID
    pub id: u64,
    /// Address creating the swap offer
    pub maker: Address,
    /// Token the maker is offering
    pub offer_token: Address,
    /// Amount of offer_token the maker is depositing
    pub offer_amount: i128,
    /// Token the maker wants in return
    pub want_token: Address,
    /// Amount of want_token the maker expects from the taker
    pub want_amount: i128,
    /// Unix timestamp after which the offer cannot be accepted (0 = no expiry)
    pub expiry: u64,
    /// Current state
    pub state: OfferState,
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
    OfferCount,
    Paused,
    Version,
    Offer(u64),
    Locked,
}

// ============================================================================
// Contract
// ============================================================================

#[contract]
pub struct TokenSwapContract;

#[contractimpl]
impl TokenSwapContract {
    /// Initialize the token swap contract with an admin.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::OfferCount, &0u64);
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

    pub fn get_offer_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::OfferCount)
            .unwrap_or(0)
    }

    pub fn get_offer(env: Env, offer_id: u64) -> Result<Offer, Error> {
        let offer = env
            .storage()
            .persistent()
            .get(&DataKey::Offer(offer_id))
            .ok_or(Error::OfferNotFound)?;
        env.storage().persistent().extend_ttl(
            &DataKey::Offer(offer_id),
            OFFER_TTL_THRESHOLD,
            OFFER_TTL_EXTEND,
        );
        Ok(offer)
    }

    // ========================================================================
    // Offer Lifecycle
    // ========================================================================

    /// Create a swap offer. The maker's offer_token is deposited into this contract.
    ///
    /// Pass `expiry = 0` for no expiry.
    /// Returns the new offer ID.
    pub fn create_offer(
        env: Env,
        maker: Address,
        offer_token: Address,
        offer_amount: i128,
        want_token: Address,
        want_amount: i128,
        expiry: u64,
    ) -> Result<u64, Error> {
        require_not_locked(&env)?;

        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(Error::ContractPaused);
        }

        maker.require_auth();

        if offer_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if want_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if offer_token == want_token {
            return Err(Error::SameToken);
        }

        if expiry != 0 && expiry <= env.ledger().timestamp() {
            return Err(Error::OfferExpired);
        }

        let mut count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::OfferCount)
            .unwrap_or(0);
        count = count.checked_add(1).ok_or(Error::OfferIdOverflow)?;

        let offer = Offer {
            id: count,
            maker: maker.clone(),
            offer_token: offer_token.clone(),
            offer_amount,
            want_token,
            want_amount,
            expiry,
            state: OfferState::Open,
            created_at: env.ledger().timestamp(),
        };

        // Effect: record offer before external call (CEI)
        env.storage()
            .persistent()
            .set(&DataKey::Offer(count), &offer);
        env.storage().persistent().extend_ttl(
            &DataKey::Offer(count),
            OFFER_TTL_THRESHOLD,
            OFFER_TTL_EXTEND,
        );
        env.storage().instance().set(&DataKey::OfferCount, &count);
        bump_instance(&env);

        // Interaction: pull offer tokens from maker under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &offer_token).transfer(
            &maker,
            &env.current_contract_address(),
            &offer_amount,
        );
        set_unlocked(&env);

        emit_offer_created(&env, count, maker, offer_amount, want_amount);
        Ok(count)
    }

    /// Accept an open offer. Atomically swaps both sides.
    ///
    /// The taker sends want_token to the maker and receives offer_token from this contract.
    /// The `min_out` parameter ensures protection against sandwich attacks and slippage.
    /// The actual_out (offer_amount) must be >= min_out or the transaction reverts.
    pub fn accept_offer(
        env: Env,
        taker: Address,
        offer_id: u64,
        min_out: i128,
    ) -> Result<(), Error> {
        require_not_locked(&env)?;

        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);
        if paused {
            return Err(Error::ContractPaused);
        }

        taker.require_auth();

        let mut offer: Offer = env
            .storage()
            .persistent()
            .get(&DataKey::Offer(offer_id))
            .ok_or(Error::OfferNotFound)?;

        if offer.state == OfferState::Filled {
            return Err(Error::AlreadyFilled);
        }
        if offer.state == OfferState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        if offer.expiry != 0 && env.ledger().timestamp() > offer.expiry {
            return Err(Error::OfferExpired);
        }

        // Validate slippage: actual_out must be >= min_out to prevent sandwich attacks
        if offer.offer_amount < min_out {
            return Err(Error::SlippageExceeded);
        }

        let want_token = offer.want_token.clone();
        let offer_token = offer.offer_token.clone();
        let maker = offer.maker.clone();
        let want_amount = offer.want_amount;
        let offer_amount = offer.offer_amount;

        // Effect: mark filled before external calls (CEI)
        offer.state = OfferState::Filled;
        env.storage()
            .persistent()
            .set(&DataKey::Offer(offer_id), &offer);
        env.storage().persistent().extend_ttl(
            &DataKey::Offer(offer_id),
            OFFER_TTL_THRESHOLD,
            OFFER_TTL_EXTEND,
        );
        bump_instance(&env);

        // Interaction: atomic swap under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &want_token).transfer(&taker, &maker, &want_amount);
        token::Client::new(&env, &offer_token).transfer(
            &env.current_contract_address(),
            &taker,
            &offer_amount,
        );
        set_unlocked(&env);

        emit_offer_accepted(&env, offer_id, taker);
        Ok(())
    }

    /// Cancel an open offer and return offer_token to the maker.
    ///
    /// The maker may cancel at any time. The admin may cancel for emergency.
    /// After expiry, any caller may trigger a cancel on behalf of the maker.
    pub fn cancel_offer(env: Env, caller: Address, offer_id: u64) -> Result<(), Error> {
        require_not_locked(&env)?;

        caller.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::AdminNotSet)?;

        let mut offer: Offer = env
            .storage()
            .persistent()
            .get(&DataKey::Offer(offer_id))
            .ok_or(Error::OfferNotFound)?;

        if offer.state == OfferState::Filled {
            return Err(Error::AlreadyFilled);
        }
        if offer.state == OfferState::Cancelled {
            return Err(Error::AlreadyCancelled);
        }

        let now = env.ledger().timestamp();
        let expired = offer.expiry != 0 && now > offer.expiry;

        if caller != offer.maker && caller != admin && !expired {
            return Err(Error::Unauthorized);
        }

        let offer_token = offer.offer_token.clone();
        let maker = offer.maker.clone();
        let offer_amount = offer.offer_amount;

        // Effect: mark cancelled before external call (CEI)
        offer.state = OfferState::Cancelled;
        env.storage()
            .persistent()
            .set(&DataKey::Offer(offer_id), &offer);
        env.storage().persistent().extend_ttl(
            &DataKey::Offer(offer_id),
            OFFER_TTL_THRESHOLD,
            OFFER_TTL_EXTEND,
        );
        bump_instance(&env);

        // Interaction: return tokens to maker under reentrancy lock
        set_locked(&env);
        token::Client::new(&env, &offer_token).transfer(
            &env.current_contract_address(),
            &maker,
            &offer_amount,
        );
        set_unlocked(&env);

        emit_offer_cancelled(&env, offer_id, caller);
        Ok(())
    }
}

mod test;
