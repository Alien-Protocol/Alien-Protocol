#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

const ORACLE_STALE_THRESHOLD: u64 = 300;

// ---------------------------------------------------------------------------
// Mocks (same pattern as test_withdraw.rs)
// ---------------------------------------------------------------------------

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
        let price_data = types::PriceData { price, timestamp };
        env.storage().persistent().set(&asset, &price_data);
    }
}

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,                            // user
    token::Client<'static>,             // token_client
    token::StellarAssetClient<'static>, // token_admin_client
    MockLendingPoolClient<'static>,
    MockOracleClient<'static>,
    Address, // token_id
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

    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    let pool_id = env.register(MockLendingPool, ());
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);
    client.set_pool(&pool_id);

    // Default price: $1.00 encoded as 10_000_000 (7 decimal places).
    oracle_client.set_price(&token_id, &10_000_000, &1000);

    (
        env,
        client,
        user,
        token_client,
        token_admin_client,
        pool_client,
        oracle_client,
        token_id,
    )
}

/// Convenience: standard valid risk params — 80% LTV, 85% liquidation threshold, 5% bonus.
fn default_risk_params() -> types::AssetRiskParams {
    types::AssetRiskParams {
        ltv_bps: 8_000,
        liquidation_threshold_bps: 8_500,
        liquidation_bonus_bps: 500,
    }
}

// ---------------------------------------------------------------------------
// Validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_set_risk_params_success() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    client.set_risk_params(&token_id, &default_risk_params());

    let stored = client.get_risk_params(&token_id);
    assert!(stored.is_some());
    let p = stored.unwrap();
    assert_eq!(p.ltv_bps, 8_000);
    assert_eq!(p.liquidation_threshold_bps, 8_500);
    assert_eq!(p.liquidation_bonus_bps, 500);
}

