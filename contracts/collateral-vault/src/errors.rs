use soroban_sdk::contracterror;

/// Typed error taxonomy for the collateral-vault contract.
///
/// # Stability contract
/// Numeric codes are part of the on-chain ABI.  Once a code is published it
/// **must not** be renumbered.  New variants must be appended at the end so
/// that existing clients that pattern-match on numeric values continue to work.
///
/// | Code | Variant                  | Meaning                                        |
/// |------|--------------------------|------------------------------------------------|
/// |  1   | InvalidInputs            | Generic guard: zero / negative / bad argument  |
/// |  2   | VaultPaused              | Operation blocked because the vault is paused  |
/// |  3   | UnsupportedAsset         | Asset has not been allow-listed                |
/// |  4   | AlreadySupported         | Asset is already in the supported-asset list   |
/// |  5   | AssetNotFound            | Asset is not in the supported-asset list       |
/// |  6   | NoPosition               | User has no active collateral position         |
/// |  7   | StalePrice               | Oracle price exceeds the maximum allowed age   |
/// |  8   | Unauthorized             | Caller is not the registered authority         |
/// |  9   | NotInitialized           | Required config address is missing             |
/// | 10   | BelowMinCollateralRatio  | Withdrawal would violate the 110 % floor       |
/// | 11   | AlreadyAdmin             | Proposed new admin is the same as the current  |
/// | 12   | AlreadyPaused            | Vault is already paused                        |
/// | 13   | NotPaused                | Vault is not paused, cannot unpause            |
/// | 14   | AlreadyInitialized       | `initialize` called on an already-live vault   |
/// | 15   | OracleNotConfigured      | Oracle address has not been set                |
/// | 16   | PriceNotFound            | Oracle returned no price for the asset         |
/// | 17   | ArithmeticOverflow       | Integer arithmetic would overflow              |
/// | 18   | LiquidationEngineNotSet  | Liquidation-engine address has not been set    |
/// | 19   | PoolNotSet               | Lending-pool address has not been set          |
/// | 20   | InsufficientBalance      | Requested amount exceeds the recorded balance  |
#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VaultError {
    // ── pre-existing published codes (must not be renumbered) ──────────────
    /// Zero, negative, or otherwise malformed input argument.
    InvalidInputs = 1,
    /// The vault is paused; state-mutating operations are blocked.
    VaultPaused = 2,
    /// The asset is not in the supported-asset allow-list.
    UnsupportedAsset = 3,
    /// The asset is already in the supported-asset allow-list.
    AlreadySupported = 4,
    /// The asset was not found in the supported-asset allow-list.
    AssetNotFound = 5,
    /// The user has no active collateral position.
    NoPosition = 6,
    /// The oracle price is older than the maximum allowed staleness window.
    StalePrice = 7,
    /// The caller is not the registered authority for this operation.
    Unauthorized = 8,
    /// A required configuration address (admin, oracle, pool…) has not been set.
    NotInitialized = 9,
    /// The withdrawal would bring the collateral ratio below the 110 % minimum.
    BelowMinCollateralRatio = 10,
    /// The proposed new admin is identical to the current admin.
    AlreadyAdmin = 11,
    /// `pause` was called but the vault is already paused.
    AlreadyPaused = 12,
    /// `unpause` was called but the vault is not currently paused.
    NotPaused = 13,

    // ── new codes (appended; existing clients unaffected) ──────────────────
    /// `initialize` was called on a vault that is already initialized.
    AlreadyInitialized = 14,
    /// The oracle address has not been configured.
    OracleNotConfigured = 15,
    /// The oracle has no price entry for the requested asset.
    PriceNotFound = 16,
    /// An arithmetic operation would overflow.
    ArithmeticOverflow = 17,
    /// The liquidation-engine address has not been configured.
    LiquidationEngineNotSet = 18,
    /// The lending-pool address has not been configured.
    PoolNotSet = 19,
    /// The requested withdrawal amount exceeds the user's recorded balance.
    InsufficientBalance = 20,
}
