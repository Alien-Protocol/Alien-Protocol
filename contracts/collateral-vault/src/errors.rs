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
    AlreadyPaused = 12,
    NotPaused = 13,
    AlreadyInitialized = 14,
    InvalidAddress = 15,
    UpgradeUnauthorized = 16,
    MigrationAlreadyApplied = 17,
    MigrationSkipped = 18,
    MigrationOutOfOrder = 19,
    /// `limit` was 0 or exceeded [`crate::types::MAX_PAGE_LIMIT`].
    PageLimitExceeded = 20,
}
