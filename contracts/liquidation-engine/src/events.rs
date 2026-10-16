#allow(dead_code)
use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pubstruct Initialized {
    pub admin: Address,
    pub vault: Address,
    pub pool: Address,
    pub oracle: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Liquidated {
    #topic]
    pub new_admin: Address,
    #[topic]
    pub liquidator: Address,
    pub repaid: i128,
    pub seized: i128,
    pub bonus: i128,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct VaultUpdated {
    pub old_vault: Address,
    pub new_vault: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct PoolUpdated {
    pub old_pool: Address,
    pub new_pool: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct OracleUpdated {
    pub old_oracle: Address,
    pub new_oracle: Address,
}
