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
    /// Initializes the Vault contract with admin, lending pool, and oracle addresses.
    pub fn initialize(
        env: Env,
        admin: Address,
        lending_pool: Address,
        oracle: Address,
    ) -> Result<(), VaultError> {
        if storage::has_admin(&env) {
            return Err(VaultError::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &oracle);
        storage::set_paused(&env, false);

        events::Initialized {
            admin,
            lending_pool,
        }
        .publish(&env);

        Ok(())
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        let current_admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
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

    pub fn set_lending_pool(env: Env, lending_pool: Address) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        storage::set_lending_pool(&env, &lending_pool);
        events::LendingPoolUpdated { lending_pool }.publish(&env);

        Ok(())
    }

    pub fn set_oracle(env: Env, oracle: Address) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        storage::set_oracle(&env, &oracle);
        Ok(())
    }

    pub fn pause(env: Env) -> Result<(), VaultError> {
        let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
        admin.require_auth();

        if storage::is_paused(&env) {
            return Err(VaultError::AlreadyPaused);
        }

        storage::set_paused(&env, true);
        events::Paused { by: admin }.publish(&env);

        Ok(())
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

        let pool_address = storage::get_lending_pool(&env).ok_or(VaultError::LendingPoolNotSet)?;
        let pool_client = LendingPoolClient::new(&env, &pool_address);
        Ok(pool_client.is_liquidatable(&user))
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
        if storage::get_position(&env, &user).is_none() {
            return Err(VaultError::NoPosition);
        }

        let balance = storage::get_position_balance(&env, &user, &asset);
        if amount > balance {
            return Err(VaultError::InvalidInputs);
        }

        // Verify withdrawal safety (collateral ratio requirement)
        if !Self::is_withdrawal_safe(env.clone(), user.clone(), asset.clone(), amount)? {
            return Err(VaultError::BelowMinCollateralRatio);
        }

        // 2. EFFECTS
        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        if storage::get_position(&env, &user).is_none() {
            storage::remove_from_position_index(&env, &user);
        }

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
        liquidation_engine.require_auth();

        let registered_engine =
            storage::get_liquidation_engine(&env).ok_or(VaultError::Unauthorized)?;
        if liquidation_engine != registered_engine {
            return Err(VaultError::Unauthorized);
        }

        if storage::is_paused(&env) {
            return Err(VaultError::VaultPaused);
        }

        let index = storage::get_position_index(&env);
        if !index.contains(&user) {
            return Err(VaultError::NoPosition);
        }

        let balance = storage::get_position_balance(&env, &user, &asset);
        if balance < amount {
            return Err(VaultError::InvalidInputs);
        }

        // EFFECTS
        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        if storage::get_position(&env, &user).is_none() {
            storage::remove_from_position_index(&env, &user);
        }

        // INTERACTIONS
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

        Ok(())
    }

    pub fn is_withdrawal_safe(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<bool, VaultError> {
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

mod errors;
mod events;
mod storage;
#[cfg(test)]
mod tests;
mod types;
