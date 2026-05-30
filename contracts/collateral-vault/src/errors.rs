use soroban_sdk::contracterror;

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VaultError {
    InvalidInputs = 1,
    VaultPaused = 2,
    UnsupportedAsset = 3,

    AlreadySupported = 4,

    AssetNotFound = 4,
    NoPosition = 5,
    StalePrice = 6,

}
