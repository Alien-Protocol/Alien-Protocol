use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum Datakey {
    Position(Address, Address),
    PositionIndex,
    LendingPool,
    LiquidationEngine,
    Admin,
    Oracle,
    Paused,
}