#[test]
fn test_set_risk_params_ltv_above_threshold_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    // ltv_bps > liquidation_threshold_bps — must fail.
    let bad = types::AssetRiskParams {
        ltv_bps: 9_000,
        liquidation_threshold_bps: 8_500,
        liquidation_bonus_bps: 500,
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_ltv_at_threshold_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    // ltv_bps == liquidation_threshold_bps — must also fail (strictly less).
    let bad = types::AssetRiskParams {
        ltv_bps: 8_500,
        liquidation_threshold_bps: 8_500,
        liquidation_bonus_bps: 500,
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_ltv_below_min_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let bad = types::AssetRiskParams {
        ltv_bps: 99, // below MIN_LTV_BPS = 100
        liquidation_threshold_bps: 8_500,
        liquidation_bonus_bps: 500,
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_ltv_above_max_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let bad = types::AssetRiskParams {
        ltv_bps: 9_901, // above MAX_LTV_BPS = 9_900
        liquidation_threshold_bps: 9_999,
        liquidation_bonus_bps: 500,
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_threshold_below_min_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let bad = types::AssetRiskParams {
        ltv_bps: 100,
        liquidation_threshold_bps: 99, // below MIN_LIQ_THRESHOLD_BPS = 100, also <= ltv
        liquidation_bonus_bps: 500,
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_threshold_above_max_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let bad = types::AssetRiskParams {
        ltv_bps: 8_000,
        liquidation_threshold_bps: 10_001, // above MAX_LIQ_THRESHOLD_BPS = 10_000
        liquidation_bonus_bps: 500,
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_bonus_above_max_fails() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let bad = types::AssetRiskParams {
        ltv_bps: 8_000,
        liquidation_threshold_bps: 8_500,
        liquidation_bonus_bps: 5_001, // above MAX_LIQ_BONUS_BPS = 5_000
    };
    let res = client.try_set_risk_params(&token_id, &bad);
    assert!(res.is_err());
}

#[test]
fn test_set_risk_params_non_admin_fails() {
    let env = Env::default();
    // No mock_all_auths — admin require_auth will reject any non-admin caller.
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let token_id = Address::generate(&env);
    let res = client.try_set_risk_params(&token_id, &default_risk_params());
    assert!(res.is_err());
}

#[test]
fn test_get_risk_params_returns_stored_value() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    assert!(client.get_risk_params(&token_id).is_none());

    client.set_risk_params(&token_id, &default_risk_params());

    let p = client.get_risk_params(&token_id).unwrap();
    assert_eq!(p, default_risk_params());
}

// ---------------------------------------------------------------------------
// Withdrawal safety — single asset
// ---------------------------------------------------------------------------

/// Deposit 1000 tokens at $1.00. Risk: 80% LTV, 85% liq threshold.
/// liquidation_value = 1000 * 0.85 = $850.
/// Debt = $500.  Withdraw 400 tokens → remaining liq_value = 600 * 0.85 = $510 >= $500. Safe.
#[test]
fn test_withdrawal_safe_single_asset_within_threshold() {
    let (_env, client, user, _tc, token_admin, pool, oracle, token_id) = setup_env();

    // $1.00 per token.
    oracle.set_price(&token_id, &10_000_000, &1000);
    client.set_risk_params(&token_id, &default_risk_params());

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &1000);

    pool.set_user_debt(&500);

    // Withdrawing 400 → 600 remaining, liq_value = 600*0.85 = 510 >= 500. ✓
    client.withdraw(&user, &token_id, &400);
    assert_eq!(client.get_position_balance(&user, &token_id), 600);
}

/// Withdraw 413 tokens → remaining liq_value = 587 * 0.85 = 499 (integer) < 500. Unsafe.
#[test]
fn test_withdrawal_unsafe_single_asset_below_threshold() {
    let (_env, client, user, _tc, token_admin, pool, oracle, token_id) = setup_env();

    oracle.set_price(&token_id, &10_000_000, &1000);
    client.set_risk_params(&token_id, &default_risk_params()); // 85% threshold

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &1000);

    pool.set_user_debt(&500);

    // 1000 tokens, liq_value = 1000 * 8500/10000 = 850.
    // Withdrawing 413: withdrawn_lv = 413 * 8500/10000 = 351.
    // remaining_lv = 850 - 351 = 499 < 500 (debt). Must be blocked.
    let res = client.try_withdraw(&user, &token_id, &413);
    assert!(
        res.is_err(),
        "withdrawal that drops liq_value below debt must be rejected"
    );
}

/// With no debt the withdrawal should always succeed regardless of ratio.
#[test]
fn test_withdrawal_safe_no_debt() {
    let (_env, client, user, _tc, token_admin, _pool, oracle, token_id) = setup_env();

    oracle.set_price(&token_id, &10_000_000, &1000);
    client.set_risk_params(&token_id, &default_risk_params());

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &1000);

    // debt = 0 (default mock returns 0)
    client.withdraw(&user, &token_id, &1000);
    assert_eq!(client.get_position_balance(&user, &token_id), 0);
}

// ---------------------------------------------------------------------------
// Multi-asset
// ---------------------------------------------------------------------------

/// Two assets, each with their own risk params.
/// Asset A: 500 tokens @ $1.00, 80% LTV, 85% threshold
///   → borrow_power = 500 * 0.80 = $400, liq_value = 500 * 0.85 = $425
/// Asset B: 200 tokens @ $2.00, 70% LTV, 75% threshold
///   → USD value = 400, borrow_power = 400 * 0.70 = $280, liq_value = 400 * 0.75 = $300
/// Total liq_value = $725, total borrow_power = $680.
/// Verify via is_withdrawal_safe with debt = $400 and trying to remove all of Asset A.
/// After removing A: liq_value = $300 >= $400? No → unsafe.
#[test]
fn test_multi_asset_borrowing_power_correct() {
    let (env, client, user, _tc, token_admin_a, pool, oracle, token_id_a) = setup_env();

    // Register a second token.
    let token_admin_b_addr = Address::generate(&env);
    let token_b_contract = env.register_stellar_asset_contract_v2(token_admin_b_addr);
    let token_id_b = token_b_contract.address();
    let token_admin_b = token::StellarAssetClient::new(&env, &token_id_b);
    client.add_supported_asset(&token_id_b);

    // Prices.
    oracle.set_price(&token_id_a, &10_000_000, &1000); // $1.00
    oracle.set_price(&token_id_b, &20_000_000, &1000); // $2.00

    // Risk params.
    client.set_risk_params(
        &token_id_a,
        &types::AssetRiskParams {
            ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
            liquidation_bonus_bps: 500,
        },
    );
    client.set_risk_params(
        &token_id_b,
        &types::AssetRiskParams {
            ltv_bps: 7_000,
            liquidation_threshold_bps: 7_500,
            liquidation_bonus_bps: 300,
        },
    );

    token_admin_a.mint(&user, &500);
    client.deposit(&user, &token_id_a, &500);

    token_admin_b.mint(&user, &200);
    client.deposit(&user, &token_id_b, &200);

    // With debt = $400, removing all of A (liq contribution = $425) →
    // remaining liq_value = $300 < $400 → unsafe.
    pool.set_user_debt(&400);
    let res = client.try_withdraw(&user, &token_id_a, &500);
    assert!(
        res.is_err(),
        "removing all of asset A should leave liq_value below debt"
    );
}

/// With the same two-asset setup, a small withdrawal from Asset A is safe.
/// Withdraw 100 of A → liq_value of A contribution drops by 100*0.85=$85
/// New liq_value = $725 - $85 = $640 >= $400. Safe.
#[test]
fn test_multi_asset_withdrawal_safe() {
    let (env, client, user, _tc, token_admin_a, pool, oracle, token_id_a) = setup_env();

    let token_admin_b_addr = Address::generate(&env);
    let token_b_contract = env.register_stellar_asset_contract_v2(token_admin_b_addr);
    let token_id_b = token_b_contract.address();
    let token_admin_b = token::StellarAssetClient::new(&env, &token_id_b);
    client.add_supported_asset(&token_id_b);

    oracle.set_price(&token_id_a, &10_000_000, &1000);
    oracle.set_price(&token_id_b, &20_000_000, &1000);

    client.set_risk_params(
        &token_id_a,
        &types::AssetRiskParams {
            ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
            liquidation_bonus_bps: 500,
        },
    );
    client.set_risk_params(
        &token_id_b,
        &types::AssetRiskParams {
            ltv_bps: 7_000,
            liquidation_threshold_bps: 7_500,
            liquidation_bonus_bps: 300,
        },
    );

    token_admin_a.mint(&user, &500);
    client.deposit(&user, &token_id_a, &500);

    token_admin_b.mint(&user, &200);
    client.deposit(&user, &token_id_b, &200);

    pool.set_user_debt(&400);

    // Withdraw 100 of A → remaining liq_value ≈ $640 >= $400. Safe.
    client.withdraw(&user, &token_id_a, &100);
    assert_eq!(client.get_position_balance(&user, &token_id_a), 400);
}

/// Same two-asset setup, but a large withdrawal from A makes it unsafe.
/// Withdraw 400 of A → A liq contribution = 400*0.85 = $340 removed
/// Remaining liq_value = $725 - $340 = $385 < $400. Unsafe.
#[test]
fn test_multi_asset_withdrawal_unsafe() {
    let (env, client, user, _tc, token_admin_a, pool, oracle, token_id_a) = setup_env();

    let token_admin_b_addr = Address::generate(&env);
    let token_b_contract = env.register_stellar_asset_contract_v2(token_admin_b_addr);
    let token_id_b = token_b_contract.address();
    let token_admin_b = token::StellarAssetClient::new(&env, &token_id_b);
    client.add_supported_asset(&token_id_b);

    oracle.set_price(&token_id_a, &10_000_000, &1000);
    oracle.set_price(&token_id_b, &20_000_000, &1000);

    client.set_risk_params(
        &token_id_a,
        &types::AssetRiskParams {
            ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
            liquidation_bonus_bps: 500,
        },
    );
    client.set_risk_params(
        &token_id_b,
        &types::AssetRiskParams {
            ltv_bps: 7_000,
            liquidation_threshold_bps: 7_500,
            liquidation_bonus_bps: 300,
        },
    );

    token_admin_a.mint(&user, &500);
    client.deposit(&user, &token_id_a, &500);

    token_admin_b.mint(&user, &200);
    client.deposit(&user, &token_id_b, &200);

    pool.set_user_debt(&400);

    // Withdraw 400 of A → remaining liq_value = $385 < $400. Unsafe.
    let res = client.try_withdraw(&user, &token_id_a, &400);
    assert!(
        res.is_err(),
        "withdrawal that drops multi-asset liq_value below debt must be rejected"
    );
}

// ---------------------------------------------------------------------------
// Risk-update propagation
// ---------------------------------------------------------------------------

/// Tightening the liquidation threshold on the asset immediately makes
/// a position that was previously safe become unsafe for withdrawal.
///
/// Setup: 1000 tokens @ $1.00, debt = $500.
/// Original: liq_threshold = 85% → liq_value = $850 >= $500. Safe to withdraw 400.
/// After tightening to 51%: liq_value = $510 - but withdrawing 400 →
/// remaining = 600 * 0.51 = 306 < 500. Unsafe.
#[test]
fn test_risk_update_affects_health_immediately() {
    let (_env, client, user, _tc, token_admin, pool, oracle, token_id) = setup_env();

    oracle.set_price(&token_id, &10_000_000, &1000);
    client.set_risk_params(&token_id, &default_risk_params()); // 85% threshold

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &1000);
    pool.set_user_debt(&500);

    // Withdrawal of 400 is currently safe (verified below).
    // We do NOT actually withdraw here — just confirm it would succeed.
    let safe = client.is_withdrawal_safe(&user, &token_id, &400);
    assert!(safe, "expected safe with 85% threshold");

    // Now tighten threshold to 51% (LTV 50%, threshold 51%).
    client.set_risk_params(
        &token_id,
        &types::AssetRiskParams {
            ltv_bps: 5_000,
            liquidation_threshold_bps: 5_100,
            liquidation_bonus_bps: 0,
        },
    );

    // Same withdrawal — remaining 600 * 0.51 = 306 < 500. Now unsafe.
    let safe_after = client.is_withdrawal_safe(&user, &token_id, &400);
    assert!(
        !safe_after,
        "expected unsafe after tightening liquidation threshold"
    );
}

