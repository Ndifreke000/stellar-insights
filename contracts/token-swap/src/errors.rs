use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Admin not set
    AdminNotSet = 2,
    /// Caller is not authorized
    Unauthorized = 3,
    /// Offer not found
    OfferNotFound = 4,
    /// Amount must be greater than zero
    InvalidAmount = 5,
    /// Offer has already been filled
    AlreadyFilled = 6,
    /// Offer has already been cancelled
    AlreadyCancelled = 7,
    /// Offer ID counter overflow
    OfferIdOverflow = 8,
    /// Offer has expired
    OfferExpired = 9,
    /// Maker and taker token must differ
    SameToken = 10,
    /// Contract is paused
    ContractPaused = 11,
    /// Reentrant call detected
    Reentrancy = 12,
    /// Actual output less than minimum required (slippage exceeded)
    SlippageExceeded = 13,
}
