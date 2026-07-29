//! Integration tests — canonical cross-contract interfaces (INTERFACE_VERSION = 1)
//!
//! Each test registers the *real* workspace contract (oracle-adapter or
//! lending-pool) inside the Soroban test environment and exercises the vault's
//! generated client against it.  A compile-time failure here means one of:
//!
//!   * a function was renamed or removed from a deployed contract, or
//!   * the shared interface trait no longer matches the contract's public API.
//!
//! That is exactly the CI gate required by acceptance criterion 3 of issue #590.

#![cfg(test)]

extern crate std;

// Pull in the real workspace contracts so their WASM-generating `#[contract]`
// impls are available for registration.
use lending_pool::PoolContract;
use oracle_adapter::OracleContract;

use super::super::*;
use shared::interfaces::{LendingPoolClient, OracleAdapterClient};
use shared::types::PriceData;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::{Address, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Register and initialize a real oracle-adapter contract, then return its
/// address, a typed client backed by the shared interface, and the admin address.
fn register_oracle(env: &Env, staleness: u64) -> (Address, OracleAdapterClient<'_>, Address) {
    env.mock_all_auths();
    let oracle_id = env.register(OracleContract, ());
    let oracle_raw = oracle_adapter::OracleContractClient::new(env, &oracle_id);
    let admin = Address::generate(env);
    oracle_raw.initialize(&admin, &staleness);
    let client = OracleAdapterClient::new(env, &oracle_id);
    (oracle_id, client, admin)
}

/// Register a real lending-pool contract and return its address and a typed
/// client backed by the shared interface.
fn register_pool(env: &Env) -> (Address, LendingPoolClient<'_>) {
    let pool_id = env.register(PoolContract, ());
    let client = LendingPoolClient::new(env, &pool_id);
    (pool_id, client)
}

// ── Oracle interface compatibility ────────────────────────────────────────────

/// `OracleAdapterClient::get_price` returns `None` for an unknown asset.
///
/// Tests INTERFACE_VERSION = 1: `get_price(env, asset) -> Option<PriceData>`
#[test]
fn test_oracle_get_price_unknown_returns_none() {
    let env = Env::default();
    env.mock_all_auths();
    let (_id, client, _admin) = register_oracle(&env, 300);

    let asset = Address::generate(&env);
    let result = client.get_price(&asset);
    assert!(result.is_none());
}

/// After a price is published via the real oracle contract, `get_price` returns
/// a `PriceData` value whose type is the canonical `shared::types::PriceData`.
///
/// Tests INTERFACE_VERSION = 1: `get_price(env, asset) -> Option<PriceData>`
#[test]
fn test_oracle_get_price_returns_shared_price_data() {
    let env = Env::default();
    env.mock_all_auths();
    let (oracle_id, client, admin) = register_oracle(&env, 300);

    // Push a price through the concrete oracle contract's own client.
    let oracle_raw = oracle_adapter::OracleContractClient::new(&env, &oracle_id);
    let asset = Address::generate(&env);
    oracle_raw.set_price(&admin, &asset, &10_000_000_i128, &100_u64);

    // Retrieve it via the shared interface client.
    let price: Option<PriceData> = client.get_price(&asset);
    assert!(price.is_some());
    let pd = price.unwrap();
    assert_eq!(pd.price, 10_000_000);
    assert_eq!(pd.timestamp, 100);
}

/// `OracleAdapterClient::is_price_fresh` returns `false` for an asset with no
/// stored price.
///
/// Tests INTERFACE_VERSION = 1: `is_price_fresh(env, asset) -> bool`
#[test]
fn test_oracle_is_price_fresh_unknown_returns_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (_id, client, _admin) = register_oracle(&env, 300);

    let asset = Address::generate(&env);
    assert!(!client.is_price_fresh(&asset));
}

// ── LendingPool interface compatibility ───────────────────────────────────────

/// `LendingPoolClient::get_user_debt` returns `0` for any user when the pool
/// has no debt state (scaffolded implementation).
///
/// Tests INTERFACE_VERSION = 1: `get_user_debt(env, user) -> i128`
#[test]
fn test_pool_get_user_debt_returns_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (_id, client) = register_pool(&env);

    let user = Address::generate(&env);
    assert_eq!(client.get_user_debt(&user), 0_i128);
}

/// `LendingPoolClient::is_liquidatable` returns `false` for any user when the
/// pool has no debt state (scaffolded implementation).
///
/// Tests INTERFACE_VERSION = 1: `is_liquidatable(env, user) -> bool`
#[test]
fn test_pool_is_liquidatable_returns_false() {
    let env = Env::default();
    env.mock_all_auths();
    let (_id, client) = register_pool(&env);

    let user = Address::generate(&env);
    assert!(!client.is_liquidatable(&user));
}

// ── Vault ↔ pool cross-contract call ─────────────────────────────────────────

/// The vault's `is_withdrawal_safe` delegates to `LendingPoolClient::get_user_debt`
/// on the real `PoolContract`.  When the pool returns 0 debt the withdrawal is
/// always safe — verifying the cross-contract call round-trip compiles and runs
/// correctly.
///
/// Tests INTERFACE_VERSION = 1 end-to-end vault→pool call.
#[test]
fn test_vault_withdrawal_safe_with_real_pool_zero_debt() {
    let env = Env::default();
    env.mock_all_auths();

    // Register vault.
    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);

    // Register real lending pool and oracle.
    let (pool_id, _pool_client) = register_pool(&env);
    let (oracle_id, _oracle_client, _oracle_admin) = register_oracle(&env, 300);

    let admin = Address::generate(&env);
    vault.initialize(&admin, &oracle_id);
    vault.set_pool(&pool_id);

    // Register a token and deposit.
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);

    vault.add_supported_asset(&token_id);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);
    vault.deposit(&user, &token_id, &500);

    // With 0 debt from the real pool, any withdrawal must be safe.
    assert!(vault.is_withdrawal_safe(&user, &token_id, &100));
}

// ── Vault ↔ oracle cross-contract call ───────────────────────────────────────

/// The vault's `get_collateral_value` calls `OracleAdapterClient::get_price_or_fail`
/// on the real `OracleContract`.  This test verifies the full call round-trip
/// with a live price observation.
///
/// Tests INTERFACE_VERSION = 1 end-to-end vault→oracle call.
#[test]
fn test_vault_collateral_value_with_real_oracle() {
    let env = Env::default();
    env.mock_all_auths();

    // Register vault.
    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);

    // Register real oracle.
    let (oracle_id, _oracle_client, oracle_admin) = register_oracle(&env, 300);
    let oracle_raw = oracle_adapter::OracleContractClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    vault.initialize(&admin, &oracle_id);

    // Register a token and deposit.
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_id);

    vault.add_supported_asset(&token_id);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1_000);
    vault.deposit(&user, &token_id, &500);

    // Publish a price: $2.00 in 7-decimal notation = 20_000_000.
    // The oracle staleness window is 300 s; set ledger time within that window.
    env.ledger().set_timestamp(100);
    oracle_raw.set_price(&oracle_admin, &token_id, &20_000_000_i128, &100_u64);

    // Expected value: 500 * 20_000_000 / 10_000_000 = 1_000 USD
    let value = vault.get_collateral_value(&user);
    assert_eq!(value, 1_000);
}
