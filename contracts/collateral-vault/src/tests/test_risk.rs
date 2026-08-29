#![cfg(test)]

use super::super::*;
use crate::risk::{required_collateral_for_debt, rounded_quote_amount, RoundingMode};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

const ORACLE_STALE_THRESHOLD: u64 = 300;

#[contract]
pub struct MockLendingPool;

#[contractimpl]
impl MockLendingPool {
    pub fn get_user_debt(env: Env, _user: Address) -> i128 {
        env.storage().persistent().get(&"debt").unwrap_or(0)
    }

    pub fn set_user_debt(env: Env, debt: i128) {
        env.storage().persistent().set(&"debt", &debt);
    }
}

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
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
        let price_data = types::PriceData {
            price,
            timestamp,
            write_timestamp: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&asset, &price_data);
    }
}

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    token::StellarAssetClient<'static>,
    MockLendingPoolClient<'static>,
    MockOracleClient<'static>,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(MockOracle, ());
    let oracle_client = MockOracleClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let pool_id = env.register(MockLendingPool, ());
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &pool_id, &oracle_id, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    (
        env,
        client,
        admin,
        user,
        token_admin_client,
        pool_client,
        oracle_client,
        token_id,
    )
}

#[test]
fn test_set_asset_config_persists_ltv_and_threshold() {
    let (_env, client, _admin, _user, _token_admin, _pool, _oracle, token_id) = setup_env();

    client.set_asset_config(&token_id, &7, &7, &6_000, &7_500);
    let config = client.get_asset_config(&token_id);
    assert_eq!(config.token_decimals, 7);
    assert_eq!(config.oracle_price_decimals, 7);
    assert_eq!(config.max_ltv_bps, 6_000);
    assert_eq!(config.liquidation_threshold_bps, 7_500);
}

#[test]
fn test_set_asset_config_rejects_lt_not_greater_than_ltv() {
    let (_env, client, _admin, _user, _token_admin, _pool, _oracle, token_id) = setup_env();

    let equal = client.try_set_asset_config(&token_id, &7, &7, &8_000, &8_000);
    assert!(equal.is_err(), "LT equal to LTV must be rejected");

    let inverted = client.try_set_asset_config(&token_id, &7, &7, &8_000, &7_000);
    assert!(inverted.is_err(), "LT below LTV must be rejected");
}

#[test]
fn test_set_asset_config_rejects_bps_above_10000() {
    let (_env, client, _admin, _user, _token_admin, _pool, _oracle, token_id) = setup_env();

    let ltv_high = client.try_set_asset_config(&token_id, &7, &7, &10_001, &10_002);
    assert!(ltv_high.is_err());

    let lt_high = client.try_set_asset_config(&token_id, &7, &7, &7_000, &10_001);
    assert!(lt_high.is_err());

    let zero_ltv = client.try_set_asset_config(&token_id, &7, &7, &0, &8_000);
    assert!(zero_ltv.is_err());
}

#[test]
fn test_set_asset_config_rejects_risk_change_when_position_open() {
    let (_env, client, _admin, user, token_admin, _pool, _oracle, token_id) = setup_env();

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &token_id, &500);

    let same = client.try_set_asset_config(&token_id, &7, &7, &7_000, &8_000);
    assert!(same.is_ok(), "identical risk params stay writable");

    let changed = client.try_set_asset_config(&token_id, &7, &7, &6_000, &7_500);
    assert!(changed.is_err(), "open positions freeze LTV and LT");
}

#[test]
fn test_get_health_factor_architecture_example() {
    let (_env, client, _admin, user, token_admin, pool, oracle, token_id) = setup_env();

    client.set_asset_config(&token_id, &7, &7, &7_000, &8_000);
    // $1.00 encoded as 10_000_000; 10_000 tokens → collateral $10_000.
    oracle.set_price(&token_id, &10_000_000, &1000);
    token_admin.mint(&user, &10_000);
    client.deposit(&user, &token_id, &10_000);
    pool.set_user_debt(&7_500);

    assert_eq!(client.get_health_factor(&user), 10_666);
}

#[test]
fn test_get_health_factor_no_position_fails() {
    let (_env, client, _admin, user, _token_admin, _pool, _oracle, _token_id) = setup_env();

    let res = client.try_get_health_factor(&user);
    assert!(res.is_err(), "should fail for a user with no position");
}

#[test]
fn test_get_health_factor_zero_debt_is_healthy() {
    let (_env, client, _admin, user, token_admin, _pool, oracle, token_id) = setup_env();

    oracle.set_price(&token_id, &10_000_000, &1000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &token_id, &500);

    assert_eq!(client.get_health_factor(&user), i128::MAX);
}

#[test]
fn test_ceiling_rounding_does_not_understate_required_collateral() {
    // 1 * 11_000 / 10_000 = 1.1. Floor would require 1 and understate; ceiling is 2.
    assert_eq!(required_collateral_for_debt(1, 11_000), 2);
    assert_eq!(rounded_quote_amount(11_000, RoundingMode::Ceiling), 2);

    // Exact multiples stay exact.
    assert_eq!(required_collateral_for_debt(10, 11_000), 11);
    assert_eq!(rounded_quote_amount(110_000, RoundingMode::Ceiling), 11);

    // 3 * 11_000 / 10_000 = 3.3 → 4.
    assert_eq!(required_collateral_for_debt(3, 11_000), 4);
}
