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
    /// Arithmetic overflow in financial computation.
    MathOverflow = 14,
    /// Arithmetic underflow in financial computation.
    MathUnderflow = 15,
    /// Division by zero in financial computation.
    MathDivisionByZero = 16,
    /// Detectable precision loss (result truncated to zero despite non-zero
    /// inputs). This protects against silent value truncation in price-scaled
    /// calculations.
    MathPrecisionLoss = 17,
}
