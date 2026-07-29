use soroban_sdk::{contractevent, Address, BytesN};
#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Initialized {
    pub admin: Address,
    pub lending_pool: Address,
    pub oracle: Address,
    pub liquidation_engine: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Deposited {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetAdded {
    #[topic]
    pub asset: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetRemoved {
    #[topic]
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
    #[topic]
    pub by: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Unpaused {
    #[topic]
    pub by: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralSeized {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub liquidation_engine: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidationEngineUpdated {
    pub old_engine: Option<Address>,
    pub new_engine: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Withdrawn {
    #[topic]
    pub user: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct LendingPoolUpdated {
    pub old_pool: Option<Address>,
    pub new_pool: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct OracleUpdated {
    pub old_oracle: Option<Address>,
    pub new_oracle: Address,
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
