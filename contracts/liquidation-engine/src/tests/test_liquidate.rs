#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

use super::super::*;
use crate::errors::EngineError;

// ---------------------------------------------------------------------------
// Minimal mock Vault / Pool / Oracle contracts.
//
// The production `collateral-vault` and `lending-pool` contracts have
// dependent logic (debt accrual, `repay_for`) that is not implemented yet in
// this snapshot of the repo, so the liquidation-engine's own test suite
// exercises `liquidate.rs` against small, directly-controllable stand-ins
// that expose the same function names/signatures the engine's `VaultClient`,
// `PoolClient`, and `OracleClient` invoke.
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
enum VaultKey {
    Engine,
    Position(Address),
    AssetConfig(Address),
    HealthFactor(Address),
    CollateralValue(Address),
}

#[contract]
struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn set_liquidation_engine(env: Env, engine: Address) {
        env.storage().persistent().set(&VaultKey::Engine, &engine);
    }

    pub fn set_position(env: Env, position: shared::Position) {
        env.storage()
            .persistent()
            .set(&VaultKey::Position(position.user.clone()), &position);
    }

    pub fn set_asset_config(env: Env, asset: Address, config: shared::AssetConfig) {
        env.storage()
            .persistent()
            .set(&VaultKey::AssetConfig(asset), &config);
    }

    pub fn set_health_factor(env: Env, user: Address, hf: i128) {
        env.storage()
            .persistent()
            .set(&VaultKey::HealthFactor(user), &hf);
    }

    pub fn set_collateral_value(env: Env, user: Address, value: i128) {
        env.storage()
            .persistent()
            .set(&VaultKey::CollateralValue(user), &value);
    }

    pub fn get_liquidation_engine(env: Env) -> Option<Address> {
        env.storage().persistent().get(&VaultKey::Engine)
    }

    pub fn get_position(env: Env, user: Address) -> shared::Position {
        env.storage()
            .persistent()
            .get(&VaultKey::Position(user))
            .expect("no position")
    }

    pub fn get_asset_config(env: Env, asset: Address) -> shared::AssetConfig {
        env.storage()
            .persistent()
            .get(&VaultKey::AssetConfig(asset))
            .expect("no asset config")
    }

    pub fn get_health_factor(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&VaultKey::HealthFactor(user))
            .unwrap_or(i128::MAX)
    }

    pub fn get_collateral_value(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&VaultKey::CollateralValue(user))
            .unwrap_or(0)
    }

    pub fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        _user: Address,
        asset: Address,
        amount: i128,
    ) {
        liquidation_engine.require_auth();
        token::Client::new(&env, &asset).transfer(
            &env.current_contract_address(),
            &liquidation_engine,
            &amount,
        );
    }
}

#[contracttype]
#[derive(Clone)]
enum PoolKey {
    BorrowAsset,
    Debt(Address),
    LastRepay,
}

#[contract]
struct MockPoolContract;

#[contractimpl]
impl MockPoolContract {
    pub fn set_borrow_asset(env: Env, asset: Address) {
        env.storage()
            .persistent()
            .set(&PoolKey::BorrowAsset, &asset);
    }

    pub fn set_debt(env: Env, user: Address, debt: i128) {
        env.storage().persistent().set(&PoolKey::Debt(user), &debt);
    }

    pub fn get_borrow_asset(env: Env) -> Option<Address> {
        env.storage().persistent().get(&PoolKey::BorrowAsset)
    }

    pub fn get_user_debt(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&PoolKey::Debt(user))
            .unwrap_or(0)
    }

    pub fn is_liquidatable(_env: Env, _user: Address) -> bool {
        false
    }

    pub fn repay_for(env: Env, payer: Address, user: Address, asset: Address, amount: i128) {
        payer.require_auth();

        let pool_addr = env.current_contract_address();
        token::Client::new(&env, &asset).transfer(&payer, &pool_addr, &amount);

        let current_debt: i128 = env
            .storage()
            .persistent()
            .get(&PoolKey::Debt(user.clone()))
            .unwrap_or(0);
        let new_debt = (current_debt - amount).max(0);
        env.storage()
            .persistent()
            .set(&PoolKey::Debt(user), &new_debt);

        env.storage()
            .persistent()
            .set(&PoolKey::LastRepay, &(payer, amount));
    }
}

