use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone)]
pub struct Position {
    pub amount: i128,
}

#[contracttype]
pub enum Datakey {
    Position(Address, Address),
    PositionIndex,
    LendingPool,
    LiquidationEngine,
    Admin,
    Paused,
}
