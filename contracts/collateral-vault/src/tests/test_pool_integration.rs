#![cfg(test)]

//! Vault + pool composition tests (issue #624).
//!
//! These tests wire up the real `VaultContract` and the real `PoolContract`
//! (from the `lending-pool` crate) as separate contract instances in the same
//! `Env`, so borrow/repay calls actually cross-invoke `get_collateral_value`
//! and `get_position` on the vault the way they would on a live network. Only
//! the price oracle is mocked. `liquidation-engine` is untouched, and
//! `seize_collateral` / `execute_seize` are not exercised here.

use super::super::*;
use lending_pool::{PoolContract, PoolContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct MockOracle;

#[contractimpl]
impl MockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        match env.storage().persistent().get(&asset) {
            Some(price_data) => price_data,
            None => panic!("price not found"),
        }
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

/// 8% APR, matching `shared::V1_BORROW_APR_BPS`.
const INTEREST_RATE_BPS: u32 = 800;
/// $1.00 per token at the default 7-decimal oracle/token precision, so
/// `collateral_value` equals the raw deposited amount exactly.
const COLLATERAL_PRICE: i128 = 10_000_000;
/// Collateral value of $1,000 at `COLLATERAL_PRICE`.
const COLLATERAL_DEPOSIT: i128 = 1_000_000_000;
/// Borrow-side liquidity available to the pool.
const LIQUIDITY_SUPPLY: i128 = 2_000_000_000;
/// Well within the 70% default max-LTV limit ($700 on $1,000 collateral).
const BORROW_AMOUNT: i128 = 500_000_000;
/// Pre-existing borrow-asset funds minted directly to the user, so they can
/// cover accrued interest on repay without depending on the borrowed
/// principal itself.
const USER_STARTING_FUNDS: i128 = 50_000_000;
/// Half of `shared::SECONDS_PER_YEAR`, chosen so the linear-interest formula
/// (`principal * rate_bps / 10_000 * elapsed / SECONDS_PER_YEAR`) divides
/// evenly.
const HALF_YEAR_SECONDS: u64 = shared::SECONDS_PER_YEAR / 2;

struct TestSetup {
    env: Env,
    vault: VaultContractClient<'static>,
    pool: PoolContractClient<'static>,
    collateral_token: Address,
    collateral_admin: token::StellarAssetClient<'static>,
    borrow_token: Address,
    borrow_client: token::Client<'static>,
    borrow_admin: token::StellarAssetClient<'static>,
    user: Address,
    lp: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let lp = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    let oracle_id = env.register(MockOracle, ());
    let oracle = MockOracleClient::new(&env, &oracle_id);

    let vault_id = env.register(VaultContract, ());
    let vault = VaultContractClient::new(&env, &vault_id);

    let pool_id = env.register(PoolContract, ());
    let pool = PoolContractClient::new(&env, &pool_id);

    let collateral_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let collateral_token = collateral_contract.address();
    let collateral_admin = token::StellarAssetClient::new(&env, &collateral_token);

    let borrow_contract = env.register_stellar_asset_contract_v2(Address::generate(&env));
    let borrow_token = borrow_contract.address();
    let borrow_client = token::Client::new(&env, &borrow_token);
    let borrow_admin = token::StellarAssetClient::new(&env, &borrow_token);

    vault.initialize(&admin, &pool_id, &oracle_id, &liquidation_engine);
    pool.initialize(
        &admin,
        &vault_id,
        &oracle_id,
        &borrow_token,
        &INTEREST_RATE_BPS,
    );

    vault.add_supported_asset(&collateral_token);
    oracle.set_price(&collateral_token, &COLLATERAL_PRICE, &1_000);

    TestSetup {
        env,
        vault,
        pool,
        collateral_token,
        collateral_admin,
        borrow_token,
        borrow_client,
        borrow_admin,
        user,
        lp,
    }
}

/// Full composition path: a user deposits collateral into the vault, a
/// liquidity provider supplies the pool, the user borrows against their
/// vault collateral, interest accrues, the user repays interest then
/// principal in full, and finally withdraws their collateral back out of the
/// vault. Every step is a real cross-contract call between `VaultContract`
/// and `PoolContract`.
#[test]
fn test_deposit_supply_borrow_repay_lifecycle() {
    let TestSetup {
        env,
        vault,
        pool,
        collateral_token,
        collateral_admin,
        borrow_token,
        borrow_client,
        borrow_admin,
        user,
        lp,
    } = setup();

    // -- deposit --------------------------------------------------------
    collateral_admin.mint(&user, &COLLATERAL_DEPOSIT);
    borrow_admin.mint(&user, &USER_STARTING_FUNDS);

    vault.deposit(&user, &collateral_token, &COLLATERAL_DEPOSIT);
    assert_eq!(
        vault.get_position_balance(&user, &collateral_token),
        COLLATERAL_DEPOSIT
    );
    assert_eq!(vault.get_collateral_value(&user), COLLATERAL_DEPOSIT);

    // -- supply -----------------------------------------------------------
    borrow_admin.mint(&lp, &LIQUIDITY_SUPPLY);
    pool.supply(&lp, &LIQUIDITY_SUPPLY);
    assert_eq!(pool.get_available_liquidity(), LIQUIDITY_SUPPLY);

    // -- borrow -----------------------------------------------------------
    pool.borrow(&user, &borrow_token, &BORROW_AMOUNT);

    assert_eq!(
        borrow_client.balance(&user),
        USER_STARTING_FUNDS + BORROW_AMOUNT
    );
    assert_eq!(pool.get_user_debt(&user), BORROW_AMOUNT);
    assert_eq!(
        pool.get_available_liquidity(),
        LIQUIDITY_SUPPLY - BORROW_AMOUNT
    );

    // -- interest accrues over time -----------------------------------
    env.ledger().set_timestamp(1_000 + HALF_YEAR_SECONDS);

    // principal * rate_bps / 10_000 * elapsed / SECONDS_PER_YEAR
    //   = 500_000_000 * 800 / 10_000 * 0.5 = 20_000_000
    let expected_interest: i128 = 20_000_000;
    assert_eq!(pool.get_user_debt(&user), BORROW_AMOUNT + expected_interest);

    // -- repay: interest first -------------------------------------------
    pool.repay(&user, &borrow_token, &expected_interest);
    assert_eq!(pool.get_user_debt(&user), BORROW_AMOUNT);
    assert_eq!(
        borrow_client.balance(&user),
        USER_STARTING_FUNDS + BORROW_AMOUNT - expected_interest
    );
    // Only principal moves TotalBorrowed/available liquidity, so paying
    // interest alone must not change available liquidity.
    assert_eq!(
        pool.get_available_liquidity(),
        LIQUIDITY_SUPPLY - BORROW_AMOUNT
    );

    // -- repay: remaining principal in full --------------------------
    pool.repay(&user, &borrow_token, &BORROW_AMOUNT);
    assert_eq!(pool.get_user_debt(&user), 0);
    assert_eq!(pool.get_available_liquidity(), LIQUIDITY_SUPPLY);
    assert_eq!(
        borrow_client.balance(&user),
        USER_STARTING_FUNDS - expected_interest
    );

    // -- withdraw: debt is fully cleared, so full collateral unlocks ---
    vault.withdraw(&user, &collateral_token, &COLLATERAL_DEPOSIT);
    assert_eq!(vault.get_position_balance(&user, &collateral_token), 0);
    assert!(!vault.get_position_index().contains(&user));

    // -- the LP's liquidity is intact and can be withdrawn too ------------
    pool.withdraw_liquidity(&lp, &LIQUIDITY_SUPPLY);
    assert_eq!(borrow_client.balance(&lp), LIQUIDITY_SUPPLY);
    assert_eq!(pool.get_total_supply(), 0);
}

/// Borrowing up to (but not beyond) the vault-derived LTV limit succeeds;
/// the same amount again, with no remaining headroom, fails.
#[test]
fn test_borrow_under_ltv_limit_succeeds() {
    let TestSetup {
        vault,
        pool,
        collateral_token,
        collateral_admin,
        borrow_token,
        borrow_client,
        borrow_admin,
        user,
        lp,
        ..
    } = setup();

    collateral_admin.mint(&user, &COLLATERAL_DEPOSIT);
    vault.deposit(&user, &collateral_token, &COLLATERAL_DEPOSIT);

    borrow_admin.mint(&lp, &LIQUIDITY_SUPPLY);
    pool.supply(&lp, &LIQUIDITY_SUPPLY);

    // 70% max-LTV (the default asset config) of a $1,000 collateral value.
    let limit = pool.calculate_limit(&user);
    assert_eq!(limit, 700_000_000);

    pool.borrow(&user, &borrow_token, &limit);
    assert_eq!(pool.get_user_debt(&user), limit);
    assert_eq!(borrow_client.balance(&user), limit);

    // No LTV headroom remains, so even a minimal additional borrow fails.
    let res = pool.try_borrow(&user, &borrow_token, &1);
    assert!(res.is_err());
}
