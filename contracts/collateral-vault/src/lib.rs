#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Symbol, Vec};

use errors::VaultError;
pub use types::PauseFlag;
use types::Position;

mod risk;

#[soroban_sdk::contractclient(name = "OracleClient")]
pub trait Oracle {
    fn get_price(env: Env, asset: Address) -> Option<types::PriceData>;
    fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData;
}

#[soroban_sdk::contractclient(name = "LendingPoolClient")]
pub trait LendingPool {
    fn get_total_borrowed(env: Env) -> i128;
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn is_liquidatable(env: Env, user: Address) -> bool;
}

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    pub fn __constructor(env: Env, admin: Address) {
        shared::bind_initializer(&env, &admin);
    }

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

        if admin == lending_pool
            || admin == oracle
            || admin == liquidation_engine
            || lending_pool == oracle
            || lending_pool == liquidation_engine
            || oracle == liquidation_engine
        {
            return Err(VaultError::InvalidAddress);
        }

        admin.require_auth();
        if !shared::take_initializer(&env, &admin) {
            return Err(VaultError::Unauthorized);
        }

        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &oracle);
        storage::set_liquidation_engine(&env, &liquidation_engine);
        storage::set_pause_mask(&env, 0);
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

    pub fn get_contract_version(_env: Env) -> u32 {
        upgrade::get_contract_version()
    }

    pub fn get_storage_schema_version(env: Env) -> u32 {
        upgrade::get_storage_schema_version(&env)
    }

    pub fn upgrade(
        env: Env,
        wasm_hash: BytesN<32>,
        expected_version: u32,
    ) -> Result<(), VaultError> {
        upgrade::upgrade(env, wasm_hash, expected_version)
    }
    pub fn finalize_upgrade(env: Env) -> Result<(), VaultError> {
        upgrade::finalize_upgrade(env)
    }

    pub fn migrate(env: Env, target_storage_schema_version: u32) -> Result<(), VaultError> {
        upgrade::migrate(env, target_storage_schema_version)
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        admin::set_admin(env, new_admin)
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) -> Result<(), VaultError> {
        admin::set_lending_pool(env, lending_pool)
    }

    pub fn set_oracle(env: Env, oracle: Address) -> Result<(), VaultError> {
        admin::set_oracle(env, oracle)
    }

    pub fn set_liquidation_engine(env: Env, engine: Address) -> Result<(), VaultError> {
        admin::set_liquidation_engine(env, engine)
    }

    pub fn pause_operation(
        env: Env,
        operation: PauseFlag,
        reason: Symbol,
    ) -> Result<(), VaultError> {
        admin::pause_operation(env, operation, reason)
    }

    pub fn unpause_operation(env: Env, operation: PauseFlag) -> Result<(), VaultError> {
        admin::unpause_operation(env, operation)
    }

    /// Returns `true` if `operation` is currently paused on this vault.
    ///
    /// Read-only and callable by anyone; the liquidation engine uses it to
    /// refuse liquidations while `PauseFlag::Liquidation` is set.
    pub fn is_operation_paused(env: Env, operation: PauseFlag) -> bool {
        storage::is_operation_paused(&env, &operation)
    }

    pub fn add_supported_asset(
        env: Env,
        asset: Address,
        config: types::AssetConfig,
    ) -> Result<(), VaultError> {
        assets::add_supported_asset(env, asset, config)
    }

    pub fn remove_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
        assets::remove_supported_asset(env, asset)
    }

    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        assets::is_supported_asset(env, asset)
    }

    pub fn set_asset_config(
        env: Env,
        asset: Address,
        token_decimals: u32,
        oracle_price_decimals: u32,
        max_ltv_bps: u32,
        liquidation_threshold_bps: u32,
    ) -> Result<(), VaultError> {
        assets::set_asset_config(
            env,
            asset,
            token_decimals,
            oracle_price_decimals,
            max_ltv_bps,
            liquidation_threshold_bps,
        )
    }

    pub fn get_asset_config(env: Env, asset: Address) -> Result<types::AssetConfig, VaultError> {
        assets::get_asset_config(env, asset)
    }

    pub fn get_health_factor(env: Env, user: Address) -> i128 {
        let debt = if let Some(pool_addr) = storage::get_lending_pool(&env) {
            let pool_client = LendingPoolClient::new(&env, &pool_addr);
            pool_client.get_user_debt(&user)
        } else {
            0
        };
        Self::health_factor_for_debt(env, user, debt)
    }

    /// Quotes health for a supplied debt without calling back into the pool.
    pub fn health_factor_for_debt(env: Env, user: Address, debt: i128) -> i128 {
        match risk::health_factor_bps(&env, &user, debt) {
            Ok(hf) => hf,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }

    pub fn can_originate(env: Env, pool: Address) -> bool {
        storage::get_lending_pool(&env) == Some(pool)
            && !storage::is_operation_paused(&env, &PauseFlag::Borrow)
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

    pub fn deposit(env: Env, user: Address, asset: Address, amount: i128) {
        user.require_auth();

        position::validate_positive_amount(amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

        if storage::is_operation_paused(&env, &PauseFlag::Deposit) {
            soroban_sdk::panic_with_error!(&env, VaultError::VaultPaused);
        }

        if !storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::UnsupportedAsset);
        }

        // Checks and state updates (effects via checked_credit) before token pull
        position::checked_credit(&env, &user, &asset, amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&user, env.current_contract_address(), &amount);

        events::Deposited {
            user,
            asset,
            amount,
        }
        .publish(&env);
    }

    pub fn withdraw(env: Env, user: Address, asset: Address, amount: i128) {
        user.require_auth();

        position::validate_positive_amount(amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

        if storage::is_operation_paused(&env, &PauseFlag::Withdraw) {
            soroban_sdk::panic_with_error!(&env, VaultError::VaultPaused);
        }

        if !storage::is_supported_asset(&env, &asset) {
            soroban_sdk::panic_with_error!(&env, VaultError::UnsupportedAsset);
        }

        if storage::get_position(&env, &user).is_none() {
            soroban_sdk::panic_with_error!(&env, VaultError::NoPosition);
        }

        // Safety check: collateral ratio (must happen before the debit)
        if !Self::is_withdrawal_safe(env.clone(), user.clone(), asset.clone(), amount) {
            soroban_sdk::panic_with_error!(&env, VaultError::BelowMinCollateralRatio);
        }

        position::checked_debit(&env, &user, &asset, amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

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

    /// Reads up to 64 positions from the active index at the current ledger.
    pub fn get_positions_page(env: Env, offset: u32, limit: u32) -> Vec<Position> {
        let mut positions = Vec::new(&env);
        for user in storage::index_page(&env, offset, limit.min(64)).iter() {
            if let Some(position) = storage::get_position(&env, &user) {
                positions.push_back(position);
            }
        }
        positions
    }

    pub fn migrate_position_index(env: Env, limit: u32) -> u32 {
        storage::migrate_index(&env, limit)
    }

    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        liquidation::execute_batch(
            &env,
            liquidation_engine,
            user,
            Vec::from_array(&env, [shared::CollateralAsset { asset, amount }]),
        )
    }
    pub fn seize_batch(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        pool: Address,
        oracle: Address,
        basket: Vec<shared::CollateralAsset>,
    ) -> Result<(), VaultError> {
        if storage::get_lending_pool(&env) != Some(pool)
            || storage::get_oracle(&env) != Some(oracle)
        {
            return Err(VaultError::InvalidAddress);
        }
        liquidation::execute_batch(&env, liquidation_engine, user, basket)
    }

    pub fn is_withdrawal_safe(env: Env, user: Address, asset: Address, amount: i128) -> bool {
        if amount <= 0
            || env
                .storage()
                .persistent()
                .has(&types::DataKey::PositionIndex)
            || storage::is_operation_paused(&env, &PauseFlag::Withdraw)
            || !storage::is_supported_asset(&env, &asset)
            || amount > storage::get_position_balance(&env, &user, &asset)
        {
            return false;
        }
        let debt = if let Some(pool_addr) = storage::get_lending_pool(&env) {
            let pool_client = LendingPoolClient::new(&env, &pool_addr);
            match pool_client.try_get_user_debt(&user) {
                Ok(Ok(debt)) => debt,
                _ => return false,
            }
        } else {
            return false;
        };

        if debt == 0 {
            return true;
        }

        risk::is_post_withdraw_healthy(&env, &user, &asset, amount, debt).unwrap_or(false)
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
mod liquidation;
mod position;
mod storage;
#[cfg(test)]
mod tests;
mod types;
mod upgrade;
