use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract already initialized
    AlreadyInitialized = 1,
    /// Caller is not authorized (not admin or guardian)
    Unauthorized = 2,
    /// Admin not set
    AdminNotSet = 3,
    /// Contract is already paused
    AlreadyPaused = 4,
    /// Contract is not paused
    NotPaused = 5,
    /// Guardian already registered
    GuardianAlreadyExists = 6,
    /// Guardian not found
    GuardianNotFound = 7,
}
