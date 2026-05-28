#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env};

use errors::VaultError;
use storage::{get_admin, is_paused, set_paused};

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(_env: Env, _admin: Address, _oracle: Address) {}

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

    pub fn unpause(env: Env) {
        let admin = get_admin(&env);
        admin.require_auth();

        let paused = is_paused(&env);
        if !paused {
            panic!("Contract is not paused");
        }

        set_paused(&env, false);

        env.events().publish(("CollateralVault", "Unpaused"), admin);
    }
}

mod errors;
mod events;
mod storage;
mod test;
mod types;
