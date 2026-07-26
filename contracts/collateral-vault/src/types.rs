use crate::admin::Role;
use soroban_sdk::{contracttype, Address, Vec};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralAsset {
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub user: Address,
    pub collateral: Vec<CollateralAsset>,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Admin,
    Paused,
    LendingPool,
    SupportedAsset(Address),
    SupportedAssets,
    Position(Address, Address),
    PositionIndex,
    UserAssets(Address),
    Oracle,
    LiquidationEngine,
    Pool,
    PendingAdmin,
    Role(Role, Address),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
