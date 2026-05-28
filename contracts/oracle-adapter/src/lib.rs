#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    pub fn get_price(env: Env, asset: Address) -> Option<PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let price_data = PriceData { price, timestamp };
        env.storage().persistent().set(&asset, &price_data);
    }
}

mod test;
