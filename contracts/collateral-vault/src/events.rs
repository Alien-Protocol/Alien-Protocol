use soroban_sdk::{contractevent, Address};

#[contractevent]
#[derive(Clone)]
pub struct LendingPoolUpdated {
    pub lending_pool: Address,
}
