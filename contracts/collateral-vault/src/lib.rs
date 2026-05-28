#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol};

pub mod errors;
pub mod events;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

use errors::VaultError;

#[contract]
pub struct CollateralVaultContract;

#[contractimpl]
impl CollateralVaultContract {
    pub fn initialize(env: Env, admin: Address, lending_pool: Address) {
        if storage::has_admin(&env) {
            panic!("Contract already initialized");
        }

        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_paused(&env, false);

        env.events().publish(
            (Symbol::new(&env, "Initialized"),),
            (admin, lending_pool),
        );
    }

    pub fn deposite_collateral(
        env: Env,
        sender: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        sender.require_auth();

        let token_client = token::Client::new(&env, &asset);

        token_client.transfer(&sender, &env.current_contract_address(), &amount);
        Ok(())
    }

    pub fn withdraw(_env: Env, _reciver: Address, _asset: Address, _amount: i128) {}

    pub fn seize_collateral(_env: Env, _user: Address, _asset: Address, _amount: i128) {}
}
