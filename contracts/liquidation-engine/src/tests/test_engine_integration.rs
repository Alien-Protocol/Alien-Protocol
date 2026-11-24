#![cfg(test)]

//! Vault + Pool + Liquidation Engine integration tests (issue #629).
//!
//! These tests wire up the real `VaultContract` (from `collateral-vault`),
//! the real `PoolContract` (from `lending-pool`), and the real `LiquidationContract`
//! (from `liquidation-engine`) as separate contract instances in the same `Env`.
//! Cross-contract calls are exercised end-to-end:
//! - Vault manages collateral deposits, custody, valuation, and seizure.
//! - Pool manages liquidity supply, debt accrual, and repayment.
//! - Engine coordinates liquidation: pulling debt repayment from the liquidator,
//!   calling `pool.repay_for`, seizing collateral via `vault.seize_collateral`,
//!   and transferring the seized collateral (including an 8% bonus) to the liquidator.

use collateral_vault::{VaultContract, VaultContractClient};
use lending_pool::{PoolContract, PoolContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

use super::super::*;
use crate::errors::EngineError;

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<shared::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> shared::PriceData {
        match env.storage().persistent().get(&asset) {
            Some(price_data) => price_data,
            None => panic!("price not found"),
        }
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let price_data = shared::PriceData {
            price,
            timestamp,
            write_timestamp: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&asset, &price_data);
    }
}

/// 8% APR, matching `shared::V1_BORROW_APR_BPS`.
const INTEREST_RATE_BPS: u32 = 800;
/// $1.00 per token at the default 7-decimal oracle/token precision.
const INITIAL_COLLATERAL_PRICE: i128 = 10_000_000;
/// Collateral deposit of 1,000 tokens ($1,000 value at initial price).
const COLLATERAL_DEPOSIT: i128 = 1_000_000_000;
/// Borrow-side liquidity available in the pool ($5,000).
const LIQUIDITY_SUPPLY: i128 = 5_000_000_000;
/// Borrow amount of $500, within the 70% max-LTV limit ($700 on $1,000 collateral).
const BORROW_AMOUNT: i128 = 500_000_000;
/// Ample borrow asset minted to the liquidator to execute liquidations.
const LIQUIDATOR_FUNDS: i128 = 5_000_000_000;

#[allow(dead_code)]
struct TestSetup {
    env: Env,
    vault: VaultContractClient<'static>,
    pool: PoolContractClient<'static>,
    engine: LiquidationContractClient<'static>,
    oracle: MockOracleClient<'static>,
    collateral_token: Address,
    collateral_admin: token::StellarAssetClient<'static>,
    collateral_client: token::Client<'static>,
    borrow_token: Address,
    borrow_admin: token::StellarAssetClient<'static>,
    borrow_client: token::Client<'static>,
    user: Address,
    liquidator: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let lp = Address::generate(&env);
    let liquidator = Address::generate(&env);

    let oracle_id = env.register(MockOracle, ());
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);

    let pool_id = env.register(PoolContract, ());
    let pool = PoolContractClient::new(&env, &pool_id);

    let engine_id = env.register(LiquidationContract, ());
    let engine = LiquidationContractClient::new(&env, &engine_id);

    let collateral_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let collateral_token = collateral_contract.address();
    let collateral_admin = token::StellarAssetClient::new(&env, &collateral_token);
    let collateral_client = token::Client::new(&env, &collateral_token);

    let borrow_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let borrow_token = borrow_contract.address();
    let borrow_admin = token::StellarAssetClient::new(&env, &borrow_token);
    let borrow_client = token::Client::new(&env, &borrow_token);

    // Initialize vault with a placeholder engine address, then wire `set_liquidation_engine`
    // to verify the engine ID registration path explicitly.
    let dummy_engine = Address::generate(&env);
    vault.initialize(&admin, &pool_id, &oracle_id, &dummy_engine);
    vault.set_liquidation_engine(&engine_id);
    assert_eq!(vault.get_liquidation_engine(), Some(engine_id));

    pool.initialize(
        &admin,
        &vault_id,
        &oracle_id,
        &borrow_token,
        &INTEREST_RATE_BPS,
    );

    engine.initialize(&admin, &vault_id, &pool_id, &oracle_id);

    vault.add_supported_asset(&collateral_token);
    oracle.set_price(&collateral_token, &INITIAL_COLLATERAL_PRICE, &1_000);

    // Provide liquidity to the pool from the LP
    borrow_admin.mint(&lp, &LIQUIDITY_SUPPLY);
    pool.supply(&lp, &LIQUIDITY_SUPPLY);

    // Mint borrow asset to liquidator
    borrow_admin.mint(&liquidator, &LIQUIDATOR_FUNDS);

    TestSetup {
        env,
        vault,
        pool,
        engine,
        oracle,
        collateral_token,
        collateral_admin,
        collateral_client,
        borrow_token,
        borrow_admin,
        borrow_client,
        user,
        liquidator,
    }
}

