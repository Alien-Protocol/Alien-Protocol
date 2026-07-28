//! External contract client trait declarations.
//!
//! Keeping these here prevents lib.rs from mixing ABI surface with
//! dependency declarations, and lets every domain module import the
//! clients it needs without a circular path through lib.

use crate::types::PriceData;
use soroban_sdk::{Address, Env};

#[soroban_sdk::contractclient(name = "OracleClient")]
pub trait Oracle {
    fn get_price(env: Env, asset: Address) -> Option<PriceData>;
    fn get_price_or_fail(env: Env, asset: Address) -> PriceData;
}

#[soroban_sdk::contractclient(name = "LendingPoolClient")]
pub trait LendingPool {
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn is_liquidatable(user: &Address) -> bool;
}
