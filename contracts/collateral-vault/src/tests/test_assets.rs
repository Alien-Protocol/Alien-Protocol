#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
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

    client.initialize(&admin, &oracle);

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
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

// ---------------------------------------------------------------------------
// Existing add/remove tests (updated for new lifecycle)
// ---------------------------------------------------------------------------

#[test]
fn test_add_asset_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let other_token = Address::generate(&env);

    client.add_supported_asset(&other_token);
    assert!(client.is_supported_asset(&other_token));
}

#[test]
fn test_add_asset_duplicate_fails() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    // Re-adding a known asset (Active or DepositDisabled) must fail.
    let res = client.try_add_supported_asset(&token_id);
    assert!(res.is_err());
}

#[test]
fn test_remove_asset_with_no_balance_succeeds() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    // No positions exist, so hard-removal is allowed.
    assert!(client.is_supported_asset(&token_id));
    client.remove_supported_asset(&token_id);
    assert!(!client.is_supported_asset(&token_id));
}

#[test]
fn test_remove_asset_not_found_fails() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let unknown_token = Address::generate(&env);

    let res = client.try_remove_supported_asset(&unknown_token);
    assert!(res.is_err());
}

/// Hard removal must fail while any user balance remains.
#[test]
fn test_remove_asset_with_open_position_fails() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Asset has an open position — removal must be blocked.
    let res = client.try_remove_supported_asset(&token_id);
    assert!(
        res.is_err(),
        "should reject removal when user balance is non-zero"
    );

    // Balance must be untouched.
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

/// After all balances are withdrawn the hard-remove succeeds.
#[test]
fn test_remove_asset_after_full_withdrawal_succeeds() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // First delist so the user can withdraw.
    client.delist_supported_asset(&token_id);
    client.withdraw(&user, &token_id, &500);

    // Now the balance is zero — hard removal should succeed.
    client.remove_supported_asset(&token_id);
    assert!(!client.is_supported_asset(&token_id));
}

#[test]
fn test_remove_asset_does_not_clear_existing_positions() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Even though removal is blocked, verify the guard preserves the balance.
    let _ = client.try_remove_supported_asset(&token_id);

    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

#[test]
fn test_is_supported_asset_true() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();
    assert!(client.is_supported_asset(&token_id));
}

#[test]
fn test_is_supported_asset_false() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let unknown_token = Address::generate(&env);
    assert!(!client.is_supported_asset(&unknown_token));
}

// ---------------------------------------------------------------------------
// Lifecycle / delist tests
// ---------------------------------------------------------------------------

/// `delist_supported_asset` transitions an Active asset to DepositDisabled.
#[test]
fn test_delist_asset_sets_deposit_disabled() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    // Initially Active — is_supported_asset returns true.
    assert!(client.is_supported_asset(&token_id));

    client.delist_supported_asset(&token_id);

    // After delisting, is_supported_asset must return false (no new deposits).
    assert!(!client.is_supported_asset(&token_id));

    // But get_asset_status must return DepositDisabled, not None.
    let status = client.get_asset_status(&token_id);
    assert!(
        matches!(status, Some(types::AssetStatus::DepositDisabled)),
        "expected DepositDisabled, got {:?}",
        status
    );
}

/// Delisting an unknown asset must fail.
#[test]
fn test_delist_unknown_asset_fails() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let unknown = Address::generate(&env);

    let res = client.try_delist_supported_asset(&unknown);
    assert!(res.is_err());
}

/// Delisting an already-delisted asset is idempotent (no panic).
#[test]
fn test_delist_already_delisted_is_idempotent() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    client.delist_supported_asset(&token_id);
    // Second call must not panic.
    client.delist_supported_asset(&token_id);

    assert!(!client.is_supported_asset(&token_id));
}

/// Deposits into a delisted (DepositDisabled) asset must be rejected.
#[test]
fn test_deposit_into_delisted_asset_fails() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.delist_supported_asset(&token_id);

    let res = client.try_deposit(&user, &token_id, &100);
    assert!(
        res.is_err(),
        "should reject deposit into DepositDisabled asset"
    );
}

/// Re-adding an asset after delisting must fail (it's still a known asset).
#[test]
fn test_add_asset_after_delist_fails() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    client.delist_supported_asset(&token_id);

    let res = client.try_add_supported_asset(&token_id);
    assert!(
        res.is_err(),
        "should not allow re-adding a known (delisted) asset"
    );
}

// ---------------------------------------------------------------------------
// Position and get_all_positions helpers
// ---------------------------------------------------------------------------

#[test]
fn test_get_all_positions_empty() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let positions = client.get_all_positions();
    assert!(positions.is_empty());
}

#[test]
fn test_get_all_positions_multiple_users() {
    let (env, client, _admin, user1, token_id, _token_client, token_admin) = setup_env();
    let user2 = Address::generate(&env);

    token_admin.mint(&user1, &1000);
    client.deposit(&user1, &token_id, &500);

    token_admin.mint(&user2, &1000);
    client.deposit(&user2, &token_id, &300);

    let positions = client.get_all_positions();
    assert_eq!(positions.len(), 2);

    let mut found_user1 = false;
    let mut found_user2 = false;
    for p in positions.iter() {
        if p.user == user1 {
            found_user1 = true;
        }
        if p.user == user2 {
            found_user2 = true;
        }
    }
    assert!(found_user1);
    assert!(found_user2);
}

#[test]
fn test_get_all_positions_excludes_withdrawn() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert_eq!(client.get_all_positions().len(), 1);

    client.withdraw(&user, &token_id, &500);

    assert_eq!(client.get_all_positions().len(), 0);
}
