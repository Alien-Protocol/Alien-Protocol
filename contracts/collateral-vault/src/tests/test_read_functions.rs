#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

const ORACLE_STALE_THRESHOLD: u64 = 300;

#[contract]
pub struct MockOracleContract;

#[contractimpl]
impl MockOracleContract {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        let price_data: types::PriceData = env
            .storage()
            .persistent()
            .get(&asset)
            .expect("price not found");
        let current_time = env.ledger().timestamp();
        let age = current_time
            .checked_sub(price_data.timestamp)
            .expect("timestamp underflow");
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
    Address, // admin
    Address, // user
    Address, // token_id
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address, // oracle_id
    MockOracleContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
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

// ── get_position ─────────────────────────────────────────────────────────────

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
fn test_get_position_no_position_fails_with_no_position() {
    let (_env, client, _admin, user, _token_id, _token_client, _token_admin, _, _) = setup_env();

    let err = client.try_get_position(&user).unwrap_err().unwrap();
    assert_eq!(err, VaultError::NoPosition);
}

#[test]
fn test_get_position_after_partial_withdraw() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, _) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.withdraw(&user, &token_id, &200);

    let position = client.get_position(&user);
    assert_eq!(position.collateral.get(0).unwrap().amount, 300);
}

#[test]
fn test_get_position_after_full_withdraw_fails_with_no_position() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, _) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.withdraw(&user, &token_id, &500);

    let err = client.try_get_position(&user).unwrap_err().unwrap();
    assert_eq!(err, VaultError::NoPosition);
}

// ── get_collateral_value ─────────────────────────────────────────────────────

#[test]
fn test_get_collateral_value_correct_calculation() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // $10.00 price → 500 * 10_000_000 / 10_000_000 = 500 USD.
    oracle.set_price(&token_id, &10_000_000, &1000);

    let val = client.get_collateral_value(&user);
    assert_eq!(val, 500);
}

#[test]
fn test_get_collateral_value_no_position_fails_with_no_position() {
    let (_env, client, _admin, user, _token_id, _token_client, _token_admin, _, _) = setup_env();

    let err = client
        .try_get_collateral_value(&user)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::NoPosition);
}

#[test]
fn test_get_collateral_value_stale_price_fails() {
    let (env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Price at t=600; ledger at t=1000 → age=400 > 300 stale threshold.
    oracle.set_price(&token_id, &10_000_000, &600);
    env.ledger().set_timestamp(1000);

    // The MockOracle panics with "stale price"; the host surfaces that as a
    // contract error. We verify the call fails (the specific host-level error
    // code is an implementation detail of the mock, not our contract logic).
    let res = client.try_get_collateral_value(&user);
    assert!(res.is_err(), "stale price should cause failure");
}

#[test]
fn test_get_collateral_value_precision() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    // 10^12 units at $100 (1_000_000_000 in 7-decimal format).
    // USD value = 10^12 * 10^9 / 10^7 = 10^14.
    let large_amount = 1_000_000_000_000_i128;
    token_admin.mint(&user, &large_amount);
    client.deposit(&user, &token_id, &large_amount);

    oracle.set_price(&token_id, &1_000_000_000_i128, &1000);

    let val = client.get_collateral_value(&user);
    assert_eq!(val, 100_000_000_000_000_i128);
}

#[test]
fn test_get_collateral_value_uses_latest_price() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // $10.00 → $500
    oracle.set_price(&token_id, &10_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 500);

    // $12.00 → $600
    oracle.set_price(&token_id, &12_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 600);
}

#[test]
fn test_get_collateral_value_price_precision_applied() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin, _, oracle) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &1);

    // $1.00 → 1 unit × $1.00 = 1 (not 10_000_000)
    oracle.set_price(&token_id, &10_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 1);

    // $0.50 → 1 unit × $0.50 = 0 (integer floor)
    oracle.set_price(&token_id, &5_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 0);

    // $2.00 → 1 unit × $2.00 = 2
    oracle.set_price(&token_id, &20_000_000, &1000);
    assert_eq!(client.get_collateral_value(&user), 2);
}

// ── get_collateral_value — no price for the asset ────────────────────────────

#[test]
fn test_get_collateral_value_no_price_fails_with_price_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    // Use a real MockOracleContract so the oracle address resolves but has no
    // price stored for the deposited token.
    let oracle_id = env.register(MockOracleContract, ());

    let admin = Address::generate(&env);
    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    let user = Address::generate(&env);
    token_admin_client.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Oracle is configured but has no price entry for token_id → PriceNotFound.
    let err = client
        .try_get_collateral_value(&user)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::PriceNotFound);
}
