#![no_std]
use soroban_sdk::{contract, contractimpl, vec, Env, String, Vec};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    pub fn initilized(env: Env, reflactor_address: Address) {}
}

mod test;
