use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Paused,
    LendingPool,
    SupportedAsset(Address),
    Position(Address),
    PositionIndex,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub deposited_at: u64,
}
