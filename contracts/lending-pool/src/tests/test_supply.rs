#![cfg(test)]

use super::super::*;
use crate::errors::PoolError;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Symbol};

#[contract]
pub struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let mut collateral = soroban_sdk::Vec::new(&env);
        collateral.push_back(shared::CollateralAsset {
            asset: Address::generate(&env),
            amount: 10_000,
        });
        shared::Position { user, collateral }
    }

    pub fn get_asset_config(_env: Env, _asset: Address) -> shared::AssetConfig {
        shared::AssetConfig {
            token_decimals: 7,
            oracle_price_decimals: 7,
            max_ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
        }
    }

    pub fn get_collateral_value(_env: Env, _user: Address) -> i128 {
        100_000
    }

    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        15_000
    }
}

fn setup_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vault = env.register(MockVaultContract, ());
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.initialize(&admin, &vault, &oracle, &token_contract_id, &500);

    (
        env,
        client,
        admin,
        user,
        vault,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_supply_success_pulls_tokens_and_records_balance() {
    let (_env, client, _admin, user, _vault, _token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&client.address), 500);
    assert_eq!(client.get_user_supply(&user), 500);
    assert_eq!(client.get_total_supply(), 500);
}

#[test]
fn test_supply_zero_or_negative_fails() {
    let (_env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);

    let res = client.try_supply(&user, &0);
    assert_eq!(res, Err(Ok(PoolError::InvalidAmount)));

    let res = client.try_supply(&user, &-100);
    assert_eq!(res, Err(Ok(PoolError::InvalidAmount)));
}

#[test]
fn test_withdraw_liquidity_zero_or_negative_fails() {
    let (_env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    let res_zero = client.try_withdraw_liquidity(&user, &0);
    assert_eq!(res_zero, Err(Ok(PoolError::InvalidAmount)));

    let res_neg = client.try_withdraw_liquidity(&user, &-100);
    assert_eq!(res_neg, Err(Ok(PoolError::InvalidAmount)));
}

#[test]
fn test_supply_when_paused_fails() {
    let (env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Supply, &Symbol::new(&env, "test"));

    let res = client.try_supply(&user, &500);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_liquidity_partial() {
    let (_env, client, _admin, user, _vault, _token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    assert_eq!(client.get_user_supply(&user), 500);
    assert_eq!(client.get_available_liquidity(), 500);

    client.withdraw_liquidity(&user, &200);

    assert_eq!(token_client.balance(&user), 700);
    assert_eq!(token_client.balance(&client.address), 300);
    assert_eq!(client.get_user_supply(&user), 300);
    assert_eq!(client.get_total_supply(), 300);
}

#[test]
fn test_withdraw_liquidity_exceeds_balance_fails() {
    let (_env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    let res = client.try_withdraw_liquidity(&user, &600);
    assert_eq!(res, Err(Ok(PoolError::InsufficientSupply)));
}

#[test]
fn test_withdraw_liquidity_insufficient_liquidity_when_borrowed() {
    let (env, client, _admin, supplier, _vault, token_id, token_client, token_admin) = setup_env();
    let borrower = Address::generate(&env);

    // Supplier deposits 1000 tokens
    token_admin.mint(&supplier, &1000);
    client.supply(&supplier, &1000);

    assert_eq!(client.get_user_supply(&supplier), 1000);
    assert_eq!(client.get_total_supply(), 1000);
    assert_eq!(client.get_available_liquidity(), 1000);

    // Borrower draws down 700 tokens of liquidity
    client.borrow(&borrower, &token_id, &700);

    assert_eq!(token_client.balance(&borrower), 700);
    assert_eq!(token_client.balance(&client.address), 300);
    assert_eq!(client.get_available_liquidity(), 300);

    // Supplier attempts to withdraw 500 tokens:
    // User's own supply is sufficient (1000 >= 500), but available liquidity is only 300 (< 500).
    // The call must fail with InsufficientLiquidity, distinguishing it from InsufficientSupply.
    let res_liquidity = client.try_withdraw_liquidity(&supplier, &500);
    assert_eq!(res_liquidity, Err(Ok(PoolError::InsufficientLiquidity)));

    // In contrast, requesting more than the supplier's own deposited balance (1200 > 1000)
    // fails with InsufficientSupply.
    let res_supply = client.try_withdraw_liquidity(&supplier, &1200);
    assert_eq!(res_supply, Err(Ok(PoolError::InsufficientSupply)));

    // Supplier can still successfully withdraw within available liquidity
    client.withdraw_liquidity(&supplier, &200);
    assert_eq!(client.get_user_supply(&supplier), 800);
    assert_eq!(client.get_total_supply(), 800);
    assert_eq!(client.get_available_liquidity(), 100);
    assert_eq!(token_client.balance(&supplier), 200);
}

#[test]
fn test_withdraw_liquidity_when_paused_fails() {
    let (env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    client.pause_operation(&PauseFlag::WithdrawLiquidity, &Symbol::new(&env, "test"));

    let res = client.try_withdraw_liquidity(&user, &200);
    assert!(res.is_err());
}

#[test]
fn test_utilization_empty_pool_is_zero() {
    let (_env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();
    assert_eq!(client.get_utilization_bps(), 0);
}

#[test]
fn test_utilization_half_full_is_five_thousand_bps() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    env.as_contract(&client.address, || {
        storage::set_total_supply(&env, 1_000);
        storage::set_total_borrowed(&env, 500);
    });

    assert_eq!(client.get_utilization_bps(), 5_000);
}

#[test]
fn test_utilization_full_is_ten_thousand_bps() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    env.as_contract(&client.address, || {
        storage::set_total_supply(&env, 1_000);
        storage::set_total_borrowed(&env, 1_000);
    });

    assert_eq!(client.get_utilization_bps(), 10_000);
}

/// `total_borrowed * 10_000` overflows i128 when `total_borrowed` is close to
/// `i128::MAX`.  The function must not panic and must clamp to 10_000 bps.
#[test]
fn test_utilization_large_borrowed_does_not_panic_and_clamps_to_ten_thousand() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    // total_borrowed chosen so that total_borrowed * 10_000 overflows i128.
    // i128::MAX / 10_000 ≈ 1.7e34, so anything above that will overflow.
    let huge: i128 = i128::MAX / 100;

    env.as_contract(&client.address, || {
        storage::set_total_supply(&env, 1);
        storage::set_total_borrowed(&env, huge);
    });

    // Must not panic; overflow path saturates to 10_000 bps.
    let util = client.get_utilization_bps();
    assert_eq!(util, 10_000);
}

/// Utilization is clamped even when borrowed legitimately exceeds supply
/// (e.g. due to accrued interest increasing total_borrowed above total_supply).
#[test]
fn test_utilization_over_supply_clamps_to_ten_thousand() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    env.as_contract(&client.address, || {
        storage::set_total_supply(&env, 1_000);
        storage::set_total_borrowed(&env, 2_000); // 200% — clamp to 10_000 bps
    });

    assert_eq!(client.get_utilization_bps(), 10_000);
}
