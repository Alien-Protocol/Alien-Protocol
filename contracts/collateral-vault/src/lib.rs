#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env};

use errors::VaultError;
use events::LendingPoolUpdated;
use storage::{get_admin, set_lending_pool};

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, _oracle: Address) {
        let storage_key = types::Datakey::Admin;
        env.storage().instance().set(&storage_key, &admin);
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) -> Result<(), VaultError> {
        // Require auth from admin
        let admin = get_admin(&env);
        admin.require_auth();

        // Validate lending_pool is not the zero address (basic validation)
        // In a real scenario, you'd also verify it's a valid contract
        
        // Write new address to LendingPool storage key
        set_lending_pool(&env, &lending_pool);

        // Emit LendingPoolUpdated event
        LendingPoolUpdated {
            lending_pool,
        }
        .publish(&env);

        Ok(())
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

    pub fn is_withdrawal_safe(_env: &Env, _user: Address, _amount: i128) {}

    pub fn get_position(_env: &Env, _user: Address) {}

    pub fn get_collateral_value(_env: &Env, _user: Address) {}
}

mod errors;
mod events;
mod storage;
mod test;
mod types;
