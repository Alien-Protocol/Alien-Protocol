#![no_std]
use soroban_sdk::{contract, contractclient, contractimpl, token, Address, Env};

use errors::VaultError;
use events::{Deposited, Withdrawn};
use storage::{
    get_admin, get_lending_pool, get_position, is_paused, remove_position, set_admin,
    set_lending_pool, set_position, update_position_index,
};
use types::Position;

#[allow(dead_code)]
#[contractclient(name = "LendingPoolClient")]
trait LendingPool {
    fn check_withdrawal_safe(user: &Address, asset: &Address, amount: &i128) -> bool;
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, _oracle: Address) {
        set_admin(&env, &admin);
    }

    pub fn deposite_collateral(
        env: Env,
        sender: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        sender.require_auth();

        if amount <= 0 {
            return Err(VaultError::InvalidInputs);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&sender, env.current_contract_address(), &amount);

        let mut position = match get_position(&env, &sender, &asset) {
            Ok(p) => p,
            Err(VaultError::NoPosition) => Position { amount: 0 },
            Err(err) => return Err(err),
        };

        position.amount += amount;
        set_position(&env, &sender, &asset, &position);
        update_position_index(&env, &sender, &asset, position.amount);

        Deposited {
            user: sender.clone(),
            asset: asset.clone(),
            amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) {
        let stored_admin = get_admin(&env).expect("Admin not set");
        stored_admin.require_auth();
        set_lending_pool(&env, &lending_pool);
    }

    pub fn withdraw(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        user.require_auth();

        if amount <= 0 {
            return Err(VaultError::InvalidInputs);
        }

        if is_paused(&env) {
            return Err(VaultError::VaultPaused);
        }

        let mut position = get_position(&env, &user, &asset)?;

        if position.amount < amount {
            return Err(VaultError::InsufficientBalance);
        }

        let lending_pool_address = get_lending_pool(&env).ok_or(VaultError::InvalidInputs)?;

        // Cross-call to LendingPool to verify withdrawal keeps collateral ratio safe
        let lending_pool_client = LendingPoolClient::new(&env, &lending_pool_address);
        let is_safe = lending_pool_client.check_withdrawal_safe(&user, &asset, &amount);
        if !is_safe {
            return Err(VaultError::InsufficientCollateral);
        }

        position.amount -= amount;

        if position.amount == 0 {
            remove_position(&env, &user, &asset);
            update_position_index(&env, &user, &asset, 0);
        } else {
            set_position(&env, &user, &asset, &position);
            update_position_index(&env, &user, &asset, position.amount);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        Withdrawn {
            user: user.clone(),
            asset: asset.clone(),
            amount,
        }
        .publish(&env);

        Ok(())
    }
    pub fn withdraw(env: Env, receiver: Address, asset: Address, amount: i128) {
        receiver.require_auth();

        if amount <= 0 {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs);
        }

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, VaultError::VaultPaused);
        }

        if !storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::UnsupportedAsset);
        }

        let balance = storage::get_position_balance(&env, &receiver, &asset);
        if balance < amount {
            panic!("insufficient balance");
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &receiver, &asset, new_balance);

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &receiver, &amount);
    }

    pub fn seize_collateral(_env: Env, _user: Address, _asset: Address, _amount: i128) {}

    pub fn is_withdrawal_safe(_env: Env, _user: Address, _amount: i128) {}

    pub fn get_position(env: Env, user: Address) -> Position {
        let index = storage::get_position_index(&env);
        let assets = storage::get_user_assets(&env, &user);
        let mut collateral = soroban_sdk::Vec::new(&env);
        let mut has_balance = false;

        for asset in assets.iter() {
            let amount = storage::get_position_balance(&env, &user, &asset);
            if amount > 0 {
                collateral.push_back(CollateralAsset {
                    asset: asset.clone(),
                    amount,
                });
                has_balance = true;
            }
        }

        if !index.contains(&user) || !has_balance {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        Position { user, collateral }
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        let position = Self::get_position(env.clone(), user);

        let oracle_address = storage::get_oracle(&env).expect("oracle not configured");
        let oracle_client = OracleClient::new(&env, &oracle_address);

        let mut total_value: i128 = 0;
        let current_time = env.ledger().timestamp();
        const ORACLE_STALE_THRESHOLD: u64 = 300; // 5 minutes

        for item in position.collateral.iter() {
            let price_opt = oracle_client.get_price(&item.asset);
            let price_data = match price_opt {
                Some(pd) => pd,
                None => panic!("price not found"),
            };

            if current_time > price_data.timestamp
                && current_time - price_data.timestamp > ORACLE_STALE_THRESHOLD
            {
                soroban_sdk::panic_with_error!(&env, VaultError::StalePrice);
            }

            let item_value = item
                .amount
                .checked_mul(price_data.price)
                .unwrap_or_else(|| panic!("overflow in value calculation"));

            total_value = total_value
                .checked_add(item_value)
                .unwrap_or_else(|| panic!("overflow in total value calculation"));
        }

        total_value
    }
}

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod tests;
mod types;
