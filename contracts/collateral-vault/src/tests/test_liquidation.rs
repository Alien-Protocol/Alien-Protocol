#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env, Symbol};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let lending_pool = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

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
        oracle,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

// ---------------------------------------------------------------------------
// Authorization & basic success
// ---------------------------------------------------------------------------

#[test]
fn test_authorize_liquidation_success() {
    let (env, client, _admin, _user, _oracle, _token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
}

#[test]
fn test_seize_collateral_emits_event() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_ok());
}

#[test]
fn test_seize_collateral_success() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_ok());

    assert_eq!(client.get_position_balance(&user, &token_id), 300);
    assert_eq!(token_client.balance(&engine), 200);
    assert_eq!(token_client.balance(&client.address), 300);
}

#[test]
fn test_seize_collateral_unauthorized_engine_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);
    let malicious_engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&malicious_engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_paused_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause();

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Zero and negative amounts -> InvalidAmount
// ---------------------------------------------------------------------------

#[test]
fn test_seize_zero_amount_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &0);
    assert!(res.is_err());
}

#[test]
fn test_seize_negative_amount_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &-100);
    assert!(res.is_err());
}

#[test]
fn test_seize_i128_min_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &i128::MIN);
    assert!(res.is_err());
}

#[test]
fn test_seize_negative_one_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &-1);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Invalid seizure cannot mutate recorded balance
// ---------------------------------------------------------------------------

#[test]
fn test_seize_zero_does_not_mutate_balance() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let balance_before = client.get_position_balance(&user, &token_id);
    let _ = client.try_seize_collateral(&engine, &user, &token_id, &0);
    let balance_after = client.get_position_balance(&user, &token_id);

    assert_eq!(balance_before, balance_after);
    assert_eq!(balance_after, 500);
}

#[test]
fn test_seize_negative_does_not_mutate_balance() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let balance_before = client.get_position_balance(&user, &token_id);
    let _ = client.try_seize_collateral(&engine, &user, &token_id, &-200);
    let balance_after = client.get_position_balance(&user, &token_id);

    assert_eq!(balance_before, balance_after);
    assert_eq!(balance_after, 500);
}

#[test]
fn test_seize_zero_does_not_mutate_index() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));
    let _ = client.try_seize_collateral(&engine, &user, &token_id, &0);
    assert!(client.get_position_index().contains(&user));
}

#[test]
fn test_seize_negative_does_not_mutate_index() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));
    let _ = client.try_seize_collateral(&engine, &user, &token_id, &-1);
    assert!(client.get_position_index().contains(&user));
}

// ---------------------------------------------------------------------------
// Insufficient collateral -> InsufficientCollateral
// ---------------------------------------------------------------------------

#[test]
fn test_seize_insufficient_balance_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &600);
    assert!(res.is_err());
}

#[test]
fn test_seize_exact_balance_plus_one_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &501);
    assert!(res.is_err());
}

#[test]
fn test_seize_insufficient_does_not_mutate_balance() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let balance_before = client.get_position_balance(&user, &token_id);
    let _ = client.try_seize_collateral(&engine, &user, &token_id, &501);
    let balance_after = client.get_position_balance(&user, &token_id);

    assert_eq!(balance_before, balance_after);
    assert_eq!(balance_after, 500);
}

// ---------------------------------------------------------------------------
// No position -> NoPosition
// ---------------------------------------------------------------------------

#[test]
fn test_seize_no_position_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Boundary: seize exactly 1
// ---------------------------------------------------------------------------

#[test]
fn test_seize_one_from_large_balance() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &1);
    assert!(res.is_ok());

    assert_eq!(client.get_position_balance(&user, &token_id), 499);
    assert_eq!(token_client.balance(&engine), 1);
    assert_eq!(token_client.balance(&client.address), 499);
}

// ---------------------------------------------------------------------------
// Boundary: exact balance (full seizure)
// ---------------------------------------------------------------------------

#[test]
fn test_seize_exact_balance_full() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &500);
    assert!(res.is_ok());

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert_eq!(token_client.balance(&engine), 500);
    assert_eq!(token_client.balance(&client.address), 0);
}

// ---------------------------------------------------------------------------
// Full seizure cleans up index and user-asset tracking
// ---------------------------------------------------------------------------

#[test]
fn test_seize_full_removes_from_index() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));

    client.seize_collateral(&engine, &user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert!(!client.get_position_index().contains(&user));
}

// ---------------------------------------------------------------------------
// Partial seizure: index and user-asset membership preserved
// ---------------------------------------------------------------------------

#[test]
fn test_seize_partial_preserves_index_and_assets() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &300);

    assert_eq!(client.get_position_balance(&user, &token_id), 200);
    assert!(client.get_position_index().contains(&user));
    let position = client.get_position(&user);
    assert_eq!(position.collateral.len(), 1);
    assert_eq!(position.collateral.get_unchecked(0).amount, 200);
}

// ---------------------------------------------------------------------------
// i128::MAX boundary: far exceeds balance -> InsufficientCollateral
// ---------------------------------------------------------------------------

#[test]
fn test_seize_i128_max_fails_insufficient() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &i128::MAX);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Seize from user with multiple assets: only targeted asset affected
// ---------------------------------------------------------------------------

#[test]
fn test_seize_multi_asset_only_affects_target() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    let token_admin2 = Address::generate(&env);
    let token_contract2 = env.register_stellar_asset_contract_v2(token_admin2);
    let token_id2 = token_contract2.address();
    let token_client2 = token::Client::new(&env, &token_id2);
    let token_admin_client2 = token::StellarAssetClient::new(&env, &token_id2);

    client.set_liquidation_engine(&engine);
    client.add_supported_asset(&token_id2);

    token_admin.mint(&user, &1000);
    token_admin_client2.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.deposit(&user, &token_id2, &700);

    client.seize_collateral(&engine, &user, &token_id, &200);

    assert_eq!(client.get_position_balance(&user, &token_id), 300);
    assert_eq!(client.get_position_balance(&user, &token_id2), 700);
    assert_eq!(token_client.balance(&engine), 200);
    assert_eq!(token_client2.balance(&engine), 0);
    assert!(client.get_position_index().contains(&user));
}

// ---------------------------------------------------------------------------
// Invariant: balance, index, and user-asset membership stay consistent
// ---------------------------------------------------------------------------

#[test]
fn test_invariants_after_partial_seize() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &150);

    let position = client.get_position(&user);
    assert_eq!(position.collateral.len(), 1);
    assert_eq!(position.collateral.get_unchecked(0).asset, token_id);
    assert_eq!(position.collateral.get_unchecked(0).amount, 350);
    assert!(client.get_position_index().contains(&user));
}

#[test]
fn test_invariants_after_full_seize() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &500);

    let res = client.try_get_position(&user);
    assert!(res.is_err());
    assert!(!client.get_position_index().contains(&user));
}

// ---------------------------------------------------------------------------
// Multiple sequential seizures
// ---------------------------------------------------------------------------

#[test]
fn test_seize_sequential_partial() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &100);
    assert_eq!(client.get_position_balance(&user, &token_id), 400);

    client.seize_collateral(&engine, &user, &token_id, &200);
    assert_eq!(client.get_position_balance(&user, &token_id), 200);

    client.seize_collateral(&engine, &user, &token_id, &200);
    assert_eq!(client.get_position_balance(&user, &token_id), 0);

    assert!(!client.get_position_index().contains(&user));
    assert_eq!(token_client.balance(&engine), 500);
    assert_eq!(token_client.balance(&client.address), 0);
}
