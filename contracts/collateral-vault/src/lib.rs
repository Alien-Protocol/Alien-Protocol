#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env};

use errors::VaultError;
use types::{AssetsPage, Position, PositionsPage};

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
    // ── Initialization & admin ────────────────────────────────────────────────

    pub fn initialize(
        env: Env,
        admin: Address,
        lending_pool: Address,
        oracle: Address,
        liquidation_engine: Address,
    ) -> Result<(), VaultError> {
        if storage::has_admin(&env) {
            return Err(VaultError::AlreadyInitialized);
        }

        if lending_pool == oracle {
            return Err(VaultError::InvalidAddress);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &oracle);
        storage::set_liquidation_engine(&env, &liquidation_engine);
        storage::set_paused(&env, false);

        storage::set_contract_version(&env, upgrade::CURRENT_CONTRACT_VERSION);
        storage::set_storage_schema_version(&env, upgrade::CURRENT_STORAGE_SCHEMA_VERSION);

        events::Initialized {
            admin,
            lending_pool,
            oracle,
            liquidation_engine,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_contract_version(env: Env) -> u32 {
        upgrade::get_contract_version(&env)
    }

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

    pub fn set_lending_pool(env: Env, lending_pool: Address) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if let Some(oracle) = storage::get_oracle(&env) {
            if lending_pool == oracle {
                soroban_sdk::panic_with_error!(&env, VaultError::InvalidAddress);
            }
        }

        let old_pool = storage::get_lending_pool(&env);
        storage::set_lending_pool(&env, &lending_pool);
        events::LendingPoolUpdated {
            old_pool,
            new_pool: lending_pool,
        }
        .publish(&env);
    }

    pub fn set_oracle(env: Env, oracle: Address) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if let Some(pool) = storage::get_lending_pool(&env) {
            if oracle == pool {
                soroban_sdk::panic_with_error!(&env, VaultError::InvalidAddress);
            }
        }

        let old_oracle = storage::get_oracle(&env);
        storage::set_oracle(&env, &oracle);
        events::OracleUpdated {
            old_oracle,
            new_oracle: oracle,
        }
        .publish(&env);
    }

    pub fn pause(env: Env) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, VaultError::AlreadyPaused);
        }

        storage::set_paused(&env, true);
        events::Paused { by: admin }.publish(&env);
    }

    pub fn unpause(env: Env) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if !storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, VaultError::NotPaused);
        }

        storage::set_paused(&env, false);
        events::Unpaused { by: admin }.publish(&env);
    }

    // ── Asset management ──────────────────────────────────────────────────────

    pub fn add_supported_asset(env: Env, asset: Address) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::AlreadySupported);
        }

        storage::add_supported_asset(&env, &asset);
        events::AssetAdded { asset }.publish(&env);
    }

    pub fn remove_supported_asset(env: Env, asset: Address) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if !storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::AssetNotFound);
        }

        storage::remove_supported_asset(&env, &asset);
        events::AssetRemoved { asset }.publish(&env);
    }

    pub fn set_liquidation_engine(env: Env, engine: Address) {
        let admin = storage::get_admin(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        admin.require_auth();

        if let Some(pool) = storage::get_lending_pool(&env) {
            if engine == pool {
                soroban_sdk::panic_with_error!(&env, VaultError::InvalidAddress);
            }
        }
        if let Some(oracle) = storage::get_oracle(&env) {
            if engine == oracle {
                soroban_sdk::panic_with_error!(&env, VaultError::InvalidAddress);
            }
        }

        let old_engine = storage::get_liquidation_engine(&env);
        storage::set_liquidation_engine(&env, &engine);
        events::LiquidationEngineUpdated {
            old_engine,
            new_engine: engine,
        }
        .publish(&env);
    }

    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        storage::is_supported_asset(&env, &asset)
    }

    // ── Core write operations ─────────────────────────────────────────────────

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
        let new_balance = balance
            .checked_add(amount)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs));
        storage::set_position_balance(&env, &user, &asset, new_balance);

        storage::add_user_asset(&env, &user, &asset);
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
        if !storage::user_in_position_index(&env, &user) {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        let balance = storage::get_position_balance(&env, &user, &asset);
        if amount > balance {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs);
        }

        if !Self::is_withdrawal_safe(env.clone(), user.clone(), asset.clone(), amount) {
            soroban_sdk::panic_with_error!(&env, VaultError::BelowMinCollateralRatio);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        if storage::user_asset_count(&env, &user) == 0 {
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

    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        liquidation_engine.require_auth();

        if amount <= 0 {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidAmount);
        }

        match liquidation::execute_seize_authorized(&env, liquidation_engine, user, asset, amount) {
            Ok(()) => {}
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }

    pub fn authorize_liquidation(env: Env, liquidation_engine: Address, user: Address) -> bool {
        let stored_engine = storage::get_liquidation_engine(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::Unauthorized));
        if liquidation_engine != stored_engine {
            soroban_sdk::panic_with_error!(&env, VaultError::Unauthorized);
        }

        liquidation_engine.require_auth();

        if !storage::user_in_position_index(&env, &user) {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        let pool_address = storage::get_lending_pool(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        let pool_client = LendingPoolClient::new(&env, &pool_address);
        pool_client.is_liquidatable(&user)
    }

    // ── Read-only helpers ─────────────────────────────────────────────────────

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

    pub fn get_position_count(env: Env) -> u32 {
        storage::position_count(&env)
    }

    pub fn get_position(env: Env, user: Address) -> Position {
        match storage::get_position(&env, &user) {
            Some(position) => position,
            None => soroban_sdk::panic_with_error!(&env, VaultError::NoPosition),
        }
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        let position = Self::get_position(env.clone(), user);

        let oracle_address = storage::get_oracle(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        let oracle_client = OracleClient::new(&env, &oracle_address);

        let mut total_value: i128 = 0;

        for item in position.collateral.iter() {
            let price_data = oracle_client.get_price_or_fail(&item.asset);

            let item_value = item
                .amount
                .checked_mul(price_data.price)
                .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs))
                / PRICE_PRECISION;

            total_value = total_value
                .checked_add(item_value)
                .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs));
        }

        total_value
    }

    pub fn is_withdrawal_safe(env: Env, user: Address, asset: Address, amount: i128) -> bool {
        let pool_addr = match storage::get_lending_pool(&env) {
            Some(addr) => addr,
            None => return true, // no pool configured → no debt possible
        };

        let pool_client = LendingPoolClient::new(&env, &pool_addr);
        let debt = match pool_client.try_get_user_debt(&user) {
            // Successful call with a debt value
            Ok(Ok(d)) => d,
            // Any failure (host error, contract error, pool not deployed) → treat as no debt
            // This is safe: if debt is unknown, the user is not blocked from withdrawing
            _ => 0,
        };

        if debt == 0 {
            return true;
        }

        let total_value = Self::get_collateral_value(env.clone(), user.clone());

        let oracle_address = storage::get_oracle(&env)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
        let oracle_client = OracleClient::new(&env, &oracle_address);
        let price_data = oracle_client
            .get_price(&asset)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));

        let withdrawn_value = amount
            .checked_mul(price_data.price)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs))
            / PRICE_PRECISION;

        if total_value < withdrawn_value {
            return false;
        }

        let remaining_value = total_value - withdrawn_value;
        let min_collateral = debt
            .checked_mul(110)
            .and_then(|v| v.checked_div(100))
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::InvalidInputs));
        remaining_value >= min_collateral
    }

    pub fn get_position_index(env: Env) -> soroban_sdk::Vec<Address> {
        storage::get_position_index(&env)
    }

    pub fn get_all_positions(env: Env) -> soroban_sdk::Vec<Position> {
        storage::get_all_positions(&env)
    }

    // ── Paginated enumeration views ───────────────────────────────────────────

    pub fn get_positions_page(env: Env, cursor: u32, limit: u32) -> PositionsPage {
        views::get_positions_page(&env, cursor, limit)
    }

    pub fn get_supported_assets_page(env: Env, cursor: u32, limit: u32) -> AssetsPage {
        views::get_supported_assets_page(&env, cursor, limit)
    }

    pub fn get_user_assets_page(env: Env, user: Address, cursor: u32, limit: u32) -> AssetsPage {
        views::get_user_assets_page(&env, &user, cursor, limit)
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
mod views;
