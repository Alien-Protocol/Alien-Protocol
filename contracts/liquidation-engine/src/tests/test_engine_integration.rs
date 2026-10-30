#![cfg(test)]

use collateral_vault::{VaultContract, VaultContractClient};
use lending_pool::{PoolContract, PoolContractClient};
use shared::types::PriceData;
use shared::{HEALTHY_HF_BPS, MIN_REMAINING_DEBT};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

use crate::errors::EngineError;
use crate::{LiquidationContract, LiquidationContractClient};

/// Mock oracle serving three-field `shared::PriceData` (price, timestamp, write_timestamp)
#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> PriceData {
        env.storage()
            .persistent()
            .get(&asset)
            .expect("price not found")
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let price_data = PriceData {
            price,
            timestamp,
            write_timestamp: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&asset, &price_data);
    }
}

const COLLATERAL_PRICE: i128 = 10_000_000; // $1.00 at 7 decimals
const BORROW_PRICE: i128 = 10_000_000; // $1.00 at 7 decimals
const INITIAL_TIMESTAMP: u64 = 1_000;

#[allow(dead_code)]
pub struct EngineIntegrationSetup {
    pub env: Env,
    pub admin: Address,
    pub oracle_id: Address,
    pub oracle: MockOracleClient<'static>,
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

fn setup() -> EngineIntegrationSetup {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().set_timestamp(INITIAL_TIMESTAMP);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let lp = Address::generate(&env);

    // 1. Mock Oracle serving shared::PriceData
    let oracle_id = env.register(MockOracle, ());
    let oracle = MockOracleClient::new(&env, &oracle_id);

    // Tokens
    let collateral_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let collateral_token = collateral_contract.address();
    let collateral_admin = token::StellarAssetClient::new(&env, &collateral_token);

    let borrow_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let borrow_token = borrow_contract.address();
    let borrow_client = token::Client::new(&env, &borrow_token);
    let borrow_admin = token::StellarAssetClient::new(&env, &borrow_token);

    oracle.set_price(&collateral_token, &COLLATERAL_PRICE, &INITIAL_TIMESTAMP);
    oracle.set_price(&borrow_token, &BORROW_PRICE, &INITIAL_TIMESTAMP);

    // 2. Real Vault Contract
    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);

    // 3. Real Lending Pool Contract
    let pool_id = env.register(PoolContract, ());
    let pool = PoolContractClient::new(&env, &pool_id);

    // 4. Real Liquidation Engine Contract
    let engine_id = env.register(LiquidationContract, ());
    let engine = LiquidationContractClient::new(&env, &engine_id);

    // Wire up: vault.set_liquidation_engine is explicitly required
    vault.initialize(&admin, &pool_id, &oracle_id, &engine_id);
    pool.initialize(&admin, &vault_id, &oracle_id, &borrow_token, &800); // 8% APR
    engine.initialize(&admin, &vault_id, &pool_id, &oracle_id);
    vault.set_liquidation_engine(&engine_id);

    // Configure collateral asset in vault (7 decimals, max LTV 70%, liquidation threshold 80%)
    vault.add_supported_asset(&collateral_token);
    vault.set_asset_config(&collateral_token, &7, &7, &7000, &8000);

    // Mint balances
    collateral_admin.mint(&user, &5_000_000_000); // $500 collateral
    borrow_admin.mint(&lp, &10_000_000_000); // $1,000 pool liquidity
    borrow_admin.mint(&user, &1_000_000_000); // $100 user starting funds
    borrow_admin.mint(&liquidator, &10_000_000_000); // $1,000 liquidator funds

