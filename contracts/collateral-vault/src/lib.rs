#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};

use errors::VaultError;
use types::Position;

mod admin;
mod assets;
mod config;
mod errors;
mod events;
mod risk;
mod storage;
mod types;

#[cfg(test)]
mod tests;

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    // ── initialization ──────────────────────────────────────────────────────

    /// One-time contract initialization.
    ///
    /// Errors:
    /// - `AlreadyInitialized` — `initialize` has already been called.
    pub fn initialize(env: Env, admin: Address, lending_pool: Address) -> Result<(), VaultError> {
        if storage::has_admin(&env) {
            return Err(VaultError::AlreadyInitialized);
        }

        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &lending_pool);
        storage::set_paused(&env, false);

        storage::set_contract_version(&env, upgrade::CURRENT_CONTRACT_VERSION);
        storage::set_storage_schema_version(&env, upgrade::CURRENT_STORAGE_SCHEMA_VERSION);

        // Emit structured contract event
        events::Initialized {
            admin,
            lending_pool,
        }
        .publish(&env);

        Ok(())
    }

    // ── admin / configuration ───────────────────────────────────────────────

    /// Transfer admin authority to `new_admin`.
    ///
    /// Errors: `NotInitialized`, `AlreadyAdmin`.
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

    /// Update the lending-pool contract address.
    ///
    /// Errors: `NotInitialized`.
    pub fn set_lending_pool(env: Env, lending_pool: Address) -> Result<(), VaultError> {
        admin::set_lending_pool(env, lending_pool)
    }

    /// Update the oracle contract address.
    ///
    /// Errors: `NotInitialized`.
    pub fn set_oracle(env: Env, oracle: Address) -> Result<(), VaultError> {
        admin::set_oracle(env, oracle)
    }

    /// Register the liquidation-engine contract address.
    ///
    /// Errors: `NotInitialized`.
    pub fn set_liquidation_engine(env: Env, engine: Address) -> Result<(), VaultError> {
        admin::set_liquidation_engine(env, engine)
    }

    /// Register the lending pool address used for debt queries.
    ///
    /// Errors: `NotInitialized`.
    pub fn set_pool(env: Env, pool: Address) -> Result<(), VaultError> {
        admin::set_pool(env, pool)
    }

    /// Pause all state-mutating vault operations.
    ///
    /// Errors: `NotInitialized`, `AlreadyPaused`.
    pub fn pause(env: Env) -> Result<(), VaultError> {
        admin::pause(env)
    }

    /// Resume vault operations after a pause.
    ///
    /// Errors: `NotInitialized`, `NotPaused`.
    pub fn unpause(env: Env) -> Result<(), VaultError> {
        admin::unpause(env)
    }

    /// Upgrade the contract WASM.
    ///
    /// Errors: `NotInitialized`.
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), VaultError> {
        admin::upgrade(env, new_wasm_hash)
    }

    // ── supported-asset allow-list ──────────────────────────────────────────

    /// Add `asset` to the supported-asset allow-list.
    ///
    /// Errors: `NotInitialized`, `AlreadySupported`.
    pub fn add_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
    pub fn add_supported_asset(env: Env, asset: Address) {
        assets::add_supported_asset(env, asset)
    }

    /// Remove `asset` from the supported-asset allow-list.
    ///
    /// Errors: `NotInitialized`, `AssetNotFound`.
    pub fn remove_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
        assets::remove_supported_asset(env, asset)
    }

    /// Return whether `asset` is currently allow-listed.
    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        assets::is_supported_asset(env, asset)
    }

    // ── user operations ─────────────────────────────────────────────────────

    /// Deposit `amount` of `asset` into the vault on behalf of `user`.
    ///
    /// Errors:
    /// - `InvalidInputs`   — `amount` is zero or negative.
    /// - `VaultPaused`     — vault is paused.
    /// - `UnsupportedAsset`— `asset` is not allow-listed.
    pub fn deposit(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), VaultError> {
        user.require_auth();

        if amount <= 0 {
            return Err(VaultError::InvalidInputs);
        }

        if storage::is_paused(&env) {
            return Err(VaultError::VaultPaused);
        }

        if !storage::is_supported_asset(&env, &asset) {
            return Err(VaultError::UnsupportedAsset);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let balance = storage::get_position_balance(&env, &user, &asset);
        let new_balance = balance
            .checked_add(amount)
            .ok_or(VaultError::ArithmeticOverflow)?;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        storage::add_user_asset(&env, &user, &asset);
        storage::add_to_position_index(&env, &user);

        events::Deposited {
            user,
            asset,
            amount,
        }
        .publish(&env);

        Ok(())
    }

    /// Withdraw `amount` of `asset` from the vault to `user`.
    ///
    /// Errors:
    /// - `InvalidInputs`          — `amount` is zero or negative.
    /// - `VaultPaused`            — vault is paused.
    /// - `UnsupportedAsset`       — `asset` is not allow-listed.
    /// - `NoPosition`             — `user` has no active position.
    /// - `InsufficientBalance`    — `amount` exceeds the recorded balance.
    /// - `BelowMinCollateralRatio`— withdrawal would breach the 110 % floor.
    /// - `OracleNotConfigured`    — oracle address not set (during ratio check).
    /// - `PriceNotFound`          — oracle has no price for `asset`.
    /// - `ArithmeticOverflow`     — arithmetic overflow during ratio check.
    pub fn withdraw(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), VaultError> {
        user.require_auth();

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
            return Err(VaultError::InsufficientBalance);
        }

        // Collateral-ratio safety check.
        if !risk::is_withdrawal_safe(&env, &user, &asset, amount)? {
            return Err(VaultError::BelowMinCollateralRatio);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

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

        Ok(())
    }

    // ── liquidation ─────────────────────────────────────────────────────────

    /// Authorize a liquidation by verifying `liquidation_engine` and checking
    /// that `user` is liquidatable via the lending pool.
    ///
    /// Errors:
    /// - `LiquidationEngineNotSet` — no engine address is configured.
    /// - `Unauthorized`            — `liquidation_engine` does not match the
    ///                               registered address.
    /// - `NoPosition`              — `user` has no active position.
    /// - `PoolNotSet`              — lending-pool address is not configured.
    pub fn authorize_liquidation(
        env: Env,
        liquidation_engine: Address,
        user: Address,
    ) -> Result<bool, VaultError> {
        let stored_engine = config::require_liquidation_engine(&env)?;

        if liquidation_engine != stored_engine {
            return Err(VaultError::Unauthorized);
        }

        liquidation_engine.require_auth();

        if storage::get_position(&env, &user).is_none() {
            return Err(VaultError::NoPosition);
        }

        let pool_address = config::require_pool(&env)?;
        let pool_client = risk::LendingPoolClient::new(&env, &pool_address);
        Ok(pool_client.is_liquidatable(&user))
    }

    /// Seize `amount` of `asset` from `user`'s position and transfer it to
    /// `liquidation_engine`.
    ///
    /// Errors:
    /// - `LiquidationEngineNotSet` — no engine address is configured.
    /// - `Unauthorized`            — `liquidation_engine` does not match the
    ///                               registered address.
    /// - `VaultPaused`             — vault is paused.
    /// - `NoPosition`              — `user` has no active position.
    /// - `InsufficientBalance`     — `amount` exceeds the recorded balance.
    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), VaultError> {
        liquidation_engine.require_auth();

        let registered_engine = config::require_liquidation_engine(&env)?;
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
            return Err(VaultError::InsufficientBalance);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        if storage::get_position(&env, &user).is_none() {
            storage::remove_from_position_index(&env, &user);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&env.current_contract_address(), &liquidation_engine, &amount);

        events::CollateralSeized {
            user,
            asset,
            amount,
            liquidation_engine,
        }
        .publish(&env);

        Ok(())
    }

    // ── read helpers ────────────────────────────────────────────────────────

    /// Return the admin address, or `None` if not yet initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    /// Return the collateral position for `user`.
    ///
    /// Errors:
    /// - `NoPosition` — `user` has no active position.
    pub fn get_position(env: Env, user: Address) -> Result<Position, VaultError> {
        storage::get_position(&env, &user).ok_or(VaultError::NoPosition)
    }

    /// Return the raw balance of `asset` held by `user`.
    pub fn get_position_balance(env: Env, user: Address, asset: Address) -> i128 {
        storage::get_position_balance(&env, &user, &asset)
    }

    /// Return the list of all addresses that have ever held a non-zero balance.
    pub fn get_position_index(env: Env) -> Vec<Address> {
        storage::get_position_index(&env)
    }

    /// Return all active positions (users with at least one non-zero balance).
    pub fn get_all_positions(env: Env) -> Vec<Position> {
        storage::get_all_positions(&env)
    }

    /// Return the total USD-denominated value of `user`'s collateral.
    ///
    /// Errors:
    /// - `NoPosition`          — `user` has no active position.
    /// - `OracleNotConfigured` — oracle address is not set.
    /// - `PriceNotFound`       — oracle has no price for an asset.
    /// - `ArithmeticOverflow`  — arithmetic overflow.
    pub fn get_collateral_value(env: Env, user: Address) -> Result<i128, VaultError> {
        risk::collateral_value(&env, &user)
    }

    /// Check whether withdrawing `amount` of `asset` from `user` would keep
    /// the collateral ratio at or above the 110 % minimum.
    ///
    /// Returns `Ok(true)` when safe, `Ok(false)` when it would breach the
    /// ratio, and `Err(_)` for infrastructure failures.
    ///
    /// Errors:
    /// - `NoPosition`          — `user` has no active position.
    /// - `OracleNotConfigured` — oracle address is not set.
    /// - `PriceNotFound`       — oracle has no price for `asset`.
    /// - `ArithmeticOverflow`  — arithmetic overflow.
    pub fn is_withdrawal_safe(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<bool, VaultError> {
        risk::is_withdrawal_safe(&env, &user, &asset, amount)
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
mod upgrade;
