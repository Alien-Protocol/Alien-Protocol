use soroban_sdk::{contractevent, Address, BytesN};
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
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
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

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct ContractUpgraded {
    pub actor: Address,
    pub old_contract_version: u32,
    pub new_contract_version: u32,
    pub wasm_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct StorageMigrated {
    pub actor: Address,
    pub old_storage_schema_version: u32,
    pub new_storage_schema_version: u32,
}
