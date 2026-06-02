use soroban_sdk::{contracttype, Address, Vec};

/// Represents a single collateral asset held by a user.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralAsset {
    pub asset: Address,
    pub amount: i128,
}

/// Represents a user's collateral position across all assets.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// The owner of this position.
    pub user: Address,
    /// All collateral assets held by this user.
    pub collateral: Vec<CollateralAsset>,
}

/// Storage keys for persistent contract state.
/// Core keys required for Issue #471 initialization:
/// - Admin: Contract administrator address
/// - Paused: Contract pause state
/// - LendingPool: Lending pool address
/// - SupportedAsset(Address): Tracks supported collateral assets
/// - Position(Address, Address): Stores balances (user, asset)
/// - PositionIndex: Index of all users with active positions
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    /// Admin address key
    Admin,
    /// Paused state key
    Paused,
    /// Lending pool address key (Issue #471)
    LendingPool,
    /// Supported asset key: stores whether a specific asset is supported
    SupportedAsset(Address),
    /// List of all supported assets
    SupportedAssets,
    /// Position key: stores balance for a user's position in an asset
    Position(Address, Address), // (user, asset)
    /// Position index key: tracks all users with active positions
    PositionIndex,
    /// Tracks which assets a user has ever deposited into
    UserAssets(Address),
    /// Oracle adapter address
    Oracle,
    /// Liquidation engine address
    LiquidationEngine,
    /// Lending pool address (alternative key)
    Pool,
}

/// Price data from the oracle.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