#[contracttype]
#[derive(Clone)]
enum OracleKey {
    Price(Address),
}

#[contract]
struct MockOracleContract;

#[contractimpl]
impl MockOracleContract {
    pub fn set_price(env: Env, asset: Address, price: shared::types::PriceData) {
        env.storage()
            .persistent()
            .set(&OracleKey::Price(asset), &price);
    }

    pub fn get_price(env: Env, asset: Address) -> Option<shared::types::PriceData> {
        env.storage().persistent().get(&OracleKey::Price(asset))
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> shared::types::PriceData {
        env.storage()
            .persistent()
            .get(&OracleKey::Price(asset))
            .expect("no price")
    }
}

// ---------------------------------------------------------------------------
// Fixture: a scaled-up version of the numeric fixture used in test_math.rs
// (collateral = 10_000, debt = 8_100, lt = 8_000 bps), multiplied by 1e6 so
// remaining debt stays well above `MIN_REMAINING_DEBT`
// (`PROTOCOL_QUOTE_PRECISION` = 10_000_000) and the dust fallback doesn't
// kick in. Collateral price is set to exactly `PROTOCOL_QUOTE_PRECISION`
// with matching 7-decimal token/oracle precision, so quote-unit values and
// collateral token units are 1:1 and easy to assert on.
// ---------------------------------------------------------------------------
const COLLATERAL_VALUE: i128 = 10_000_000_000;
const DEBT: i128 = 8_100_000_000;
const LT_BPS: u32 = 8_000;
const HEALTH_FACTOR_UNHEALTHY: i128 = 9_876; // floor(10_000_000_000 * 8_000 / 8_100_000_000)
const HEALTH_FACTOR_HEALTHY: i128 = 10_500;
// Exact result of `calculate_partial_repayment`'s ceil_div-based formula for
// this fixture; ceil_div does not scale linearly with the underlying
// fixture, so this is the precise value rather than a naive x1e6 scale-up of
// test_math.rs's 3_856 fixture result.
const EXPECTED_PARTIAL_REPAY: i128 = 3_855_932_204;
const COLLATERAL_BALANCE: i128 = 6_000_000_000;

fn expected_bonus(repay: i128) -> i128 {
    shared::ceil_div(repay * 800, 10_000).unwrap()
}

#[allow(dead_code)]
struct Env_ {
    env: Env,
    engine: LiquidationContractClient<'static>,
    engine_addr: Address,
    vault: MockVaultContractClient<'static>,
    vault_addr: Address,
    pool: MockPoolContractClient<'static>,
    pool_addr: Address,
    oracle: MockOracleContractClient<'static>,
    oracle_addr: Address,
    admin: Address,
    liquidator: Address,
    user: Address,
    borrow_asset: Address,
    borrow_token: token::Client<'static>,
    collateral_asset: Address,
    collateral_token: token::Client<'static>,
}

fn setup() -> Env_ {
    let env = Env::default();
    // The engine authorizes nested contract-to-contract calls it initiates
    // (e.g. the pool pulling repayment tokens from the engine's own balance),
    // which are not root-invocation arguments, so plain `mock_all_auths`
    // isn't enough here.
    env.mock_all_auths_allowing_non_root_auth();

    let engine_id = env.register(LiquidationContract, ());
    let engine = LiquidationContractClient::new(&env, &engine_id);

    let vault_id = env.register(MockVaultContract, ());
    let vault = MockVaultContractClient::new(&env, &vault_id);

    let pool_id = env.register(MockPoolContract, ());
    let pool = MockPoolContractClient::new(&env, &pool_id);

    let oracle_id = env.register(MockOracleContract, ());
    let oracle = MockOracleContractClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let user = Address::generate(&env);

    env.as_contract(&engine_id, || {
        crate::storage::set_admin(&env, &admin);
        crate::storage::set_vault(&env, &vault_id);
        crate::storage::set_pool(&env, &pool_id);
        crate::storage::set_oracle(&env, &oracle_id);
    });

    vault.set_liquidation_engine(&engine_id);

    let borrow_token_admin = Address::generate(&env);
    let borrow_asset = env
        .register_stellar_asset_contract_v2(borrow_token_admin)
        .address();
    let borrow_token = token::Client::new(&env, &borrow_asset);
    let borrow_token_admin_client = token::StellarAssetClient::new(&env, &borrow_asset);
    borrow_token_admin_client.mint(&liquidator, &1_000_000_000_000);
    pool.set_borrow_asset(&borrow_asset);

    let collateral_token_admin = Address::generate(&env);
    let collateral_asset = env
        .register_stellar_asset_contract_v2(collateral_token_admin)
        .address();
    let collateral_token = token::Client::new(&env, &collateral_asset);
    let collateral_token_admin_client = token::StellarAssetClient::new(&env, &collateral_asset);
    collateral_token_admin_client.mint(&vault_id, &COLLATERAL_BALANCE);

    vault.set_asset_config(
        &collateral_asset,
        &shared::AssetConfig {
            token_decimals: 7,
            oracle_price_decimals: 7,
            max_ltv_bps: 7_000,
            liquidation_threshold_bps: LT_BPS,
        },
    );
    oracle.set_price(
        &collateral_asset,
        &shared::types::PriceData {
            price: shared::PROTOCOL_QUOTE_PRECISION,
            timestamp: 1,
            write_timestamp: 1,
        },
    );
    vault.set_position(&shared::Position {
        user: user.clone(),
        collateral: soroban_sdk::vec![
            &env,
            shared::CollateralAsset {
                asset: collateral_asset.clone(),
                amount: COLLATERAL_BALANCE,
            },
        ],
    });

    vault.set_collateral_value(&user, &COLLATERAL_VALUE);
    pool.set_debt(&user, &DEBT);
    vault.set_health_factor(&user, &HEALTH_FACTOR_UNHEALTHY);

    Env_ {
        env,
        engine,
        engine_addr: engine_id,
        vault,
        vault_addr: vault_id,
        pool,
        pool_addr: pool_id,
        oracle,
        oracle_addr: oracle_id,
        admin,
        liquidator,
        user,
        borrow_asset,
        borrow_token,
        collateral_asset,
        collateral_token,
    }
}

// ---------------------------------------------------------------------------
// is_liquidatable
// ---------------------------------------------------------------------------

#[test]
fn test_is_liquidatable_false_when_healthy_or_no_debt() {
    let ctx = setup();

    // Unhealthy + debt -> liquidatable.
    assert!(ctx.engine.is_liquidatable(&ctx.user));

    // No debt -> never liquidatable, regardless of health factor.
    ctx.pool.set_debt(&ctx.user, &0);
    assert!(!ctx.engine.is_liquidatable(&ctx.user));

    // Debt present but healthy -> not liquidatable.
    ctx.pool.set_debt(&ctx.user, &DEBT);
    ctx.vault
        .set_health_factor(&ctx.user, &HEALTH_FACTOR_HEALTHY);
    assert!(!ctx.engine.is_liquidatable(&ctx.user));
}

// ---------------------------------------------------------------------------
// liquidate: happy path
// ---------------------------------------------------------------------------

#[test]
fn test_liquidate_success_repays_seizes_and_pays_bonus() {
    let ctx = setup();

    let result = ctx.engine.liquidate(&ctx.liquidator, &ctx.user, &DEBT);

    assert_eq!(result.user, ctx.user);
    assert_eq!(result.repaid, EXPECTED_PARTIAL_REPAY);

    let bonus = expected_bonus(EXPECTED_PARTIAL_REPAY);
    assert_eq!(result.bonus, bonus);
    assert_eq!(result.seized, EXPECTED_PARTIAL_REPAY + bonus);

    // Liquidator paid `repaid` of the borrow asset and received the seized
    // collateral (1:1 token units, given the fixture's price/decimals).
    assert_eq!(
        ctx.borrow_token.balance(&ctx.liquidator),
        1_000_000_000_000 - EXPECTED_PARTIAL_REPAY
    );
    assert_eq!(
        ctx.collateral_token.balance(&ctx.liquidator),
        EXPECTED_PARTIAL_REPAY + bonus
    );
    assert_eq!(
        ctx.collateral_token.balance(&ctx.vault_addr),
        COLLATERAL_BALANCE - (EXPECTED_PARTIAL_REPAY + bonus)
    );

    // Debt fell by the repaid amount (mock pool tracks this via repay_for).
    assert_eq!(
        ctx.pool.get_user_debt(&ctx.user),
        DEBT - EXPECTED_PARTIAL_REPAY
    );
}

#[test]
fn test_liquidate_respects_max_repay_amount() {
    let ctx = setup();

    let max_repay = 1_000_000_000i128;
    assert!(max_repay < EXPECTED_PARTIAL_REPAY);

    let result = ctx.engine.liquidate(&ctx.liquidator, &ctx.user, &max_repay);

    assert_eq!(result.repaid, max_repay);
    let bonus = expected_bonus(max_repay);
    assert_eq!(result.bonus, bonus);
    assert_eq!(result.seized, max_repay + bonus);
    assert_eq!(
        ctx.borrow_token.balance(&ctx.liquidator),
        1_000_000_000_000 - max_repay
    );
    assert_eq!(
        ctx.collateral_token.balance(&ctx.liquidator),
        max_repay + bonus
    );
}

// ---------------------------------------------------------------------------
// liquidate: rejections
// ---------------------------------------------------------------------------

#[test]
fn test_liquidate_rejects_healthy_user() {
    let ctx = setup();
    ctx.vault
        .set_health_factor(&ctx.user, &HEALTH_FACTOR_HEALTHY);

    let res = ctx.engine.try_liquidate(&ctx.liquidator, &ctx.user, &DEBT);

    assert_eq!(res, Err(Ok(EngineError::NotLiquidatable)));
}

#[test]
fn test_liquidate_rejects_self_liquidation() {
    let ctx = setup();

    let res = ctx.engine.try_liquidate(&ctx.user, &ctx.user, &DEBT);

    assert_eq!(res, Err(Ok(EngineError::Unauthorized)));
}

#[test]
fn test_liquidate_rejects_non_positive_max_repay() {
    let ctx = setup();

    let res_zero = ctx.engine.try_liquidate(&ctx.liquidator, &ctx.user, &0);
    assert_eq!(res_zero, Err(Ok(EngineError::InvalidAmount)));

    let res_negative = ctx.engine.try_liquidate(&ctx.liquidator, &ctx.user, &-1);
    assert_eq!(res_negative, Err(Ok(EngineError::InvalidAmount)));
}

#[test]
fn test_liquidate_requires_liquidator_auth() {
    // No `mock_all_auths` / explicit auth for the liquidator: the very first
    // `liquidator.require_auth()` in `liquidate` must fail the call.
    let env = Env::default();

    let engine_id = env.register(LiquidationContract, ());
    let engine = LiquidationContractClient::new(&env, &engine_id);

    let liquidator = Address::generate(&env);
    let user = Address::generate(&env);

    let res = engine.try_liquidate(&liquidator, &user, &1_000);
    assert!(res.is_err());
}

#[test]
fn test_liquidate_fails_if_vault_engine_mismatch() {
    let ctx = setup();

    let other_engine = Address::generate(&ctx.env);
    ctx.vault.set_liquidation_engine(&other_engine);

    let res = ctx.engine.try_liquidate(&ctx.liquidator, &ctx.user, &DEBT);

    assert_eq!(res, Err(Ok(EngineError::Unauthorized)));
}