    EngineIntegrationSetup {
        env,
        admin,
        oracle_id,
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

/// Scenario: deposit collateral -> supply liquidity -> borrow under LTV -> crash oracle price
/// until HF < 1.0 -> liquidate -> assert debt down, liquidator profit ≈ 8%, user leftover HF is
/// either 0 (full close) or >= 10_000 bps.
#[test]
fn test_deposit_borrow_price_crash_liquidate_lifecycle() {
    let s = setup();

    // 1. LP supplies 5,000 borrow asset ($500)
    s.pool.supply(&s.lp, &5_000_000_000);
    assert_eq!(s.pool.get_available_liquidity(), 5_000_000_000);

    // 2. User deposits 1,000 collateral tokens ($100 at $1.00)
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);
    assert_eq!(s.vault.get_collateral_value(&s.user), 1_000_000_000);

    // 3. User borrows 600 borrow tokens ($60, within 70% max LTV = $70)
    s.pool.borrow(&s.user, &s.borrow_token, &600_000_000);

    // 4. Collateral price crashes from $1.00 (10_000_000) to $0.65 (6_500_000)
    // Collateral value = $65. Threshold (80%) value = $52. Debt = $60.
    // HF = $52 / $60 = 0.8666 (< 10_000 bps) -> Liquidatable!
    let crash_time = INITIAL_TIMESTAMP + 10;
    s.env.ledger().set_timestamp(crash_time);
    s.oracle
        .set_price(&s.collateral_token, &6_500_000, &crash_time);

    assert!(s.engine.is_liquidatable(&s.user));

    // Capture exact accrued debt right before liquidation
    let debt_before = s.pool.get_user_debt(&s.user);

    // 5. Liquidator liquidates position
    let result = s.engine.liquidate(&s.liquidator, &s.user, &1_000_000_000);

    // 6. Assertions: debt decreased by repaid amount
    let debt_after = s.pool.get_user_debt(&s.user);
    assert!(debt_after < debt_before);
    assert_eq!(result.repaid, debt_before - debt_after);

    // 7. Liquidator profit ≈ 8% (800 bps) of repaid debt
    let bonus_bps = (result.bonus * 10_000) / result.repaid;
    assert!((800..=801).contains(&bonus_bps));
    assert_eq!(result.seized, result.repaid + result.bonus);

    // 8. User leftover HF is either 0 (full close) or >= 10_000 bps
    let leftover_hf = s.vault.get_health_factor(&s.user);
    assert!(debt_after == 0 || leftover_hf >= HEALTHY_HF_BPS);
}

/// Acceptance criterion: Bonus paid to liquidator is 8% of repaid value.
#[test]
fn test_liquidation_bonus_is_eight_percent_of_repaid_value() {
    let s = setup();

    s.pool.supply(&s.lp, &5_000_000_000);
    s.vault
        .deposit(&s.user, &s.collateral_token, &1_000_000_000);
    s.pool.borrow(&s.user, &s.borrow_token, &600_000_000);

    let crash_time = INITIAL_TIMESTAMP + 10;
    s.env.ledger().set_timestamp(crash_time);
    s.oracle
        .set_price(&s.collateral_token, &6_500_000, &crash_time);

    assert!(s.engine.is_liquidatable(&s.user));

    let result = s.engine.liquidate(&s.liquidator, &s.user, &300_000_000);

    assert!(result.repaid > 0);
    let expected_bonus = (result.repaid * 800) / 10_000;
    assert_eq!(result.bonus, expected_bonus);
    assert_eq!(result.seized, result.repaid + result.bonus);
}

/// Second scenario: max_repay_amount below the close-factor cap still executes a partial
/// liquidation when leftover stays healthy, or full-closes when issue 12 fallback says so.
/// Assert which path ran.
#[test]
fn test_max_repay_partial_or_full_close_matches_math() {
    let s = setup();

    // -----------------------------------------------------------------------
    // Part A: Dust leftover fallback triggers full close (Issue 12 fallback)
    // -----------------------------------------------------------------------
    s.pool.supply(&s.lp, &2_000_000_000);
    // User deposits 2,000 collateral tokens ($200)
    s.vault
        .deposit(&s.user, &s.collateral_token, &2_000_000_000);

    // Small borrow near MIN_REMAINING_DEBT ($1.50 = 15_000_000)
    let small_borrow = 15_000_000;
    s.pool.borrow(&s.user, &s.borrow_token, &small_borrow);

    // Crash collateral price to $0.09 (90_000)
    // Collateral value = 200 * 0.09 = $18.00. Threshold (80%) value = $14.40.
    // Debt = $15.00. HF = 14.40 / 15.00 = 0.96 (< 1.0) -> Liquidatable!
    let now = INITIAL_TIMESTAMP + 20;
    s.env.ledger().set_timestamp(now);
    s.oracle.set_price(&s.collateral_token, &90_000, &now);

    assert!(s.engine.is_liquidatable(&s.user));

    let debt_before = s.pool.get_user_debt(&s.user);

    // Since remaining debt after any partial close would be < MIN_REMAINING_DEBT (10_000_000),
    // calculate_partial_repayment triggers dust full-close fallback (repay = full debt).
    let result = s.engine.liquidate(&s.liquidator, &s.user, &100_000_000);

    // Assert: Issue 12 full-close fallback path ran!
    assert_eq!(
        result.repaid, debt_before,
        "Path: Issue 12 full-close fallback ran"
    );
    assert_eq!(s.pool.get_user_debt(&s.user), 0);

    // -----------------------------------------------------------------------
    // Part B: Partial liquidation path leaving healthy non-dust leftover
    // -----------------------------------------------------------------------
    let user2 = Address::generate(&s.env);
    s.collateral_admin.mint(&user2, &5_000_000_000);
    s.borrow_admin.mint(&user2, &1_000_000_000);

    // Reset collateral price to $1.00
    let now2 = now + 10;
    s.env.ledger().set_timestamp(now2);
    s.oracle
        .set_price(&s.collateral_token, &COLLATERAL_PRICE, &now2);

    // User2 deposits 2,000 collateral tokens ($200) and borrows $135 (max LTV 70% allows up to $140)
    s.vault.deposit(&user2, &s.collateral_token, &2_000_000_000);
    s.pool.borrow(&user2, &s.borrow_token, &1_350_000_000);

    // Price drops moderately to $0.80 (8_000_000):
    // Collateral value = 2,000 * 0.80 = $160. Threshold (80%) = $128. Debt = $135.
    // HF = 128 / 135 = 0.9481 < 1.0 -> Liquidatable!
    // At HF = 0.9481, partial repayment R needed to restore HF to 1.05 is capped at 50% ($67.50).
    // Repaying 50% ($67.50) leaves leftover debt $67.50 and leftover threshold $69.68 -> HF = 1.0322 >= 1.0 (Healthy)!
    let crash_time2 = now2 + 10;
    s.env.ledger().set_timestamp(crash_time2);
    s.oracle
        .set_price(&s.collateral_token, &8_000_000, &crash_time2);

    assert!(s.engine.is_liquidatable(&user2));

    // Liquidate with max_repay allowing partial repayment
    let res2 = s.engine.liquidate(&s.liquidator, &user2, &700_000_000);

    // Assert: Partial liquidation path ran!
    let debt2_left = s.pool.get_user_debt(&user2);
    assert!(
        debt2_left > MIN_REMAINING_DEBT,
        "Path: Partial liquidation ran leaving non-dust debt"
    );
    assert!(res2.repaid < 1_350_000_000);
    assert!(s.vault.get_health_factor(&user2) >= HEALTHY_HF_BPS);
}

/// Healthy position cannot be liquidated end-to-end. Must revert with NotLiquidatable.
#[test]
fn test_healthy_position_cannot_be_liquidated_end_to_end() {
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
