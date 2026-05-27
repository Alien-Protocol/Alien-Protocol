#![no_std]
use soroban_sdk::{contract, contractimpl, contractclient, token, Address, Env};

use errors::VaultError;
use events::Withdrawn;
use storage::{get_position, set_position, remove_position, update_position_index, get_lending_pool, set_lending_pool, is_paused};

#[allow(dead_code)]
#[contractclient(name = "LendingPoolClient")]
trait LendingPool {
    fn check_withdrawal_safe(user: &Address, amount: &i128) -> bool;
}

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

    pub fn set_lending_pool(env: Env, admin: Address, lending_pool: Address) {
        admin.require_auth();
        set_lending_pool(&env, &lending_pool);
    }

    pub fn withdraw(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), VaultError> {
        user.require_auth();

        if amount <= 0 {
            return Err(VaultError::InvalidInputs);
        }

        if is_paused(&env) {
            return Err(VaultError::VaultPaused);
        }

        let mut position = get_position(&env, &user)?;

        if position.amount < amount {
            return Err(VaultError::InsufficientBalance);
        }

        let lending_pool_address = get_lending_pool(&env).ok_or(VaultError::InvalidInputs)?;
        
        // Cross-call to LendingPool to verify withdrawal keeps collateral ratio safe
        let lending_pool_client = LendingPoolClient::new(&env, &lending_pool_address);
        let is_safe = lending_pool_client.check_withdrawal_safe(&user, &amount);
        if !is_safe {
            return Err(VaultError::InsufficientCollateral);
        }

        position.amount -= amount;

        if position.amount == 0 {
            remove_position(&env, &user);
            update_position_index(&env, &user, 0);
        } else {
            set_position(&env, &user, &position);
            update_position_index(&env, &user, position.amount);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        Withdrawn {
            user: user.clone(),
            asset: asset.clone(),
            amount,
        }.publish(&env);

        Ok(())
    }

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
