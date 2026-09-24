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
    /// Transfer not found
    TransferNotFound = 4,
    /// Amount must be greater than zero
    InvalidAmount = 5,
    /// Unlock ledger must be in the future
    InvalidUnlockTime = 6,
    /// Unlock ledger has not been reached yet
    NotUnlockedYet = 7,
    /// Transfer already executed
    AlreadyExecuted = 8,
    /// Transfer already cancelled
    AlreadyCancelled = 9,
    /// Transfer ID counter overflow
    TxIdOverflow = 10,
    /// Contract is paused
    ContractPaused = 11,
}
