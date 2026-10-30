#![cfg(test)]

use collateral_vault::{VaultContract, VaultContractClient, VaultError};
use lending_pool::{PoolContract, PoolContractClient};
use oracle_adapter::{OracleContract, OracleContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env};

use crate::{EngineError, LiquidationContract, LiquidationContractClient};

/// Precision and scaling constants
const COLLATERAL_PRICE: i128 = 10_000_000; // $1.00 at 7 decimals
const BORROW_PRICE: i128 = 10_000_000; // $1.00 at 7 decimals
const INITIAL_TIMESTAMP: u64 = 1_000;
const STALENESS_THRESHOLD: u64 = 300; // 300 seconds

#[allow(dead_code)]
pub struct FourContractSetup {
    pub env: Env,
    pub admin: Address,
    pub oracle: OracleContractClient<'static>,
    pub vault: VaultContractClient<'static>,
    pub pool: PoolContractClient<'static>,
    pub engine: LiquidationContractClient<'static>,
    pub collateral_token: Address,
    pub collateral_admin: token::StellarAssetClient<'static>,
    pub borrow_token: Address,
    pub borrow_client: token::Client<'static>,
    pub borrow_admin: token::StellarAssetClient<'static>,
    pub user: Address,
    pub liquidator: Address,
    pub lp: Address,
}

fn setup() -> FourContractSetup {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(INITIAL_TIMESTAMP);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let lp = Address::generate(&env);

    // 1. Oracle Contract
    let oracle_id = env.register(OracleContract, ());
    let oracle = OracleContractClient::new(&env, &oracle_id);
    oracle.initialize(&admin, &STALENESS_THRESHOLD);

    // Tokens
    let collateral_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let collateral_token = collateral_contract.address();
    let collateral_admin = token::StellarAssetClient::new(&env, &collateral_token);

    let borrow_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let borrow_token = borrow_contract.address();
    let borrow_client = token::Client::new(&env, &borrow_token);
    let borrow_admin = token::StellarAssetClient::new(&env, &borrow_token);

    // Initial oracle prices: $1.00 for both assets at timestamp 1_000
    oracle.set_price(
        &admin,
        &collateral_token,
        &COLLATERAL_PRICE,
        &INITIAL_TIMESTAMP,
    );
    oracle.set_price(&admin, &borrow_token, &BORROW_PRICE, &INITIAL_TIMESTAMP);

    // 2. Vault Contract
    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);

    // 3. Pool Contract
    let pool_id = env.register(PoolContract, ());
    let pool = PoolContractClient::new(&env, &pool_id);

    // 4. Liquidation Engine Contract
    let engine_id = env.register(LiquidationContract, ());
    let engine = LiquidationContractClient::new(&env, &engine_id);

    // Initialization order: oracle -> vault -> pool -> engine, then vault.set_liquidation_engine
    vault.initialize(&admin, &pool_id, &oracle_id, &engine_id);
    pool.initialize(&admin, &vault_id, &oracle_id, &borrow_token, &800); // 8% APR
    engine.initialize(&admin, &vault_id, &pool_id, &oracle_id);
    vault.set_liquidation_engine(&engine_id);

    // Configure collateral asset in vault (decimals: 7, oracle: 7, max_ltv: 70%, liquidation_threshold: 80%)
    vault.add_supported_asset(&collateral_token);
    vault.set_asset_config(&collateral_token, &7, &7, &7000, &8000);

    // Fund accounts
    collateral_admin.mint(&user, &2_000_000_000); // $200 collateral
    borrow_admin.mint(&lp, &10_000_000_000); // $1,000 pool liquidity
    borrow_admin.mint(&user, &500_000_000); // $50 user starting borrow asset
    borrow_admin.mint(&liquidator, &10_000_000_000); // $1,000 liquidator funds

    FourContractSetup {
        env,
        admin,
        oracle,
        vault,
        pool,
        engine,
        collateral_token,
        collateral_admin,
        borrow_token,
        borrow_client,
        borrow_admin,
        user,
        liquidator,
        lp,
    }
}

