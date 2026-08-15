use soroban_sdk::{contracttype, Address};

pub use shared::types::Debt;

/// Identifies which pool operation to pause or unpause.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum PauseFlag {
    Supply,
    Borrow,
    Repay,
    WithdrawLiquidity,
}

impl PauseFlag {
    pub fn bit(&self) -> u32 {
        match self {
            PauseFlag::Supply => 1 << 0,
            PauseFlag::Borrow => 1 << 1,
            PauseFlag::Repay => 1 << 2,
            PauseFlag::WithdrawLiquidity => 1 << 3,
        }
    }
}

#[allow(dead_code)]
pub fn pause_flag_symbol(env: &soroban_sdk::Env, flag: &PauseFlag) -> soroban_sdk::Symbol {
    match flag {
        PauseFlag::Supply => soroban_sdk::Symbol::new(env, "supply"),
        PauseFlag::Borrow => soroban_sdk::Symbol::new(env, "borrow"),
        PauseFlag::Repay => soroban_sdk::Symbol::new(env, "repay"),
        PauseFlag::WithdrawLiquidity => soroban_sdk::Symbol::new(env, "withdraw_liq"),
    }
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Admin,
    Vault,
    Oracle,
    BorrowAsset,
    InterestRateBps,
    PauseMask,
    TotalSupply,
    TotalBorrowed,
    Supply(Address),
    Debt(Address),
}
