#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env};

use errors::VaultError;

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, _oracle: Address) {
        if storage::get_admin(&env).is_some() {
            panic!("already initialized");
        }
        storage::set_admin(&env, &admin);
        storage::set_paused(&env, false);
    }

    pub fn set_paused(env: Env, paused: bool) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();
        storage::set_paused(&env, paused);
    }

    pub fn add_supported_asset(env: Env, asset: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();
        storage::add_supported_asset(&env, &asset);
    }

    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        storage::is_supported_asset(&env, &asset)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    pub fn get_position_balance(env: Env, user: Address, asset: Address) -> i128 {
        storage::get_position_balance(&env, &user, &asset)
    }

    pub fn get_position_index(env: Env) -> soroban_sdk::Vec<Address> {
        storage::get_position_index(&env)
    }

    pub fn deposit(env: Env, user: Address, asset: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        if storage::is_paused(&env) {
            panic!("vault is paused");
        }

        if !storage::is_supported_asset(&env, &asset) {
            panic!("asset is not supported");
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let balance = storage::get_position_balance(&env, &user, &asset);
        let new_balance = balance + amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        storage::add_to_position_index(&env, &user);

        env.events().publish(
            (),
            events::Deposited {
                user,
                asset,
                amount,
            },
        );
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

