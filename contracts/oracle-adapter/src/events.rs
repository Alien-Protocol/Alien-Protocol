use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct Initialized {
    pub admin: Address,
    pub staleness_threshold: u64,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceUpdated {
    pub asset: Address,
    pub price: i128,
    pub timestamp: u64,
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
pub struct FeederRemoved {
    pub feeder: Address,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct StalenessThresholdUpdated {
    pub threshold: u64,
}
