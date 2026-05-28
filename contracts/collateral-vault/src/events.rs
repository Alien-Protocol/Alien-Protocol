use soroban_sdk::{contractevent, Address};

#[contractevent]
pub struct LendingPoolUpdated {
    pub lending_pool: Address,
}
