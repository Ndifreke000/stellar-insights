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
    /// Escrow not found
    EscrowNotFound = 4,
    /// Escrow already funded
    AlreadyFunded = 5,
    /// Escrow not funded yet
    NotFunded = 6,
    /// Escrow is not in a state that allows this action
    InvalidState = 7,
    /// Amount must be greater than zero
    InvalidAmount = 8,
    /// Deadline has already passed
    DeadlineExpired = 9,
    /// Deadline has not passed yet (for refund path)
    DeadlineNotExpired = 10,
    /// Dispute already raised
    AlreadyDisputed = 11,
    /// No active dispute on this escrow
    NotDisputed = 12,
    /// Escrow ID overflow
    EscrowIdOverflow = 13,
    /// Token transfer failed
    TransferFailed = 14,
    /// Contract is paused
    ContractPaused = 15,
    /// Reentrant call detected
    Reentrancy = 16,
}
