use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VaultError {
    InvalidInputs = 1,
    NoPosition = 2,
    VaultPaused = 3,
    InsufficientCollateral = 4,
    InsufficientBalance = 5,
    AlreadyInitialized = 6,
}
