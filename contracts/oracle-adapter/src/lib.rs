#![no_std]
use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    // pub fn initilized(env: Env, reflactor_address: Address) {}
}

mod test;
