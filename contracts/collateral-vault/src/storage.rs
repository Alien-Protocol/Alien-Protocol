use soroban_sdk::{Address, Env};
use crate::types::Datakey;

pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&Datakey::Admin).unwrap()
}

pub fn set_lending_pool(env: &Env, lending_pool: &Address) {
    env.storage().instance().set(&Datakey::LendingPool, lending_pool);
}

pub fn get_lending_pool(env: &Env) -> Address {
    env.storage().instance().get(&Datakey::LendingPool).unwrap()
}

