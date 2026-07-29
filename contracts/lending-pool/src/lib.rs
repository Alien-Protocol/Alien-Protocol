//! Lending-pool contract — scaffolded implementation.
//!
//! This contract exposes the [`LendingPoolInterface`] defined in
//! `shared::interfaces`.  The function signatures here are the authoritative
//! deployed surface; the collateral-vault's [`LendingPoolClient`] is generated
//! from those shared interface definitions and must always match what this
//! contract implements.
//!
//! # Interface compliance (INTERFACE_VERSION = 1)
//!
//! | Shared interface method       | Implemented here | Notes                          |
//! |-------------------------------|------------------|--------------------------------|
//! | `get_user_debt(env, user)`    | ✅               | Scaffolded — returns 0         |
//! | `is_liquidatable(env, user)`  | ✅               | Scaffolded — returns false     |
//!
//! Full debt-accounting and liquidation-threshold logic will replace the
//! scaffolded stubs in a future issue.

#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct PoolContract;

#[contractimpl]
impl PoolContract {
    /// Returns the total outstanding debt for `user`.
    ///
    /// **Scaffolded:** always returns `0` until full debt accounting is
    /// implemented.  Signature matches [`shared::interfaces::LendingPoolInterface`].
    pub fn get_user_debt(_env: Env, _user: Address) -> i128 {
        0
    }

    /// Returns `true` when `user`'s position is eligible for liquidation.
    ///
    /// **Scaffolded:** always returns `false` until liquidation-threshold logic
    /// is implemented.  Signature matches [`shared::interfaces::LendingPoolInterface`].
    pub fn is_liquidatable(_env: Env, _user: Address) -> bool {
        false
    }
}

mod test;
