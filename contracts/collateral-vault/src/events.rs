use soroban_sdk::{contractevent, Address};

#[contractevent]
pub struct Deposited {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
pub struct Withdrawn {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
}
