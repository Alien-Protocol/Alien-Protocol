use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Deposited {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetAdded {
    pub asset: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetRemoved {
    pub asset: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Paused {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Unpaused {
    pub paused: bool,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralSeized {
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub liquidation_engine: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidationEngineSet {
    pub engine: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Withdrawn {
    pub receiver: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct LendingPoolUpdated {
    pub lending_pool: Address,
}
