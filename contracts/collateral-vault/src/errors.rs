use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VaultError {
    InvalidInputs = 1,
    VaultPaused = 2,
    UnsupportedAsset = 3,
    AlreadySupported = 4,
    AssetNotFound = 5,
    NoPosition = 6,
    StalePrice = 7,
    Unauthorized = 8,
    NotInitialized = 9,
    BelowMinCollateralRatio = 10,
    AlreadyAdmin = 11,
}
