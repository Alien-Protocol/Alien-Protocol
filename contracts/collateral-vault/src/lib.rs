#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

use errors::VaultError;
use types::Position;

#[soroban_sdk::contractclient(name = "OracleClient")]
pub trait Oracle {
    fn get_price(env: Env, asset: Address) -> Option<types::PriceData>;
    fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData;
}

#[soroban_sdk::contractclient(name = "LendingPoolClient")]
pub trait LendingPool {
    fn get_user_debt(env: Env, user: Address) -> i128;
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

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        let current_admin = storage::get_admin(&env).ok_or(VaultError::InvalidInputs)?;
        current_admin.require_auth();

        if current_admin == new_admin {
            return Err(VaultError::AlreadyAdmin);
        }

        storage::set_admin(&env, &new_admin);

        events::AdminChanged {
            old_admin: current_admin,
            new_admin,
        }
        .publish(&env);

        Ok(())
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        storage::set_lending_pool(&env, &lending_pool);

        events::LendingPoolUpdated { lending_pool }.publish(&env);
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

        if storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::AlreadySupported);
        }

        storage::add_supported_asset(&env, &asset);

        events::AssetAdded { asset }.publish(&env);
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

    pub fn authorize_liquidation(env: Env, engine: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        storage::set_liquidation_engine(&env, &engine);

        events::LiquidationEngineSet { engine }.publish(&env);
    }

    pub fn set_pool(env: Env, pool: Address) {
        let admin = storage::get_admin(&env).expect("not initialized");
        admin.require_auth();

        storage::set_pool(&env, &pool);
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

    pub fn get_position_index(env: Env) -> Vec<Address> {
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
        token_client.transfer(&user, env.current_contract_address(), &amount);

        let balance = storage::get_position_balance(&env, &user, &asset);
        let new_balance = balance + amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        // Track this asset for the user (used to build Position)
        storage::add_user_asset(&env, &user, &asset);
        // Add user to the global position index if not already present
        storage::add_to_position_index(&env, &user);

        events::Deposited {
            user,
            asset,
            amount,
        }
        .publish(&env);
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
        if amount > balance {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs);
        }

        // Safety check: collateral ratio
        if !Self::is_withdrawal_safe(env.clone(), receiver.clone(), asset.clone(), amount) {
            soroban_sdk::panic_with_error!(&env, VaultError::BelowMinCollateralRatio);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &receiver, &asset, new_balance);

        // If the user has no remaining balance across any asset, remove from index
        if storage::get_position(&env, &receiver).is_none() {
            storage::remove_from_position_index(&env, &receiver);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &receiver, &amount);

        events::Withdrawn {
            receiver,
            asset,
            amount,
        }
        .publish(&env);
    }

    pub fn get_all_positions(env: Env) -> Vec<Position> {
        storage::get_all_positions(&env)
    }

    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        liquidation_engine.require_auth();

        let registered_engine =
            storage::get_liquidation_engine(&env).expect("liquidation engine not authorized");
        if liquidation_engine != registered_engine {
            soroban_sdk::panic_with_error!(&env, VaultError::Unauthorized);
        }

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, VaultError::VaultPaused);
        }

        // Verify user has an active position
        let index = storage::get_position_index(&env);
        if !index.contains(&user) {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        let balance = storage::get_position_balance(&env, &user, &asset);
        if balance < amount {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        // If the user has no remaining balance across any asset, remove from index
        if storage::get_position(&env, &user).is_none() {
            storage::remove_from_position_index(&env, &user);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(
            &env.current_contract_address(),
            &liquidation_engine,
            &amount,
        );

        events::CollateralSeized {
            user,
            asset,
            amount,
            liquidation_engine,
        }
        .publish(&env);
    }

    pub fn is_withdrawal_safe(env: Env, user: Address, asset: Address, amount: i128) -> bool {
        let debt = if let Some(pool_addr) = storage::get_pool(&env) {
            let pool_client = LendingPoolClient::new(&env, &pool_addr);
            pool_client.get_user_debt(&user)
        } else {
            0
        };

        if debt == 0 {
            return true;
        }

        let total_value = Self::get_collateral_value(env.clone(), user.clone());

        let oracle_address = storage::get_oracle(&env).expect("oracle not configured");
        let oracle_client = OracleClient::new(&env, &oracle_address);
        let price_data = oracle_client.get_price_or_fail(&asset);

        let withdrawn_value = amount
            .checked_mul(price_data.price)
            .unwrap_or_else(|| panic!("overflow in withdrawn value calculation"));

        if total_value < withdrawn_value {
            return false;
        }

        let remaining_value = total_value - withdrawn_value;

        // Minimum collateral ratio: 110% (1.1)
        remaining_value >= (debt * 110) / 100
    }

    pub fn get_position(env: Env, user: Address) -> Position {
        match storage::get_position(&env, &user) {
            Some(position) => position,
            None => soroban_sdk::panic_with_error!(&env, VaultError::NoPosition),
        }
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        let position = Self::get_position(env.clone(), user);

        let oracle_address = storage::get_oracle(&env).expect("oracle not configured");
        let oracle_client = OracleClient::new(&env, &oracle_address);

        let mut total_value: i128 = 0;

        for item in position.collateral.iter() {
            let price_data = oracle_client.get_price_or_fail(&item.asset);

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
