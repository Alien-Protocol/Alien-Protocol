#![no_std]
use soroban_sdk::{contract, contractclient, contractimpl, token, Address, Env};

use errors::VaultError;
use events::Withdrawn;
use storage::{
    get_lending_pool, get_position, is_paused, remove_position, set_lending_pool, set_position,
    update_position_index,
};

#[allow(dead_code)]
#[contractclient(name = "LendingPoolClient")]
trait LendingPool {
    fn check_withdrawal_safe(user: &Address, amount: &i128) -> bool;
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();
        if storage::get_admin(&env).is_some() {
            panic!("already initialized");
        }
        storage::set_admin(&env, &admin);
        storage::set_paused(&env, false);
        storage::set_oracle(&env, &oracle);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        storage::set_admin(&env, &new_admin);

        events::AdminChanged {
            old_admin: admin,
            new_admin,
        }
        .publish(&env);
    }

    pub fn pause(env: Env) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        if storage::is_paused(&env) {
            panic!("already paused");
        }

        storage::set_paused(&env, true);

        events::Paused { paused: true }.publish(&env);
    }

    pub fn unpause(env: Env) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        if !storage::is_paused(&env) {
            panic!("vault is not paused");
        }

        storage::set_paused(&env, false);

        events::Unpaused { paused: false }.publish(&env);
    }

    pub fn add_supported_asset(env: Env, asset: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();
        storage::add_supported_asset(&env, &asset);
    }

    pub fn remove_supported_asset(env: Env, asset: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        if !storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::AssetNotFound);
        }

        storage::remove_supported_asset(&env, &asset);

        events::AssetRemoved { asset }.publish(&env);
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
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs);
        }

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, VaultError::VaultPaused);
        }

        if !storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::UnsupportedAsset);
        }

        let token_client = token::Client::new(&env, &asset);

        token_client.transfer(&sender, env.current_contract_address(), &amount);
        Ok(())
    }

    pub fn set_lending_pool(env: Env, admin: Address, lending_pool: Address) {
        admin.require_auth();
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
