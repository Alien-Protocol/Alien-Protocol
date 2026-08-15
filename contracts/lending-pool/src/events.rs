#![allow(dead_code)]
use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Initialized {
    pub admin: Address,
    pub vault: Address,
    pub oracle: Address,
    pub borrow_asset: Address,
    pub interest_rate_bps: u32,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct OperationPaused {
    pub by: Address,
    pub operation: soroban_sdk::Symbol,
    pub reason: soroban_sdk::Symbol,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct OperationUnpaused {
    pub by: Address,
    pub operation: soroban_sdk::Symbol,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Supplied {
    #[topic]
    pub user: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidityWithdrawn {
    #[topic]
    pub user: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Borrowed {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Repaid {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub interest_paid: i128,
    pub principal_paid: i128,
}