/// Lifecycle test: deposit -> supply -> borrow -> repay -> withdraw with real oracle
#[test]
fn test_deposit_supply_borrow_repay_withdraw_with_real_oracle() {
    let s = setup();

    // 1. LP supplies 2,000 borrow asset ($200)
    s.pool.supply(&s.lp, &2_000_000_000);
    assert_eq!(s.pool.get_available_liquidity(), 2_000_000_000);

    // 2. User deposits 1,000 collateral ($100)
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);
    assert_eq!(
        s.vault.get_position_balance(&s.user, &s.collateral_token),
        1_000_000_000
    );
    assert_eq!(s.vault.get_collateral_value(&s.user), 1_000_000_000);

    // 3. User borrows 500 borrow asset ($50) (max LTV 70% allows up to $70)
    s.pool.borrow(&s.user, &s.borrow_token, &500_000_000);
    assert_eq!(s.pool.get_user_debt(&s.user), 500_000_000);
    assert_eq!(s.pool.get_available_liquidity(), 1_500_000_000);

    // 4. User repays debt in full
    s.pool.repay(&s.user, &s.borrow_token, &500_000_000);
    assert_eq!(s.pool.get_user_debt(&s.user), 0);
    assert_eq!(s.pool.get_available_liquidity(), 2_000_000_000);

    // 5. User withdraws collateral in full
    s.vault
        .withdraw(&s.user, &s.collateral_token, &1_000_000_000);
    assert_eq!(
        s.vault.get_position_balance(&s.user, &s.collateral_token),
        0
    );
}

/// Stale oracle price must block borrow operations
#[test]
fn test_stale_price_blocks_borrow() {
    let s = setup();

    s.pool.supply(&s.lp, &2_000_000_000);
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);

    // Advance ledger time beyond 300s staleness threshold without price update
    s.env.ledger().set_timestamp(INITIAL_TIMESTAMP + 500);

    // Borrow should fail because vault's collateral value check fails closed on stale price
    let res = s.pool.try_borrow(&s.user, &s.borrow_token, &500_000_000);
    assert!(res.is_err());
}

/// Stale oracle price must block liquidation operations
#[test]
fn test_stale_price_blocks_liquidate() {
    let s = setup();

    s.pool.supply(&s.lp, &2_000_000_000);
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);
    s.pool.borrow(&s.user, &s.borrow_token, &600_000_000);

    // Price crashes at time 1_050 to make position liquidatable
    let crash_time = INITIAL_TIMESTAMP + 50;
    s.env.ledger().set_timestamp(crash_time);
    s.oracle
        .set_price(&s.admin, &s.collateral_token, &6_000_000, &crash_time);
    assert!(s.engine.is_liquidatable(&s.user));

    // Advance ledger time past staleness threshold
    s.env.ledger().set_timestamp(crash_time + 500);

    // Liquidate should fail closed on stale price
    let res = s.engine.try_liquidate(&s.liquidator, &s.user, &500_000_000);
    assert!(res.is_err());
}

