use crate::admin::Role;
use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Initialized {
    pub admin: Address,
    pub lending_pool: Address,
}

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
pub struct AdminProposed {
    pub pending_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminAccepted {
    pub new_admin: Address,
    pub old_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminTransferCancelled {
    pub cancelled_by: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct RoleGranted {
    pub role: Role,
    pub address: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct RoleRevoked {
    pub role: Role,
    pub address: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Paused {
    pub by: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Unpaused {
    pub by: Address,
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
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct LendingPoolUpdated {
    pub lending_pool: Address,
}
