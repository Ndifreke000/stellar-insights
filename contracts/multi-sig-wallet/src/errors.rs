use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Contract not yet initialized
    NotInitialized = 2,
    /// Caller is not an owner
    Unauthorized = 3,
    /// Threshold must be at least 1
    InvalidThreshold = 4,
    /// Owners list is empty
    InvalidOwners = 5,
    /// Transaction not found
    TransactionNotFound = 6,
    /// Owner has already confirmed this transaction
    AlreadyConfirmed = 7,
    /// Owner has not confirmed this transaction
    NotConfirmed = 8,
    /// Transaction has already been executed
    AlreadyExecuted = 9,
    /// Transaction has already been cancelled
    AlreadyCancelled = 10,
    /// Not enough confirmations to execute
    ThresholdNotMet = 11,
    /// Amount must be greater than zero
    InvalidAmount = 12,
    /// Transaction ID counter overflow
    TxIdOverflow = 13,
    /// Duplicate address in owners list
    DuplicateOwner = 14,
    /// Reentrant call detected
    Reentrancy = 15,
    /// Threshold exceeds the number of owners
    ThresholdExceedsSigCount = 16,
}
