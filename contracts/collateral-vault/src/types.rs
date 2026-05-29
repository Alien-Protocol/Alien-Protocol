use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Admin,
    Paused,
    SupportedAsset(Address),
    Position(Address, Address), // (user, asset)
    PositionIndex,
    UserPosition(Address),      // whole-position record keyed by user
}

/// Aggregated collateral position for a single user.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub user: Address,
    pub amount: i128,
}
