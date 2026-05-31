use soroban_sdk::{contracttype, Address};

/// Represents a user's collateral position across all assets.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralAsset {
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// The owner of this position.
    pub user: Address,
    /// All collateral assets held by this user.
    pub collateral: Vec<CollateralAsset>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Admin,
    Paused,
    SupportedAsset(Address),
    SupportedAssets,
    Position(Address, Address), // (user, asset)
    PositionIndex,
    /// Tracks which assets a user has ever deposited into (used to build Position).
    UserAssets(Address),
    Oracle,
    LiquidationEngine,
    Pool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
