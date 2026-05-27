use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Deposited {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
}