// ---------------------------------------------------------------------------
// Boundary tests
// ---------------------------------------------------------------------------

/// ltv at MAX_LTV_BPS (9900) with threshold at 9901 is valid.
#[test]
fn test_set_risk_params_ltv_max_boundary() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let params = types::AssetRiskParams {
        ltv_bps: 9_900,                   // MAX_LTV_BPS
        liquidation_threshold_bps: 9_901, // just above LTV, within MAX (10_000)
        liquidation_bonus_bps: 0,
    };
    client.set_risk_params(&token_id, &params);
    assert_eq!(client.get_risk_params(&token_id).unwrap().ltv_bps, 9_900);
}

/// A liquidation bonus of 0 is explicitly valid.
#[test]
fn test_set_risk_params_bonus_zero_allowed() {
    let (_env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    let params = types::AssetRiskParams {
        ltv_bps: 8_000,
        liquidation_threshold_bps: 8_500,
        liquidation_bonus_bps: 0, // MIN_LIQ_BONUS_BPS
    };
    client.set_risk_params(&token_id, &params);
    assert_eq!(
        client
            .get_risk_params(&token_id)
            .unwrap()
            .liquidation_bonus_bps,
        0
    );
}

/// Updating risk params emits a RiskParamsUpdated event with old and new values.
#[test]
fn test_set_risk_params_emits_event() {
    let (env, client, _user, _tc, _ta, _pool, _oracle, token_id) = setup_env();

    client.set_risk_params(&token_id, &default_risk_params());

    let updated = types::AssetRiskParams {
        ltv_bps: 7_000,
        liquidation_threshold_bps: 7_500,
        liquidation_bonus_bps: 300,
    };
    client.set_risk_params(&token_id, &updated);

    // Verify at least one event was emitted from the vault contract.
    let all_events = env.events().all();
    let vault_event_count = all_events.iter().filter(|e| e.0 == client.address).count();
    assert!(
        vault_event_count > 0,
        "expected at least one event from vault contract"
    );
}
