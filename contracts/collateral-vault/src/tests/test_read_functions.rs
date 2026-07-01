#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

const ORACLE_STALE_THRESHOLD: u64 = 300;

// Mock Oracle Contract to inject prices for testing get_collateral_value
#[contract]
pub struct MockOracleContract;

#[contractimpl]
impl MockOracleContract {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        let price_data: types::PriceData = match env.storage().persistent().get(&asset) {
            Some(pd) => pd,
            None => panic!("price not found"),
        };
        let current_time = env.ledger().timestamp();
        let age = match current_time.checked_sub(price_data.timestamp) {
            Some(delta) => delta,
            None => panic!("stale price"),
        };
        if age > ORACLE_STALE_THRESHOLD {
            panic!("stale price");
        }
        price_data
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let price_data = types::PriceData { price, timestamp };
        env.storage().persistent().set(&asset, &price_data);
    }
}

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address, // Oracle contract address
    MockOracleContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    // Set a sensible ledger timestamp for tests
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(MockOracleContract, ());
    let oracle_client = MockOracleContractClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.add_supported_asset(&token_contract_id);

    (
        env,
        client,
        admin,
        user,
        token_contract_id,
        token_client,
        token_admin_client,
        oracle_id,
        oracle_client,
    )
}

#[test]
fn test_get_position_returns_correct_data() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, _) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let position = client.get_position(&user);
    assert_eq!(position.user, user);
    assert_eq!(position.collateral.len(), 1);

    let asset_col = position.collateral.get(0).unwrap();
    assert_eq!(asset_col.asset, token_id);
    assert_eq!(asset_col.amount, 500);
}

#[test]
fn test_get_position_no_position_panics() {
    let (_env, client, _admin, user, _token_id, _token_client, _token_admin, _, _) = setup_env();

    let res = client.try_get_position(&user);
    assert!(res.is_err(), "should panic for unknown user");
}

#[test]
fn test_get_position_after_partial_withdraw() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, _) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Perform partial withdrawal
    client.withdraw(&user, &token_id, &200);

    let position = client.get_position(&user);
    assert_eq!(position.collateral.get(0).unwrap().amount, 300);
}

#[test]
fn test_get_collateral_value_correct_calculation() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Price: $10.00 encoded as 10_000_000 (7-decimal oracle format).
    // USD value = 500 * 10_000_000 / 10_000_000 = 500.
    oracle.set_price(&token_id, &10_000_000, &1000);

    let val = client.get_collateral_value(&user);
    assert_eq!(val, 500);
}

#[test]
fn test_get_collateral_value_no_position_panics() {
    let (_env, client, _admin, user, _token_id, _token_client, _token_admin, _, _) = setup_env();

    let res = client.try_get_collateral_value(&user);
    assert!(res.is_err(), "should panic for user with no position");
}

#[test]
fn test_get_collateral_value_stale_price_panics() {
    let (env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Mock price at timestamp 600
    oracle.set_price(&token_id, &10_000_000, &600);

    // Set ledger timestamp to 1000. Time difference is 400 (> 300 stale threshold)
    env.ledger().set_timestamp(1000);

    let res = client.try_get_collateral_value(&user);
    assert!(res.is_err(), "should panic due to stale price");
}

#[test]
fn test_get_collateral_value_precision() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    // 10^12 units at a price of $100 (1_000_000_000 in 7-decimal format).
    // USD value = 10^12 * 10^9 / 10^7 = 10^14.
    let large_amount = 1_000_000_000_000_i128;
    token_admin.mint(&user, &large_amount);
    client.deposit(&user, &token_id, &large_amount);

    let large_price = 1_000_000_000_i128;
    oracle.set_price(&token_id, &large_price, &1000);

    let val = client.get_collateral_value(&user);
    assert_eq!(val, 100_000_000_000_000_i128); // 10^14
}

#[test]
fn test_get_collateral_value_uses_latest_price() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // $10.00 price → 500 * 10_000_000 / 10_000_000 = $500
    oracle.set_price(&token_id, &10_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 500);

    // $12.00 price → 500 * 12_000_000 / 10_000_000 = $600
    oracle.set_price(&token_id, &12_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 600);
}

#[test]
fn test_get_position_after_full_withdraw_panics() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, _) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Full withdrawal
    client.withdraw(&user, &token_id, &500);

    let res = client.try_get_position(&user);
    assert!(res.is_err(), "should panic after full withdrawal");
}

/// Verifies that PRICE_PRECISION (10_000_000) is applied so that the returned
/// value is denominated in USD rather than raw (amount × raw_price) units.
#[test]
fn test_get_collateral_value_price_precision_applied() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &1);

    // $1.00 encoded as 10_000_000; 1 unit × $1.00 = $1 (i.e. 1, not 10_000_000).
    oracle.set_price(&token_id, &10_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 1);

    // $0.50 encoded as 5_000_000; 1 unit × $0.50 = 0 (integer floor).
    oracle.set_price(&token_id, &5_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 0);

    // $2.00 encoded as 20_000_000; 1 unit × $2.00 = $2.
    oracle.set_price(&token_id, &20_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 2);
}
