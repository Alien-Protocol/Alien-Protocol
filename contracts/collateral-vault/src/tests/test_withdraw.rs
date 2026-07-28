#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
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
        let price_data = types::PriceData { price, timestamp };
        env.storage().persistent().set(&asset, &price_data);
    }
}

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
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

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    let pool_id = env.register(MockLendingPool, ());
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);
    client.set_pool(&pool_id);

    // Default price: 1 token = 100USD (7 decimals)
    oracle_client.set_price(&token_id, &1_000_000_000, &1000);

    (
        env,
        client,
        admin,
        user,
        token_client,
        token_admin_client,
        pool_client,
        oracle_client,
        token_id,
    )
}

#[test]
fn test_withdraw_success() {
    let (_env, client, _admin, user, token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.withdraw(&user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert_eq!(token_client.balance(&user), 1000);
}

#[test]
fn test_withdraw_partial() {
    let (_env, client, _admin, user, token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.withdraw(&user, &token_id, &200);

    assert_eq!(client.get_position_balance(&user, &token_id), 300);
    assert_eq!(token_client.balance(&user), 700);
}

#[test]
fn test_withdraw_clears_position_on_zero() {
    let (_env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));

    client.withdraw(&user, &token_id, &500);

    assert!(!client.get_position_index().contains(&user));
}

#[test]
fn test_withdraw_exceeds_balance_fails() {
    let (_env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_withdraw(&user, &token_id, &600);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let (_env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_withdraw(&user, &token_id, &0);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_no_position_fails() {
    let (_env, client, _admin, user, _token_client, _token_admin, _pool, _oracle, token_id) =
        setup_env();

    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_when_paused_fails() {
    let (_env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause();

    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_without_auth_fails() {
    let env = Env::default();
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    let token = Address::generate(&env);

    // No mock_all_auths - withdraw should fail auth check immediately
    let res = client.try_withdraw(&user, &token, &100);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_collateral_ratio_check() {
    let (_env, client, _admin, user, _token_client, token_admin, pool, oracle, token_id) =
        setup_env();

    // Price: $1.00 encoded as 10_000_000 (7-decimal oracle format).
    // 500 tokens → collateral_value = 500 * 10_000_000 / PRICE_PRECISION = $500 USD.
    oracle.set_price(&token_id, &10_000_000, &1000);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Debt is denominated in USD (same unit as collateral_value after PRICE_PRECISION).
    // debt = $400.  Minimum required collateral = 400 * 110 / 100 = $440.
    // Withdrawing 101 → remaining = 500 − 101 = $399 < $440 → blocked.
    pool.set_user_debt(&400);

    let res = client.try_withdraw(&user, &token_id, &101);
    assert!(
        res.is_err(),
        "should block withdrawal that reduces ratio below 110%"
    );

    // Withdrawing 50 → remaining = 500 − 50 = $450 ≥ $440 → allowed.
    client.withdraw(&user, &token_id, &50);
    assert_eq!(client.get_position_balance(&user, &token_id), 450);
}

#[test]
fn test_withdraw_emits_event() {
    let (env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.withdraw(&user, &token_id, &100);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    // Verify it's a "Withdrawn" event
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, soroban_sdk::Symbol::new(&env, "withdrawn"));
}

#[test]
fn test_withdraw_tokens_returned() {
    let (_env, client, _admin, user, token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert_eq!(token_client.balance(&user), 500);

    client.withdraw(&user, &token_id, &200);

    assert_eq!(token_client.balance(&user), 700);
}

// ---------------------------------------------------------------------------
// Post-delist withdrawal tests (issue #575)
// ---------------------------------------------------------------------------

/// A user can perform a partial withdrawal after an asset is delisted.
#[test]
fn test_withdraw_partial_after_delist() {
    let (_env, client, _admin, user, token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Delist the asset — no new deposits allowed.
    client.delist_supported_asset(&token_id);

    // Partial withdrawal must succeed.
    client.withdraw(&user, &token_id, &200);

    assert_eq!(
        client.get_position_balance(&user, &token_id),
        300,
        "remaining balance should be 300 after partial withdrawal"
    );
    assert_eq!(
        token_client.balance(&user),
        700,
        "user should receive the withdrawn tokens back"
    );
}

/// A user can withdraw their full balance after an asset is delisted.
#[test]
fn test_withdraw_full_after_delist() {
    let (_env, client, _admin, user, token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.delist_supported_asset(&token_id);

    client.withdraw(&user, &token_id, &500);

    assert_eq!(
        client.get_position_balance(&user, &token_id),
        0,
        "balance should be zero after full withdrawal"
    );
    assert_eq!(
        token_client.balance(&user),
        1000,
        "user should get all tokens back"
    );
    // User should be removed from the position index.
    assert!(
        !client.get_position_index().contains(&user),
        "user should be removed from position index after full withdrawal"
    );
}

/// New deposits into a delisted asset are rejected even when the user still
/// has an existing balance.
#[test]
fn test_deposit_blocked_after_delist_with_existing_balance() {
    let (_env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &300);

    client.delist_supported_asset(&token_id);

    let res = client.try_deposit(&user, &token_id, &100);
    assert!(res.is_err(), "deposit into delisted asset must be rejected");
}

/// A delisted position remains priceable: get_collateral_value should work.
#[test]
fn test_delisted_position_remains_priceable() {
    let (_env, client, _admin, user, _token_client, token_admin, _pool, oracle, token_id) =
        setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Price: $1.00 encoded with 7 decimal places.
    oracle.set_price(&token_id, &10_000_000, &1000);

    client.delist_supported_asset(&token_id);

    // Valuation must still work — 500 tokens × $1.00 / 10_000_000 = $500 USD.
    let value = client.get_collateral_value(&user);
    assert_eq!(value, 500, "delisted position must remain priceable");
}

/// A delisted position is still liquidatable via seize_collateral.
#[test]
fn test_delisted_position_is_liquidatable() {
    let (env, client, _admin, user, token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    let engine = Address::generate(&env);
    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Delist the asset.
    client.delist_supported_asset(&token_id);

    // Seizure must succeed regardless of asset status.
    client.seize_collateral(&engine, &user, &token_id, &300);

    assert_eq!(
        client.get_position_balance(&user, &token_id),
        200,
        "200 tokens should remain after partial seizure"
    );
    assert_eq!(
        token_client.balance(&engine),
        300,
        "engine should receive the seized tokens"
    );
}

/// Full liquidation of a delisted position cleans up the user's index entry.
#[test]
fn test_full_liquidation_after_delist_clears_position() {
    let (env, client, _admin, user, _token_client, token_admin, _pool, _oracle, token_id) =
        setup_env();

    let engine = Address::generate(&env);
    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.delist_supported_asset(&token_id);

    client.seize_collateral(&engine, &user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert!(
        !client.get_position_index().contains(&user),
        "user should be removed from index after full liquidation"
    );
}