/// Scenario:
/// 1. Deposit collateral (1,000 tokens @ $1.00 = $1,000 value).
/// 2. Supply liquidity to pool ($5,000 available).
/// 3. Borrow under LTV limit ($500 borrowed, max LTV is 70% = $700 limit).
/// 4. Health factor is initially 1.60 (healthy).
/// 5. Crash oracle price to $0.60, causing HF to drop to 0.96 (unhealthy, < 1.0).
/// 6. Liquidate user via liquidation engine.
/// 7. Assert debt is down, liquidator bonus is ~8% of repaid value,
///    and user leftover HF is either 0 (full close) or >= 10_000 (healthy).
#[test]
fn test_deposit_borrow_price_crash_liquidate_lifecycle() {
    let TestSetup {
        vault,
        pool,
        engine,
        oracle,
        collateral_token,
        collateral_admin,
        user,
        liquidator,
        ..
    } = setup();

    // 1. Deposit collateral
    collateral_admin.mint(&user, &COLLATERAL_DEPOSIT);
    vault.deposit(&user, &collateral_token, &COLLATERAL_DEPOSIT);
    assert_eq!(
        vault.get_position_balance(&user, &collateral_token),
        COLLATERAL_DEPOSIT
    );
    assert_eq!(vault.get_collateral_value(&user), COLLATERAL_DEPOSIT);

    // 2. Borrow under LTV limit (max limit is 70% of $1,000 = $700)
    let limit = pool.calculate_limit(&user);
    assert_eq!(limit, 700_000_000);
    pool.borrow(&user, &pool.get_borrow_asset().unwrap(), &BORROW_AMOUNT);

    let initial_debt = pool.get_user_debt(&user);
    assert_eq!(initial_debt, BORROW_AMOUNT);

    // Position is initially healthy: HF = (1,000 * 0.80) / 500 = 1.60 = 16_000 bps
    let initial_hf = vault.get_health_factor(&user);
    assert_eq!(initial_hf, 16_000);
    assert!(!engine.is_liquidatable(&user));

    // 3. Price crash: collateral price drops from $1.00 to $0.60
    // Collateral value becomes $600.
    // HF = (600 * 0.80) / 500 = 0.96 = 9_600 bps (< 10_000 bps)
    let crashed_price: i128 = 6_000_000; // $0.60
    oracle.set_price(&collateral_token, &crashed_price, &1_000);

    let crashed_hf = vault.get_health_factor(&user);
    assert_eq!(crashed_hf, 9_600);
    assert!(crashed_hf < shared::HEALTHY_HF_BPS);
    assert!(engine.is_liquidatable(&user));

    // 4. Liquidate
    let result = engine.liquidate(&liquidator, &user, &i128::MAX);

    // 5. Assert debt down
    let remaining_debt = pool.get_user_debt(&user);
    assert!(
        remaining_debt < initial_debt,
        "debt must decrease after liquidation"
    );
    assert_eq!(result.repaid, initial_debt - remaining_debt);

    // 6. Assert liquidator profit / bonus is 8% of repaid value
    let expected_bonus = shared::ceil_div(
        result.repaid * (shared::LIQUIDATION_BONUS_BPS as i128),
        10_000,
    )
    .unwrap();
    assert_eq!(result.bonus, expected_bonus);
    assert_eq!(result.seized, result.repaid + result.bonus);

    // 7. Assert user leftover HF is either 0 (full close) or >= 10_000 (healthy)
    if remaining_debt == 0 {
        // Full close path: remaining debt is cleared
        assert_eq!(remaining_debt, 0);
    } else {
        // Partial liquidation path: remaining position must be healthy
        let leftover_hf = vault.get_health_factor(&user);
        assert!(
            leftover_hf >= shared::HEALTHY_HF_BPS,
            "leftover HF must be >= 10_000 bps, got {}",
            leftover_hf
        );
    }
}

