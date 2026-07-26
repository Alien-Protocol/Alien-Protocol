use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u32)]
pub enum VaultError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    VaultPaused = 4,
    AlreadyPaused = 5,
    NotPaused = 6,
    InvalidInputs = 7,
    UnsupportedAsset = 8,
    AlreadySupported = 9,
    AssetNotFound = 10,
    NoPosition = 11,
    BelowMinCollateralRatio = 12,
    AlreadyAdmin = 13,
    LendingPoolNotSet = 14,
    OracleNotConfigured = 15,
    PriceNotFound = 16,
    MathOverflow = 17,
}
