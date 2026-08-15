use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Admin,
    Vault,
    Pool,
    Oracle,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidationResult {
    pub user: Address,
    pub repaid: i128,
    pub seized: i128,
    pub bonus: i128,
}