/// Verifies that the liquidation bonus paid to the liquidator is exactly 8%
/// of the repaid value, and that token balances correctly reflect this transfer.
#[test]
fn test_liquidation_bonus_is_eight_percent_of_repaid_value() {
    let TestSetup {
        vault,
        pool,
        engine,
        oracle,
        collateral_token,
        collateral_admin,
        collateral_client,
        borrow_client,
        user,
        liquidator,
        ..
    } = setup();

    // Deposit collateral and borrow
    collateral_admin.mint(&user, &COLLATERAL_DEPOSIT);
    vault.deposit(&user, &collateral_token, &COLLATERAL_DEPOSIT);
    pool.borrow(&user, &pool.get_borrow_asset().unwrap(), &BORROW_AMOUNT);

    // Crash price to trigger liquidation
    let crashed_price: i128 = 6_000_000; // $0.60
    oracle.set_price(&collateral_token, &crashed_price, &1_000);

    let liquidator_borrow_before = borrow_client.balance(&liquidator);
    let liquidator_collateral_before = collateral_client.balance(&liquidator);

    let result = engine.liquidate(&liquidator, &user, &i128::MAX);

    let liquidator_borrow_after = borrow_client.balance(&liquidator);
    let liquidator_collateral_after = collateral_client.balance(&liquidator);

    // Liquidator paid exactly `result.repaid` of borrow asset
    let borrow_spent = liquidator_borrow_before - liquidator_borrow_after;
    assert_eq!(borrow_spent, result.repaid);

    // Liquidator received collateral tokens
    let collateral_received = liquidator_collateral_after - liquidator_collateral_before;
    assert!(collateral_received > 0);

    // Bonus is strictly 8% of repaid value (LIQUIDATION_BONUS_BPS = 800)
    let expected_bonus = shared::ceil_div(result.repaid * 800, 10_000).unwrap();
    assert_eq!(result.bonus, expected_bonus);
    assert_eq!(result.seized, result.repaid + result.bonus);

    // Bonus percentage is 8% (scaled by 10_000 bps)
    let bonus_bps = (result.bonus * 10_000) / result.repaid;
    assert_eq!(bonus_bps, 800);

    // Converted to quote units, collateral received equals `result.seized`
    let seized_quote_value = (collateral_received * crashed_price) / shared::PROTOCOL_QUOTE_PRECISION;
    assert_eq!(seized_quote_value, result.seized);
}

