#![no_std]

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

use errors::VaultError;
use types::{AssetsPage, Position, PositionsPage};

// ─────────────────────────────────────────────────────────────────────────────
// Module declarations
// ─────────────────────────────────────────────────────────────────────────────

mod admin;
mod assets;
pub mod clients;
pub mod errors;
mod events;
mod liquidation;
mod position;
mod risk;
mod storage;
#[cfg(test)]
mod tests;
pub mod types;
mod upgrade;
mod views;

// ─────────────────────────────────────────────────────────────────────────────
// Contract entry point — thin delegating stubs only
// ─────────────────────────────────────────────────────────────────────────────

#[contract]
pub struct VaultContract;

#[contractimpl]
impl VaultContract {
    // ── Initialization ────────────────────────────────────────────────────────

    pub fn initialize(
        env: Env,
        admin: Address,
        lending_pool: Address,
        oracle: Address,
        liquidation_engine: Address,
    ) -> Result<(), VaultError> {
        admin::initialize(&env, admin, lending_pool, oracle, liquidation_engine)
    }

    // ── Upgrade / versioning ──────────────────────────────────────────────────

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

    // ── Admin ─────────────────────────────────────────────────────────────────

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
        admin::set_admin(env, new_admin)
    }

    pub fn set_lending_pool(env: Env, lending_pool: Address) {
        admin::set_lending_pool(env, lending_pool);
    }

    pub fn set_oracle(env: Env, oracle: Address) {
        admin::set_oracle(env, oracle);
    }

    pub fn set_liquidation_engine(env: Env, engine: Address) {
        admin::set_liquidation_engine(env, engine);
    }

    pub fn pause(env: Env) {
        admin::pause(env);
    }

    pub fn unpause(env: Env) {
        admin::unpause(env);
    }

    // ── Asset management ──────────────────────────────────────────────────────

    pub fn add_supported_asset(env: Env, asset: Address) {
        assets::add_supported_asset(env, asset);
    }

    pub fn remove_supported_asset(env: Env, asset: Address) {
        assets::remove_supported_asset(env, asset);
    }

    pub fn is_supported_asset(env: Env, asset: Address) -> bool {
        assets::is_supported_asset(env, asset)
    }

    // ── Core write operations ─────────────────────────────────────────────────

    pub fn deposit(env: Env, user: Address, asset: Address, amount: i128) {
        position::deposit(&env, user, asset, amount);
    }

    pub fn withdraw(env: Env, user: Address, asset: Address, amount: i128) {
        position::withdraw(&env, user, asset, amount);
    }

    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    ) {
        liquidation::seize_collateral(&env, liquidation_engine, user, asset, amount);
    }

    pub fn authorize_liquidation(env: Env, liquidation_engine: Address, user: Address) -> bool {
        liquidation::authorize_liquidation(&env, liquidation_engine, user)
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

    pub fn get_position(env: Env, user: Address) -> Position {
        match storage::get_position(&env, &user) {
            Some(p) => p,
            None => soroban_sdk::panic_with_error!(&env, VaultError::NoPosition),
        }
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        risk::get_collateral_value(&env, &user)
    }

    pub fn is_withdrawal_safe(env: Env, user: Address, asset: Address, amount: i128) -> bool {
        risk::is_withdrawal_safe(&env, &user, &asset, amount)
    }

    pub fn get_position_index(env: Env) -> soroban_sdk::Vec<Address> {
        storage::get_position_index(&env)
    }

    pub fn get_position_count(env: Env) -> u32 {
        storage::position_count(&env)
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
