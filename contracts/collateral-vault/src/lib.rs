#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

use errors::VaultError;
use types::Position;

mod risk;

#[soroban_sdk::contractclient(name = "OracleClient")]
pub trait Oracle {
    fn get_price(env: Env, asset: Address) -> Option<types::PriceData>;
    fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData;
}

#[soroban_sdk::contractclient(name = "LendingPoolClient")]
pub trait LendingPool {
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn is_liquidatable(user: &Address) -> bool;
}

/// Maximum age (in seconds) an oracle price may have before it is considered stale.

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn initialize(env: Env, admin: Address, lending_pool: Address) {
        // Strict initialization guard: panic if already initialized
        if storage::has_admin(&env) {
            panic!("already initialized");
        }

        admin.require_auth();

        // Commit admin and configured contract addresses to persistent storage
        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &lending_pool);

        // Explicitly set Paused to false
        storage::set_paused(&env, false);

        // Emit structured contract event
        events::Initialized {
            admin,
            lending_pool,
        }
        .publish(&env);
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        admin::set_admin(env, new_admin)
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) {
        admin::set_lending_pool(env, lending_pool)
    }

    pub fn set_oracle(env: Env, oracle: Address) {
        admin::set_oracle(env, oracle)
    }

    pub fn set_liquidation_engine(env: Env, engine: Address) {
        admin::set_liquidation_engine(env, engine)
    }

    pub fn set_pool(env: Env, pool: Address) {
        admin::set_pool(env, pool)
    }

    pub fn set_asset_config(
        env: Env,
        asset: Address,
        token_decimals: u32,
        oracle_price_decimals: u32,
    ) -> Result<(), VaultError> {
        assets::set_asset_config(env, asset, token_decimals, oracle_price_decimals)
    }

    pub fn pause(env: Env) {
        admin::pause(env)
    }

    pub fn unpause(env: Env) {
        admin::unpause(env)
    }

    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        admin::upgrade(env, new_wasm_hash)
    }

    pub fn add_supported_asset(env: Env, asset: Address) {
        assets::add_supported_asset(env, asset)
    }

    pub fn remove_supported_asset(env: Env, asset: Address) {
        assets::remove_supported_asset(env, asset)
    }

    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        assets::is_supported_asset(env, asset)
    }

    pub fn authorize_liquidation(env: Env, liquidation_engine: Address, user: Address) -> bool {
        let stored_engine =
            storage::get_liquidation_engine(&env).expect("Liquidation engine not set");
        if liquidation_engine != stored_engine {
            soroban_sdk::panic_with_error!(&env, VaultError::Unauthorized);
        }

        liquidation_engine.require_auth();

        let position = storage::get_position(&env, &user);
        if position.is_none() {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        let pool_address = storage::get_pool(&env).expect("Lending pool not set");
        let pool_client = LendingPoolClient::new(&env, &pool_address);
        pool_client.is_liquidatable(&user)
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

    pub fn withdraw(env: Env, user: Address, asset: Address, amount: i128) {
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

        if storage::get_position(&env, &user).is_none() {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        let balance = storage::get_position_balance(&env, &user, &asset);
        if amount > balance {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs);
        }

        // Safety check: collateral ratio
        if !Self::is_withdrawal_safe(env.clone(), user.clone(), asset.clone(), amount) {
            soroban_sdk::panic_with_error!(&env, VaultError::BelowMinCollateralRatio);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        // If this asset balance reached zero, remove asset from user's assets list
        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        // If the user has no remaining balance across any asset, remove from index
        if storage::get_position(&env, &user).is_none() {
            storage::remove_from_position_index(&env, &user);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        events::Withdrawn {
            user,
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

        // If this asset balance reached zero, remove asset from user's assets list
        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

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
        let price_data = oracle_client.get_price(&asset).expect("price not found");

        let config = risk::validate_and_load_asset_config(&env, &asset);
        let withdrawn_value = risk::collateral_value(
            amount,
            price_data.price,
            config.token_decimals,
            config.oracle_price_decimals,
        );

        if total_value < withdrawn_value {
            return false;
        }

        let remaining_value = total_value - withdrawn_value;

        // Minimum collateral ratio: 110% (1.1)
        let required_collateral = risk::required_collateral_for_debt(debt, 11_000);
        risk::compare_collateral_with_debt(remaining_value, required_collateral)
    }

    pub fn get_position(env: Env, user: Address) -> Position {
        match storage::get_position(&env, &user) {
            Some(position) => position,
            None => soroban_sdk::panic_with_error!(&env, VaultError::NoPosition),
        }
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        risk::normalized_collateral_value(&env, &user)
    }
}

mod admin;
mod assets;
mod errors;
mod events;
mod storage;
#[cfg(test)]
mod tests;
mod types;