/// Second scenario:
/// Verifies both liquidation execution branches matching the math:
/// 1. `max_repay_amount` below the close-factor cap executes a partial liquidation
///    when the leftover stays healthy. Asserts the partial liquidation path ran.
/// 2. In a deep price crash where a close-factor capped partial repayment would leave
///    the position unhealthy (post-HF < 1.0), the issue 12 fallback triggers and
///    executes a full close. Asserts the full-close path ran.
#[test]
fn test_max_repay_partial_or_full_close_matches_math() {
    let TestSetup {
        env,
        vault,
        pool,
        engine,
        oracle,
        collateral_token,
        collateral_admin,
        borrow_admin,
        liquidator,
        ..
    } = setup();

    // -----------------------------------------------------------------------
    // Branch 1: max_repay_amount below close-factor cap -> partial liquidation
    // -----------------------------------------------------------------------
    let user1 = Address::generate(&env);
    collateral_admin.mint(&user1, &COLLATERAL_DEPOSIT);
    vault.deposit(&user1, &collateral_token, &COLLATERAL_DEPOSIT);
    pool.borrow(&user1, &pool.get_borrow_asset().unwrap(), &BORROW_AMOUNT);

    // Price drops to $0.60:
    // Collateral value = $600, debt = $500.
    // Close factor cap is 50% = $250.
    // calculate_partial_repayment returns $250, leaving leftover HF = 10_560 (>= 10_000).
    oracle.set_price(&collateral_token, &6_000_000, &1_000);

    // Liquidator specifies max_repay_amount = $150 (below the $250 close-factor cap)
    let max_repay: i128 = 150_000_000;
    let res1 = engine.liquidate(&liquidator, &user1, &max_repay);

    // Assert partial liquidation path ran
    assert_eq!(res1.repaid, max_repay);
    let debt1_after = pool.get_user_debt(&user1);
    assert_eq!(debt1_after, BORROW_AMOUNT - max_repay);
    assert!(debt1_after > 0, "partial liquidation leaves positive debt");

    let hf1_after = vault.get_health_factor(&user1);
    assert!(
        hf1_after >= shared::HEALTHY_HF_BPS,
        "leftover HF must remain healthy (>= 10_000), got {}",
        hf1_after
    );

    // Reset price to initial $1.00 so user2 has full LTV headroom to borrow $500
    oracle.set_price(&collateral_token, &INITIAL_COLLATERAL_PRICE, &1_000);

    let user2 = Address::generate(&env);
    collateral_admin.mint(&user2, &COLLATERAL_DEPOSIT);
    vault.deposit(&user2, &collateral_token, &COLLATERAL_DEPOSIT);
    pool.borrow(&user2, &pool.get_borrow_asset().unwrap(), &BORROW_AMOUNT);

    // Reset price temporarily so user2 borrow succeeds
    // Now crash price deeply to $0.55 (5_500_000):
    // Collateral value = $550, debt = $500.
    // Pre-HF = (550 * 0.80) / 500 = 0.88 = 8_800 bps.
    // If capped at 50% ($250):
    // Seized value = 250 * 1.08 = $270.
    // Remaining collateral value = $550 - $270 = $280.
    // Remaining debt = $500 - $250 = $250.
    // Post-HF = (280 * 0.80) / 250 = 0.896 = 8_960 bps (< 10_000 bps!).
    // Because post-HF would be unhealthy, calculate_partial_repayment triggers
    // the issue 12 unsafe leftover fallback and returns the FULL debt ($500).
    oracle.set_price(&collateral_token, &5_500_000, &1_000);

    let calculated_repay = engine.calculate_partial_repayment(&user2);
    let debt2_before = pool.get_user_debt(&user2);
    assert_eq!(
        calculated_repay, debt2_before,
        "fallback must return full debt when capped repayment cannot restore health"
    );

    // Liquidator liquidates user2
    borrow_admin.mint(&liquidator, &LIQUIDATOR_FUNDS);
    let res2 = engine.liquidate(&liquidator, &user2, &i128::MAX);

    // Assert full close path ran
    assert_eq!(res2.repaid, debt2_before);
    let debt2_after = pool.get_user_debt(&user2);
    assert_eq!(debt2_after, 0, "full close must clear all debt");
}

/// Verifies that a healthy position (HF >= 1.0) cannot be liquidated,
/// end-to-end with real contracts.
#[test]
fn test_healthy_position_cannot_be_liquidated_end_to_end() {
    let TestSetup {
        vault,
        pool,
        engine,
        collateral_token,
        collateral_admin,
        user,
        liquidator,
        ..
    } = setup();

    // Deposit collateral ($1,000) and borrow ($500)
    collateral_admin.mint(&user, &COLLATERAL_DEPOSIT);
    vault.deposit(&user, &collateral_token, &COLLATERAL_DEPOSIT);
    pool.borrow(&user, &pool.get_borrow_asset().unwrap(), &BORROW_AMOUNT);

    // Price remains $1.00 -> HF is 1.60 (16_000 bps >= 10_000 bps)
    assert!(!engine.is_liquidatable(&user));
    assert_eq!(vault.get_health_factor(&user), 16_000);

    // Liquidation must be rejected with NotLiquidatable error
    let res = engine.try_liquidate(&liquidator, &user, &100_000_000);
    assert_eq!(res, Err(Ok(EngineError::NotLiquidatable)));

    // User position and pool debt remain completely untouched
    assert_eq!(pool.get_user_debt(&user), BORROW_AMOUNT);
    assert_eq!(
        vault.get_position_balance(&user, &collateral_token),
        COLLATERAL_DEPOSIT
    );
}
