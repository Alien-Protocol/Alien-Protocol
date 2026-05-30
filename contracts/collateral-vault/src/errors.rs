use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VaultError {
    InvalidInputs = 1,
    NoPosition = 2,
    Unauthorized = 3,
    VaultPaused = 4,
    InsufficientCollateral = 5,
    InsufficientBalance = 6,
}
