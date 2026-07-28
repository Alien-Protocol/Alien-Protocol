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

/// Emitted when an asset is transitioned to `DepositDisabled`.
/// Existing positions remain valid; new deposits are rejected.
#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetDelisted {
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
pub struct PoolUpdated {
    pub old_pool: Option<Address>,
    pub new_pool: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct ContractUpgraded {
    pub old_hash: Option<BytesN<32>>,
    pub new_hash: BytesN<32>,
}

/// Emitted when per-asset risk parameters are set or updated (issue #579).
/// Carries both the previous and the new configuration so off-chain indexers
/// can track every governance action without additional storage reads.
#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct RiskParamsUpdated {
    #[topic]
    pub asset: Address,
    pub old_params: Option<crate::types::AssetRiskParams>,
    pub new_params: crate::types::AssetRiskParams,
}
