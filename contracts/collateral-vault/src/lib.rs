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
    fn is_liquidatable(user: &Address) -> bool;
}

/// Oracle prices are encoded with 7 decimal places (e.g. $1.00 = 10_000_000).
/// Dividing `amount * price` by this constant yields the USD-denominated value.
const PRICE_PRECISION: i128 = 10_000_000;

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    /// Atomically initialize the vault with all required external dependencies.
    ///
    /// Accepts distinct addresses for the admin, lending pool, oracle adapter,
    /// and liquidation engine. Rejects duplicate initialization and prevents
    /// the lending-pool address from being stored as the oracle accidentally.
    pub fn initialize(
        env: Env,
        admin: Address,
        lending_pool: Address,
        oracle: Address,
        liquidation_engine: Address,
    ) -> Result<(), VaultError> {
        // Strict initialization guard: reject if already initialized
        if storage::has_admin(&env) {
            return Err(VaultError::AlreadyInitialized);
        }

        // Prevent accidental role-address collision
        if lending_pool == oracle {
            return Err(VaultError::InvalidAddress);
        }

        admin.require_auth();

        // Commit admin and all configured contract addresses to persistent storage
        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &oracle);
        storage::set_liquidation_engine(&env, &liquidation_engine);

        // Explicitly set Paused to false
        storage::set_paused(&env, false);

        events::Initialized {
            admin,
            lending_pool,
            oracle,
            liquidation_engine,
        }
        .publish(&env);

        Ok(())
    }

        Ok(())
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        let current_admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        current_admin.require_auth();

    pub fn get_storage_schema_version(env: Env) -> u32 {
        upgrade::get_storage_schema_version(&env)
    }

    pub fn upgrade(env: Env, wasm_hash: BytesN<32>) -> Result<(), VaultError> {
        upgrade::upgrade(env, wasm_hash)
    }

    pub fn migrate(env: Env, target_storage_schema_version: u32) -> Result<(), VaultError> {
        upgrade::migrate(env, target_storage_schema_version)
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        admin::set_admin(env, new_admin)
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        storage::set_lending_pool(&env, &lending_pool);
        events::LendingPoolUpdated { lending_pool }.publish(&env);

        Ok(())
    }

    pub fn pause(env: Env) {
        admin::pause(env)
    }

    pub fn unpause(env: Env) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        if !storage::is_paused(&env) {
            return Err(VaultError::NotPaused);
        }

        storage::set_paused(&env, false);
        events::Unpaused { by: admin }.publish(&env);

        Ok(())
    }

    pub fn add_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        if storage::is_supported_asset(&env, &asset) {
            return Err(VaultError::AlreadySupported);
        }

        storage::add_supported_asset(&env, &asset);
        events::AssetAdded { asset }.publish(&env);

        Ok(())
    }

    pub fn remove_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        if !storage::is_supported_asset(&env, &asset) {
            return Err(VaultError::AssetNotFound);
        }

        storage::remove_supported_asset(&env, &asset);
        events::AssetRemoved { asset }.publish(&env);

        Ok(())
    }

    pub fn set_liquidation_engine(env: Env, engine: Address) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        storage::set_liquidation_engine(&env, &engine);
        events::LiquidationEngineSet { engine }.publish(&env);

        Ok(())
    }

    pub fn authorize_liquidation(
        env: Env,
        liquidation_engine: Address,
        user: Address,
    ) -> Result<bool, VaultError> {
        let stored_engine =
            storage::get_liquidation_engine(&env).ok_or(VaultError::Unauthorized)?;
        if liquidation_engine != stored_engine {
            return Err(VaultError::Unauthorized);
        }

        liquidation_engine.require_auth();

        if storage::get_position(&env, &user).is_none() {
            return Err(VaultError::NoPosition);
        }

        let pool_address = storage::get_lending_pool(&env).expect("Lending pool not set");
        let pool_client = LendingPoolClient::new(&env, &pool_address);
        pool_client.is_liquidatable(&user)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    pub fn get_lending_pool(env: Env) -> Option<Address> {
        storage::get_lending_pool(&env)
    }

    pub fn get_oracle(env: Env) -> Option<Address> {
        storage::get_oracle(&env)
    }

    pub fn get_liquidation_engine(env: Env) -> Option<Address> {
        storage::get_liquidation_engine(&env)
    }

    pub fn get_position_balance(env: Env, user: Address, asset: Address) -> i128 {
        storage::get_position_balance(&env, &user, &asset)
    }

    pub fn get_position_index(env: Env) -> Vec<Address> {
        storage::get_position_index(&env)
    }

    /// Deposit collateral following the Checks-Effects-Interactions pattern.
    pub fn deposit(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        user.require_auth();

        // 1. CHECKS
        if amount <= 0 {
            return Err(VaultError::InvalidInputs);
        }
        if storage::is_paused(&env) {
            return Err(VaultError::VaultPaused);
        }
        if !storage::is_supported_asset(&env, &asset) {
            return Err(VaultError::UnsupportedAsset);
        }

        // 2. EFFECTS (Update internal ledger before external call)
        let balance = storage::get_position_balance(&env, &user, &asset);
        let new_balance = balance
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;

        storage::set_position_balance(&env, &user, &asset, new_balance);
        storage::add_user_asset(&env, &user, &asset);
        storage::add_to_position_index(&env, &user);

        // 3. INTERACTIONS (External token transfer)
        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&user, env.current_contract_address(), &amount);

        events::Deposited {
            user,
            asset,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Withdraw collateral following Checks-Effects-Interactions.
    pub fn withdraw(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        user.require_auth();

        position::validate_positive_amount(amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

        if storage::is_paused(&env) {
            return Err(VaultError::VaultPaused);
        }
        if !storage::is_supported_asset(&env, &asset) {
            return Err(VaultError::UnsupportedAsset);
        }
        if storage::get_position(&env, &user).is_none() {
            return Err(VaultError::NoPosition);
        }

        // Safety check: collateral ratio (must happen before the debit)
        if !Self::is_withdrawal_safe(env.clone(), user.clone(), asset.clone(), amount) {
            soroban_sdk::panic_with_error!(&env, VaultError::BelowMinCollateralRatio);
        }

        position::checked_debit(&env, &user, &asset, amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

        // 3. INTERACTIONS
        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &user, &amount);

        events::Withdrawn {
            user,
            asset,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        liquidation::execute_seize(&env, liquidation_engine, user, asset, amount)
    }

    pub fn is_withdrawal_safe(env: Env, user: Address, asset: Address, amount: i128) -> bool {
        let debt = if let Some(pool_addr) = storage::get_lending_pool(&env) {
            let pool_client = LendingPoolClient::new(&env, &pool_addr);
            pool_client.get_user_debt(&user)
        } else {
            0
        };

        if debt == 0 {
            return Ok(true);
        }

        let total_value = Self::get_collateral_value(env.clone(), user.clone())?;

        let oracle_address = storage::get_oracle(&env).ok_or(VaultError::OracleNotConfigured)?;
        let oracle_client = OracleClient::new(&env, &oracle_address);
        let price_data = oracle_client
            .get_price(&asset)
            .ok_or(VaultError::PriceNotFound)?;

        let withdrawn_value = amount
            .checked_mul(price_data.price)
            .ok_or(VaultError::MathOverflow)?
            / PRICE_PRECISION;

        if total_value < withdrawn_value {
            return Ok(false);
        }

        let remaining_value = total_value - withdrawn_value;

        // Minimum collateral ratio requirement: 110%
        let required_collateral = debt.checked_mul(110).ok_or(VaultError::MathOverflow)? / 100;

        Ok(remaining_value >= required_collateral)
    }

    pub fn get_position(env: Env, user: Address) -> Result<Position, VaultError> {
        storage::get_position(&env, &user).ok_or(VaultError::NoPosition)
    }

    pub fn get_collateral_value(env: Env, user: Address) -> Result<i128, VaultError> {
        let position = Self::get_position(env.clone(), user)?;

        let oracle_address = storage::get_oracle(&env).ok_or(VaultError::OracleNotConfigured)?;
        let oracle_client = OracleClient::new(&env, &oracle_address);

        let mut total_value: i128 = 0;

        for item in position.collateral.iter() {
            let price_data = oracle_client.get_price_or_fail(&item.asset);

            let item_value = item
                .amount
                .checked_mul(price_data.price)
                .ok_or(VaultError::MathOverflow)?
                / PRICE_PRECISION;

            total_value = total_value
                .checked_add(item_value)
                .ok_or(VaultError::MathOverflow)?;
        }

        Ok(total_value)
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

    pub fn get_all_positions(env: Env) -> Vec<Position> {
        storage::get_all_positions(&env)
    }
}

mod admin;
mod assets;
mod errors;
mod events;
mod liquidation;
mod position;
mod storage;
#[cfg(test)]
mod tests;
mod types;
mod upgrade;
