#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};

use clients::{LendingPoolClient, OracleAdapterClient};
use errors::VaultError;
use types::Position;

/// Oracle prices are encoded with 7 decimal places (e.g. USD 1.00 = 10_000_000).
/// Dividing `amount * price` by this constant yields the USD-denominated value.
const PRICE_PRECISION: i128 = 10_000_000;

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    /// Initialise the vault.  Must be called exactly once after deployment.
    ///
    /// * `admin`        — the initial administrator address.
    /// * `lending_pool` — address of the oracle-adapter contract (legacy
    ///                    parameter name kept for backward compatibility with
    ///                    existing test harnesses; use `set_oracle` to update).
    pub fn initialize(env: Env, admin: Address, lending_pool: Address) {
        // Strict initialization guard: panic if already initialized.
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

        // Commit admin and configured contract addresses to persistent storage.
        // Commit admin and all configured contract addresses to persistent storage
        storage::set_admin(&env, &admin);
        storage::set_lending_pool(&env, &lending_pool);
        storage::set_oracle(&env, &oracle);
        storage::set_liquidation_engine(&env, &liquidation_engine);

        // Explicitly set Paused to false.
        storage::set_paused(&env, false);

        storage::set_contract_version(&env, upgrade::CURRENT_CONTRACT_VERSION);
        storage::set_storage_schema_version(&env, upgrade::CURRENT_STORAGE_SCHEMA_VERSION);

        // Emit structured contract event.
        events::Initialized {
            admin,
            lending_pool,
            oracle,
            liquidation_engine,
        }
        .publish(&env);

        Ok(())
    }

    /// Returns the current contract bytecode version.
    pub fn get_contract_version(env: Env) -> u32 {
        upgrade::get_contract_version(&env)
    }

    /// Returns the current storage-schema version.
    pub fn get_storage_schema_version(env: Env) -> u32 {
        upgrade::get_storage_schema_version(&env)
    }

    /// Upgrade the contract bytecode.  Caller must be the admin.
    pub fn upgrade(env: Env, wasm_hash: BytesN<32>) -> Result<(), VaultError> {
        upgrade::upgrade(env, wasm_hash)
    }

    /// Apply incremental storage migrations up to `target_storage_schema_version`.
    pub fn migrate(env: Env, target_storage_schema_version: u32) -> Result<(), VaultError> {
        upgrade::migrate(env, target_storage_schema_version)
    }

    /// Replace the admin address.  Caller must be the current admin.
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        admin::set_admin(env, new_admin)
    }

    /// Update the configured lending-pool address.  Caller must be the admin.
    pub fn set_lending_pool(env: Env, lending_pool: Address) {
        admin::set_lending_pool(env, lending_pool)
    }

    /// Update the configured oracle-adapter address.  Caller must be the admin.
    pub fn set_oracle(env: Env, oracle: Address) {
        admin::set_oracle(env, oracle)
    }

    /// Update the configured liquidation-engine address.  Caller must be the admin.
    pub fn set_liquidation_engine(env: Env, engine: Address) {
        admin::set_liquidation_engine(env, engine)
    }

    /// Update the configured lending-pool (pool) address.  Caller must be the admin.
    pub fn set_pool(env: Env, pool: Address) {
        admin::set_pool(env, pool)
    }

    /// Pause all state-mutating vault operations.  Caller must be the admin.
    pub fn pause(env: Env) {
        admin::pause(env)
    }

    /// Resume vault operations.  Caller must be the admin.
    pub fn unpause(env: Env) {
        admin::unpause(env)
    }

    /// Add `asset` to the supported-asset allowlist.  Caller must be the admin.
    pub fn add_supported_asset(env: Env, asset: Address) {
        assets::add_supported_asset(env, asset)
    }

    /// Remove `asset` from the supported-asset allowlist.  Caller must be the admin.
    pub fn remove_supported_asset(env: Env, asset: Address) {
        assets::remove_supported_asset(env, asset)
    }

    /// Returns `true` when `asset` is on the supported-asset allowlist.
    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        assets::is_supported_asset(env, asset)
    }

    /// Check whether a liquidation engine is authorised to liquidate `user`.
    ///
    /// Delegates the health check to the configured `LendingPoolClient`.
    /// The `is_liquidatable` call uses the canonical interface signature:
    /// `fn is_liquidatable(env: Env, user: Address) -> bool`.
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

        let pool_address = storage::get_lending_pool(&env).expect("Lending pool not set");
        let pool_client = LendingPoolClient::new(&env, &pool_address);
        // Canonical signature: fn is_liquidatable(env: Env, user: Address) -> bool
        pool_client.is_liquidatable(&user)
    }

    /// Returns the admin address if the contract has been initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    /// Returns the deposited balance of `asset` for `user`.
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

    /// Returns the ordered list of all users that have an active position.
    pub fn get_position_index(env: Env) -> Vec<Address> {
        storage::get_position_index(&env)
    }

    /// Deposit `amount` of `asset` into the vault on behalf of `user`.
    ///
    /// Requires `user` auth.  The vault must not be paused and `asset` must be
    /// on the supported-asset allowlist.
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

        // Track this asset for the user (used to build Position).
        storage::add_user_asset(&env, &user, &asset);
        // Add user to the global position index if not already present.
        storage::add_to_position_index(&env, &user);

        events::Deposited {
            user,
            asset,
            amount,
        }
        .publish(&env);
    }

    /// Withdraw `amount` of `asset` from the vault for `user`.
    ///
    /// Requires `user` auth.  Enforces minimum collateral ratio (110 %).
    pub fn withdraw(env: Env, user: Address, asset: Address, amount: i128) {
        user.require_auth();

        position::validate_positive_amount(amount)
            .unwrap_or_else(|e| soroban_sdk::panic_with_error!(&env, e));

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

        // Safety check: collateral ratio.
        // Safety check: collateral ratio (must happen before the debit)
        if !Self::is_withdrawal_safe(env.clone(), user.clone(), asset.clone(), amount) {
            soroban_sdk::panic_with_error!(&env, VaultError::BelowMinCollateralRatio);
        }

        let new_balance = balance - amount;
        storage::set_position_balance(&env, &user, &asset, new_balance);

        // If this asset balance reached zero, remove asset from user's assets list.
        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        // If the user has no remaining balance across any asset, remove from index.
        if storage::get_position(&env, &user).is_none() {
            storage::remove_from_position_index(&env, &user);
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

    /// Returns a snapshot of all active user positions.
    pub fn get_all_positions(env: Env) -> Vec<Position> {
        storage::get_all_positions(&env)
    }

    /// Seize `amount` of `asset` from `user`'s position and transfer it to
    /// `liquidation_engine`.
    ///
    /// Callable only by the registered liquidation engine.
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

        // Verify user has an active position.
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

        // If this asset balance reached zero, remove asset from user's assets list.
        if new_balance == 0 {
            storage::remove_user_asset(&env, &user, &asset);
        }

        // If the user has no remaining balance across any asset, remove from index.
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
    ) -> Result<(), VaultError> {
        liquidation::execute_seize(&env, liquidation_engine, user, asset, amount)
    }

    /// Returns `true` when withdrawing `amount` of `asset` for `user` would
    /// keep their collateral ratio at or above the 110 % minimum.
    pub fn is_withdrawal_safe(env: Env, user: Address, asset: Address, amount: i128) -> bool {
        let debt = if let Some(pool_addr) = storage::get_lending_pool(&env) {
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
        let oracle_client = OracleAdapterClient::new(&env, &oracle_address);
        let price_data = oracle_client.get_price(&asset).expect("price not found");

        // Apply the same PRICE_PRECISION scaling used by get_collateral_value so
        // that withdrawn_value is denominated in USD and comparable to total_value.
        let withdrawn_value = amount
            .checked_mul(price_data.price)
            .unwrap_or_else(|| panic!("overflow in withdrawn value calculation"))
            / PRICE_PRECISION;

        if total_value < withdrawn_value {
            return false;
        }

        let remaining_value = total_value - withdrawn_value;

        // Minimum collateral ratio: 110 % (1.1).
        remaining_value >= (debt * 110) / 100
    }

    /// Returns the full collateral position for `user`.
    ///
    /// Panics with `VaultError::NoPosition` if the user has no open position.
    pub fn get_position(env: Env, user: Address) -> Position {
        match storage::get_position(&env, &user) {
            Some(position) => position,
            None => soroban_sdk::panic_with_error!(&env, VaultError::NoPosition),
        }
    }

    /// Returns the total USD-denominated collateral value for `user`, scaled by
    /// `PRICE_PRECISION` (10^7).
    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        let position = Self::get_position(env.clone(), user);

        let oracle_address = storage::get_oracle(&env).expect("oracle not configured");
        let oracle_client = OracleAdapterClient::new(&env, &oracle_address);

        let mut total_value: i128 = 0;

        for item in position.collateral.iter() {
            let price_data = oracle_client.get_price_or_fail(&item.asset);

            // Compute USD value: amount * price / PRICE_PRECISION.
            // checked_mul guards against overflow before the safe integer division.
            let item_value = item
                .amount
                .checked_mul(price_data.price)
                .unwrap_or_else(|| panic!("overflow in value calculation"))
                / PRICE_PRECISION;

            total_value = total_value
                .checked_add(item_value)
                .unwrap_or_else(|| panic!("overflow in total value calculation"));
        }

        total_value
    }
}

mod admin;
mod assets;
mod clients;
mod errors;
mod events;
mod liquidation;
mod position;
mod storage;
#[cfg(test)]
mod tests;
mod types;
mod upgrade;
