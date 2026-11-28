#![cfg(test)]

//! Native integration tests with the real oracle, vault, lending pool and liquidation engine.

use collateral_vault::{VaultContract, VaultContractClient};
use lending_pool::{PoolContract, PoolContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env};

use super::super::*;
use crate::errors::EngineError;

use oracle_adapter::{OracleContract, OracleContractClient};

/// 8% APR, matching `shared::V1_BORROW_APR_BPS`.
const INTEREST_RATE_BPS: u32 = 800;
/// $1.00 per token at the default 7-decimal oracle/token precision.
const INITIAL_COLLATERAL_PRICE: i128 = 10_000_000;
/// Collateral deposit of 1,000 tokens ($1,000 value at initial price).
const COLLATERAL_DEPOSIT: i128 = 10_000_000_000;
/// Borrow-side liquidity available in the pool ($5,000).
const LIQUIDITY_SUPPLY: i128 = 50_000_000_000;
/// Borrow amount of $500, within the 70% max-LTV limit ($700 on $1,000 collateral).
const BORROW_AMOUNT: i128 = 5_000_000_000;
/// Ample borrow asset minted to the liquidator to execute liquidations.
const LIQUIDATOR_FUNDS: i128 = 50_000_000_000;

#[allow(dead_code)]
struct TestSetup {
    env: Env,
    vault: VaultContractClient<'static>,
    pool: PoolContractClient<'static>,
    engine: LiquidationContractClient<'static>,
    admin: Address,
    oracle: OracleContractClient<'static>,
    collateral_token: Address,
    collateral_admin: token::StellarAssetClient<'static>,
    collateral_client: token::Client<'static>,
    borrow_token: Address,
    borrow_admin: token::StellarAssetClient<'static>,
    borrow_client: token::Client<'static>,
    user: Address,
    lp: Address,
    liquidator: Address,
}

extern crate std;