/// Price crash -> HF < 10_000 -> partial liquidate pays 8% liquidation bonus
#[test]
fn test_price_crash_partial_liquidate_pays_bonus() {
    let s = setup();

    s.pool.supply(&s.lp, &5_000_000_000);
    // User deposits 1,000 collateral tokens ($100 at initial $1.00)
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);

    // User borrows 600 borrow tokens ($60) (max LTV 70% of $100 = $70)
    s.pool.borrow(&s.user, &s.borrow_token, &600_000_000);
    assert_eq!(s.pool.get_user_debt(&s.user), 600_000_000);

    // Collateral price crashes from $1.00 (10_000_000) to $0.65 (6_500_000)
    // Collateral value = $65. Threshold (80%) value = $52.
    // HF = $52 / $60 = 0.8666 (< 10_000 bps) -> Liquidatable!
    let now = INITIAL_TIMESTAMP + 50;
    s.env.ledger().set_timestamp(now);
    s.oracle
        .set_price(&s.admin, &s.collateral_token, &6_500_000, &now);
    s.oracle
        .set_price(&s.admin, &s.borrow_token, &BORROW_PRICE, &now);

    assert!(s.engine.is_liquidatable(&s.user));

    let result = s.engine.liquidate(&s.liquidator, &s.user, &500_000_000);

    assert!(result.repaid > 0);
    // Bonus is 8% (800 bps) of repaid amount
    let expected_bonus = result.repaid * 800 / 10_000;
    assert_eq!(result.bonus, expected_bonus);
    assert_eq!(result.seized, result.repaid + result.bonus);

    // Debt decreased by repaid amount
    assert!(s.pool.get_user_debt(&s.user) < 600_000_076);
}

/// Dust leftover using shared::MIN_REMAINING_DEBT triggers full close
#[test]
fn test_dust_leftover_takes_full_close() {
    let s = setup();

    s.pool.supply(&s.lp, &2_000_000_000);
    s.vault
        .deposit(&s.user, &s.collateral_token, &2_000_000_000);

    // Small borrow near MIN_REMAINING_DEBT ($1.00 = 10_000_000)
    let borrow_amt = 15_000_000; // $1.50
    s.pool.borrow(&s.user, &s.borrow_token, &borrow_amt);

    // Collateral is 200 tokens. Set price to $0.09 (900_000).
    // Collateral value = 200 * 0.09 = $18.00 (enough to cover $1.50 * 1.08 = $1.62).
    // 80% threshold value = $14.40. Debt = $15.00.
    // HF = 14.40 / 15.00 = 0.96 (< 1.0) -> Liquidatable!
    let now = INITIAL_TIMESTAMP + 10;
    s.env.ledger().set_timestamp(now);
    s.oracle
        .set_price(&s.admin, &s.collateral_token, &90_000, &now);
    s.oracle
        .set_price(&s.admin, &s.borrow_token, &BORROW_PRICE, &now);

    assert!(s.engine.is_liquidatable(&s.user));

    let debt_before = s.pool.get_user_debt(&s.user);
    // Because remaining debt after any partial repayment would be < MIN_REMAINING_DEBT,
    // calculate_partial_repayment triggers dust full-close fallback returning full debt.
    let result = s.engine.liquidate(&s.liquidator, &s.user, &100_000_000);
    assert_eq!(result.repaid, debt_before);
    assert_eq!(s.pool.get_user_debt(&s.user), 0);
}

/// Unauthorized caller cannot seize collateral directly from vault
#[test]
fn test_unauthorized_caller_cannot_seize_directly() {
    let s = setup();

    let attacker = Address::generate(&s.env);
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);

    // Direct seize_collateral from non-engine caller must fail with Unauthorized
    let res = s
        .vault
        .try_seize_collateral(&attacker, &s.user, &s.collateral_token, &100);
    assert_eq!(res.err().unwrap().unwrap(), VaultError::Unauthorized);
}

/// Liquidating a healthy position must fail with NotLiquidatable
#[test]
fn test_healthy_position_liquidate_fails() {
    let s = setup();

    s.pool.supply(&s.lp, &2_000_000_000);
    // User deposits $100 collateral and borrows $20 -> HF = $80 / $20 = 4.0 >> 1.0 (very healthy)
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);
    s.pool.borrow(&s.user, &s.borrow_token, &200_000_000);

    assert!(!s.engine.is_liquidatable(&s.user));

    let res = s.engine.try_liquidate(&s.liquidator, &s.user, &100_000_000);
    assert_eq!(res.err().unwrap().unwrap(), EngineError::NotLiquidatable);
}