fn register_contract(
    env: &Env,
    admin: &Address,
    name: &str,
    native: impl FnOnce() -> Address,
) -> Address {
    if let Some(directory) = std::env::var_os("ALIEN_PROTOCOL_WASM_DIR") {
        let path = std::path::Path::new(&directory).join(std::format!("{name}.wasm"));
        let wasm = std::fs::read(&path).expect("build the release WASMs before running the suite");
        env.register(wasm.as_slice(), (admin,))
    } else {
        native()
    }
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let lp = Address::generate(&env);
    let liquidator = Address::generate(&env);

    let oracle_id = register_contract(&env, &admin, "oracle_adapter", || {
        env.register(OracleContract, (&admin,))
    });
    let oracle = OracleContractClient::new(&env, &oracle_id);

    oracle.initialize(&admin, &60);

    let vault_id = register_contract(&env, &admin, "collateral_vault", || {
        env.register(VaultContract, (&admin,))
    });
    let vault = VaultContractClient::new(&env, &vault_id);

    let pool_id = register_contract(&env, &admin, "lending_pool", || {
        env.register(PoolContract, (&admin,))
    });
    let pool = PoolContractClient::new(&env, &pool_id);

    let engine_id = register_contract(&env, &admin, "liquidation_engine", || {
        env.register(LiquidationContract, (&admin,))
    });
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

    vault.add_supported_asset(&collateral_token, &shared::AssetConfig::default());
    oracle.set_price(&admin, &collateral_token, &INITIAL_COLLATERAL_PRICE, &997);

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
        admin,
        oracle,
        collateral_token,
        collateral_admin,
        collateral_client,
        borrow_token,
        borrow_admin,
        borrow_client,
        user,
        lp,
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
        admin,
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
    assert_eq!(limit, 7_000_000_000);
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
    oracle.set_price(&admin, &collateral_token, &crashed_price, &1_000);

    let crashed_hf = vault.get_health_factor(&user);
    assert_eq!(crashed_hf, 9_600);
    assert!(crashed_hf < shared::HEALTHY_HF_BPS);
    assert!(engine.is_liquidatable(&user));
    assert!(pool.is_liquidatable(&user));

    // Only the liquidator signs. The nested authorization pays the pool directly.
    use soroban_sdk::{
        testutils::{MockAuth, MockAuthInvoke},
        IntoVal,
    };
    let env = &engine.env;
    let repay = engine.calculate_partial_repayment(&user);
    let asset = pool.get_borrow_asset().unwrap();
    env.mock_auths(&[MockAuth {
        address: &liquidator,
        invoke: &MockAuthInvoke {
            contract: &engine.address,
            fn_name: "liquidate",
            args: (liquidator.clone(), user.clone(), i128::MAX).into_val(env),
            sub_invokes: &[MockAuthInvoke {
                contract: &pool.address,
                fn_name: "repay_for",
                args: (liquidator.clone(), user.clone(), asset.clone(), repay).into_val(env),
                sub_invokes: &[MockAuthInvoke {
                    contract: &asset,
                    fn_name: "transfer",
                    args: (liquidator.clone(), pool.address.clone(), repay).into_val(env),
                    sub_invokes: &[],
                }],
            }],
        },
    }]);
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
            "leftover HF must be >= 10_000 bps, got {leftover_hf}"
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
        admin,
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
    oracle.set_price(&admin, &collateral_token, &crashed_price, &1_000);

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
    let seized_quote_value =
        (collateral_received * crashed_price) / shared::PROTOCOL_QUOTE_PRECISION;
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
        admin,
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
    oracle.set_price(&admin, &collateral_token, &6_000_000, &998);

    // Liquidator specifies max_repay_amount = $150 (below the $250 close-factor cap)
    let max_repay: i128 = 1_500_000_000;
    let res1 = engine.liquidate(&liquidator, &user1, &max_repay);

    // Assert partial liquidation path ran
    assert_eq!(res1.repaid, max_repay);
    let debt1_after = pool.get_user_debt(&user1);
    assert_eq!(debt1_after, BORROW_AMOUNT - max_repay);
    assert!(debt1_after > 0, "partial liquidation leaves positive debt");

    let hf1_after = vault.get_health_factor(&user1);
    assert!(
        hf1_after >= shared::HEALTHY_HF_BPS,
        "leftover HF must remain healthy (>= 10_000), got {hf1_after}"
    );

    // Reset price to initial $1.00 so user2 has full LTV headroom to borrow $500
    oracle.set_price(&admin, &collateral_token, &INITIAL_COLLATERAL_PRICE, &999);

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
    oracle.set_price(&admin, &collateral_token, &5_500_000, &1_000);

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

#[test]
fn stale_price_blocks_new_risk_until_refreshed() {
    let t = setup();
    t.collateral_admin.mint(&t.user, &COLLATERAL_DEPOSIT);
    t.vault
        .deposit(&t.user, &t.collateral_token, &COLLATERAL_DEPOSIT);
    t.pool.borrow(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    t.env.ledger().set_timestamp(1061);
    assert!(t.engine.try_is_liquidatable(&t.user).is_err());
    assert!(t.pool.try_is_liquidatable(&t.user).is_err());
    assert!(t.pool.try_borrow(&t.user, &t.borrow_token, &1).is_err());
    assert!(t
        .vault
        .try_withdraw(&t.user, &t.collateral_token, &1)
        .is_err());
    assert!(t
        .engine
        .try_liquidate(&t.liquidator, &t.user, &i128::MAX)
        .is_err());
    assert_eq!(
        t.collateral_client.balance(&t.vault.address),
        COLLATERAL_DEPOSIT
    );
    assert_eq!(t.borrow_client.balance(&t.liquidator), LIQUIDATOR_FUNDS);
    t.oracle.set_price(
        &t.admin,
        &t.collateral_token,
        &INITIAL_COLLATERAL_PRICE,
        &1061,
    );
    t.vault.withdraw(&t.user, &t.collateral_token, &1);
}

#[test]
fn failed_final_payment_rolls_back_collateral_movements() {
    let t = setup();
    t.collateral_admin.mint(&t.user, &COLLATERAL_DEPOSIT);
    t.vault
        .deposit(&t.user, &t.collateral_token, &COLLATERAL_DEPOSIT);
    t.pool.borrow(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    t.oracle
        .set_price(&t.admin, &t.collateral_token, &6_000_000, &1000);
    let unfunded = Address::generate(&t.env);
    assert!(t
        .engine
        .try_liquidate(&unfunded, &t.user, &i128::MAX)
        .is_err());
    assert_eq!(t.collateral_client.balance(&unfunded), 0);
    assert_eq!(
        t.collateral_client.balance(&t.vault.address),
        COLLATERAL_DEPOSIT
    );
    assert_eq!(
        t.vault.get_position_balance(&t.user, &t.collateral_token),
        COLLATERAL_DEPOSIT
    );
    assert_eq!(t.pool.get_user_debt(&t.user), BORROW_AMOUNT);
}

#[test]
fn interest_is_redeemable_pro_rata_by_current_suppliers() {
    let t = setup();
    let second = Address::generate(&t.env);
    t.borrow_admin.mint(&second, &LIQUIDITY_SUPPLY);
    t.pool.supply(&second, &LIQUIDITY_SUPPLY);
    t.collateral_admin.mint(&t.user, &COLLATERAL_DEPOSIT);
    t.vault
        .deposit(&t.user, &t.collateral_token, &COLLATERAL_DEPOSIT);
    t.pool.borrow(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    t.env
        .ledger()
        .set_timestamp(1000 + shared::SECONDS_PER_YEAR);
    let interest = BORROW_AMOUNT * 8 / 100;
    t.borrow_admin.mint(&t.user, &interest);
    t.pool.repay(&t.user, &t.borrow_token, &i128::MAX);
    let claim = LIQUIDITY_SUPPLY + interest / 2;
    assert_eq!(t.pool.get_user_supply(&t.lp), claim);
    assert_eq!(t.pool.get_user_supply(&second), claim);

    let late = Address::generate(&t.env);
    t.borrow_admin.mint(&late, &claim);
    t.pool.supply(&late, &claim);
    for supplier in [&t.lp, &second, &late] {
        assert_eq!(t.pool.get_user_supply(supplier), claim);
        t.pool.withdraw_liquidity(supplier, &claim);
        assert_eq!(t.borrow_client.balance(supplier), claim);
    }
    assert_eq!(t.pool.get_total_supply(), 0);
    assert_eq!(t.borrow_client.balance(&t.pool.address), 0);
}

#[test]
fn collateral_shortfall_leaves_collectible_borrower_debt() {
    let t = setup();
    t.collateral_admin.mint(&t.user, &COLLATERAL_DEPOSIT);
    t.vault
        .deposit(&t.user, &t.collateral_token, &COLLATERAL_DEPOSIT);
    t.pool.borrow(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    // Exactly 270 quote units cover a payment of 250 plus its 8% bonus.
    t.oracle
        .set_price(&t.admin, &t.collateral_token, &2_700_000, &1000);
    let covered = 250 * shared::PROTOCOL_QUOTE_PRECISION;
    assert_eq!(t.engine.calculate_partial_repayment(&t.user), covered);
    let result = t.engine.liquidate(&t.liquidator, &t.user, &i128::MAX);
    assert_eq!(result.repaid, covered);
    assert_eq!(
        t.collateral_client.balance(&t.liquidator),
        COLLATERAL_DEPOSIT
    );
    assert_eq!(t.pool.get_user_debt(&t.user), BORROW_AMOUNT - covered);
    assert_eq!(t.pool.get_user_supply(&t.lp), LIQUIDITY_SUPPLY);
    t.pool.repay(&t.user, &t.borrow_token, &i128::MAX);
    assert_eq!(t.pool.get_available_liquidity(), LIQUIDITY_SUPPLY);
}

#[test]
fn coarse_collateral_cannot_overpay_a_rounded_seizure() {
    let t = setup();
    t.oracle
        .set_price(&t.admin, &t.collateral_token, &100_000_000, &998);
    t.collateral_admin.mint(&t.user, &(COLLATERAL_DEPOSIT / 10));
    t.vault
        .deposit(&t.user, &t.collateral_token, &(COLLATERAL_DEPOSIT / 10));
    t.pool.borrow(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    t.oracle
        .set_price(&t.admin, &t.collateral_token, &60_000_000, &1000);
    assert!(t.engine.is_liquidatable(&t.user));
    assert_eq!(
        t.engine.try_liquidate(&t.liquidator, &t.user, &1),
        Err(Ok(EngineError::InvalidAmount))
    );
    assert_eq!(t.pool.get_user_debt(&t.user), BORROW_AMOUNT);
    assert_eq!(t.collateral_client.balance(&t.liquidator), 0);
    // A repayment with an exactly representable seizure still settles.
    assert_eq!(
        t.engine
            .liquidate(&t.liquidator, &t.user, &1_500_000_000)
            .repaid,
        1_500_000_000
    );
}

#[test]
fn batch_seizure_uses_the_initial_mixed_position_threshold() {
    let t = setup();
    let low = t
        .env
        .register_stellar_asset_contract_v2(Address::generate(&t.env))
        .address();
    let config = shared::AssetConfig {
        max_ltv_bps: 4900,
        liquidation_threshold_bps: 5000,
        ..Default::default()
    };
    t.vault.add_supported_asset(&low, &config);
    t.oracle
        .set_price(&t.admin, &low, &INITIAL_COLLATERAL_PRICE, &997);
    let unit = shared::PROTOCOL_QUOTE_PRECISION;
    token::StellarAssetClient::new(&t.env, &low).mint(&t.user, &(2000 * unit));
    t.vault.deposit(&t.user, &low, &(2000 * unit));
    t.collateral_admin.mint(&t.user, &(1000 * unit));
    t.vault
        .deposit(&t.user, &t.collateral_token, &(1000 * unit));
    t.pool.borrow(&t.user, &t.borrow_token, &(650 * unit));
    t.oracle.set_price(&t.admin, &low, &(unit / 10), &1000);
    let result = t.engine.liquidate(&t.liquidator, &t.user, &i128::MAX);
    assert_eq!(t.vault.get_position_balance(&t.user, &low), 0);
    assert!(t.collateral_client.balance(&t.liquidator) > 0);
    assert_eq!(t.pool.get_user_debt(&t.user), 650 * unit - result.repaid);
}

#[test]
fn live_debt_blocks_rotation_and_vault_pause_blocks_origination() {
    let t = setup();
    t.collateral_admin.mint(&t.user, &COLLATERAL_DEPOSIT);
    t.vault
        .deposit(&t.user, &t.collateral_token, &COLLATERAL_DEPOSIT);
    t.pool.borrow(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    assert!(t
        .vault
        .try_set_lending_pool(&Address::generate(&t.env))
        .is_err());
    assert!(t.pool.try_set_vault(&Address::generate(&t.env)).is_err());
    t.vault.pause_operation(
        &collateral_vault::PauseFlag::Borrow,
        &soroban_sdk::Symbol::new(&t.env, "test"),
    );
    assert!(t.pool.try_borrow(&t.user, &t.borrow_token, &1).is_err());
    t.pool.repay(&t.user, &t.borrow_token, &BORROW_AMOUNT);
    t.vault.set_lending_pool(&Address::generate(&t.env));
    t.vault
        .unpause_operation(&collateral_vault::PauseFlag::Borrow);
    assert!(t.pool.try_borrow(&t.user, &t.borrow_token, &1).is_err());
}

#[test]
fn position_writes_have_bounded_ledger_footprints() {
    fn measure(background: usize) -> [(u32, u32, u32); 2] {
        let t = setup();
        let deposit = |owner: &Address| {
            t.collateral_admin.mint(owner, &1);
            t.vault.deposit(owner, &t.collateral_token, &1);
        };
        for _ in 0..background {
            deposit(&Address::generate(&t.env));
        }
        let footprint = || {
            let cost = t.env.cost_estimate().resources();
            (
                cost.memory_read_entries,
                cost.write_entries,
                cost.write_bytes,
            )
        };
        deposit(&t.user);
        let insertion = footprint();
        deposit(&Address::generate(&t.env));
        t.vault.withdraw(&t.user, &t.collateral_token, &1);
        [insertion, footprint()]
    }
    assert_eq!(measure(3), measure(96));
}
