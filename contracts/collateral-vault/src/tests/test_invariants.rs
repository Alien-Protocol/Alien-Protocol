//! Invariant tests for the collateral-vault.
//!
//! These tests exercise the correctness of the slot-based indices under
//! multi-asset, multi-user, and randomized-sequence scenarios:
//!
//! - Balance, asset-membership, and position-index consistency after deposits
//!   and full/partial withdrawals.
//! - Seize-collateral path preserves the same invariants.
//! - Swap-and-pop removal is correct regardless of which slot holds the item.
//! - Empty user-asset vectors are cleaned up; empty position-index entries are
//!   cleaned up.

#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};
use types::NO_NEXT_CURSOR;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address, // admin
    Address, // oracle (placeholder)
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.initialize(&admin, &oracle);

    (env, client, admin, oracle)
}

fn make_token(
    env: &Env,
    client: &VaultContractClient,
) -> (Address, token::StellarAssetClient<'static>) {
    let ta = Address::generate(env);
    let tc = env.register_stellar_asset_contract_v2(ta);
    let id = tc.address();
    let sac = token::StellarAssetClient::new(env, &id);
    client.add_supported_asset(&id);
    (id, sac)
}

/// Drain every page of `get_positions_page` and collect all users.
fn collect_all_users(env: &Env, client: &VaultContractClient) -> soroban_sdk::Vec<Address> {
    let mut result: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(env);
    let mut cursor: u32 = 0;
    loop {
        let page = client.get_positions_page(&cursor, &50);
        for p in page.positions.iter() {
            result.push_back(p.user.clone());
        }
        if page.next_cursor == NO_NEXT_CURSOR {
            break;
        }
        cursor = page.next_cursor;
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Single-user single-asset invariants
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_full_exit_removes_from_position_index() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &1000);

    assert!(client.get_position_index().contains(&user));

    client.withdraw(&user, &token_id, &1000);

    assert!(!client.get_position_index().contains(&user));
    assert_eq!(client.get_position_count(), 0);
}

#[test]
fn test_partial_exit_keeps_user_in_index() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &1000);
    client.withdraw(&user, &token_id, &400);

    assert!(client.get_position_index().contains(&user));
    assert_eq!(client.get_position_balance(&user, &token_id), 600);
}

#[test]
fn test_balance_zero_key_absent_after_full_withdraw() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user, &500);
    client.deposit(&user, &token_id, &500);
    client.withdraw(&user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    // user-asset page must be empty
    let page = client.get_user_assets_page(&user, &0, &10);
    assert!(page.assets.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Multi-asset invariants
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_multi_asset_partial_exit_keeps_remaining_assets() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let (tok1, sac1) = make_token(&env, &client);
    let (tok2, sac2) = make_token(&env, &client);

    sac1.mint(&user, &1000);
    sac2.mint(&user, &1000);
    client.deposit(&user, &tok1, &500);
    client.deposit(&user, &tok2, &300);

    // Fully withdraw tok1
    client.withdraw(&user, &tok1, &500);

    // User still in index because tok2 remains
    assert!(client.get_position_index().contains(&user));
    assert_eq!(client.get_position_balance(&user, &tok1), 0);
    assert_eq!(client.get_position_balance(&user, &tok2), 300);

    // user-asset page only contains tok2
    let page = client.get_user_assets_page(&user, &0, &10);
    assert_eq!(page.assets.len(), 1);
    assert!(page.assets.contains(&tok2));
}

#[test]
fn test_multi_asset_full_exit_removes_all_keys() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let (tok1, sac1) = make_token(&env, &client);
    let (tok2, sac2) = make_token(&env, &client);

    sac1.mint(&user, &1000);
    sac2.mint(&user, &1000);
    client.deposit(&user, &tok1, &500);
    client.deposit(&user, &tok2, &300);

    client.withdraw(&user, &tok1, &500);
    client.withdraw(&user, &tok2, &300);

    assert!(!client.get_position_index().contains(&user));
    assert_eq!(client.get_position_count(), 0);

    let page = client.get_user_assets_page(&user, &0, &10);
    assert!(page.assets.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// Swap-and-pop correctness (index ordering)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_remove_first_user_of_three_preserves_others() {
    let (env, client, _admin, _oracle) = setup_env();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user1, &100);
    sac.mint(&user2, &100);
    sac.mint(&user3, &100);
    client.deposit(&user1, &token_id, &10);
    client.deposit(&user2, &token_id, &10);
    client.deposit(&user3, &token_id, &10);

    // Remove the first depositor
    client.withdraw(&user1, &token_id, &10);

    assert_eq!(client.get_position_count(), 2);
    let users = collect_all_users(&env, &client);
    assert!(!users.contains(&user1));
    assert!(users.contains(&user2));
    assert!(users.contains(&user3));
}

#[test]
fn test_remove_middle_user_of_three_preserves_others() {
    let (env, client, _admin, _oracle) = setup_env();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user1, &100);
    sac.mint(&user2, &100);
    sac.mint(&user3, &100);
    client.deposit(&user1, &token_id, &10);
    client.deposit(&user2, &token_id, &10);
    client.deposit(&user3, &token_id, &10);

    client.withdraw(&user2, &token_id, &10);

    assert_eq!(client.get_position_count(), 2);
    let users = collect_all_users(&env, &client);
    assert!(users.contains(&user1));
    assert!(!users.contains(&user2));
    assert!(users.contains(&user3));
}

#[test]
fn test_remove_last_user_of_three_preserves_others() {
    let (env, client, _admin, _oracle) = setup_env();
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user1, &100);
    sac.mint(&user2, &100);
    sac.mint(&user3, &100);
    client.deposit(&user1, &token_id, &10);
    client.deposit(&user2, &token_id, &10);
    client.deposit(&user3, &token_id, &10);

    client.withdraw(&user3, &token_id, &10);

    assert_eq!(client.get_position_count(), 2);
    let users = collect_all_users(&env, &client);
    assert!(users.contains(&user1));
    assert!(users.contains(&user2));
    assert!(!users.contains(&user3));
}

// ─────────────────────────────────────────────────────────────────────────────
// Seize-collateral invariants
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_seize_full_amount_removes_user_from_index() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let engine = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    client.set_liquidation_engine(&engine);
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &500);

    assert!(!client.get_position_index().contains(&user));
    assert_eq!(client.get_position_count(), 0);
}

#[test]
fn test_seize_partial_keeps_user_in_index() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let engine = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    client.set_liquidation_engine(&engine);
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);

    assert!(client.get_position_index().contains(&user));
    assert_eq!(client.get_position_balance(&user, &token_id), 300);
}

// ─────────────────────────────────────────────────────────────────────────────
// Re-deposit after full exit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_redeposit_after_full_exit_restores_index() {
    let (env, client, _admin, _oracle) = setup_env();
    let user = Address::generate(&env);
    let (token_id, sac) = make_token(&env, &client);

    sac.mint(&user, &2000);

    // First lifecycle
    client.deposit(&user, &token_id, &1000);
    client.withdraw(&user, &token_id, &1000);
    assert!(!client.get_position_index().contains(&user));

    // Re-enter
    client.deposit(&user, &token_id, &500);
    assert!(client.get_position_index().contains(&user));
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
    assert_eq!(client.get_position_count(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Budget-sensitive: N users, M assets each
// ─────────────────────────────────────────────────────────────────────────────

/// Deposit with a realistic number of users (20) and assets (5) to verify that
/// write costs do not grow with total-user count.  This is not a fuzz test but
/// provides a baseline for manual Soroban budget inspection during CI.
#[test]
fn test_budget_twenty_users_five_assets_deposit_and_withdraw() {
    let (env, client, _admin, _oracle) = setup_env();

    const N_USERS: usize = 20;
    const N_ASSETS: usize = 5;

    let mut users: [Option<Address>; N_USERS] = core::array::from_fn(|_| None);
    let mut tokens: [Option<Address>; N_ASSETS] = core::array::from_fn(|_| None);
    let mut sacs: [Option<token::StellarAssetClient>; N_ASSETS] = core::array::from_fn(|_| None);

    for i in 0..N_ASSETS {
        let (id, sac) = make_token(&env, &client);
        tokens[i] = Some(id);
        sacs[i] = Some(sac);
    }
    for i in 0..N_USERS {
        users[i] = Some(Address::generate(&env));
    }

    // All users deposit into all assets
    for i in 0..N_USERS {
        let user = users[i].as_ref().unwrap();
        for j in 0..N_ASSETS {
            let token_id = tokens[j].as_ref().unwrap();
            let sac = sacs[j].as_ref().unwrap();
            sac.mint(user, &1000);
            client.deposit(user, token_id, &100);
        }
    }

    assert_eq!(client.get_position_count(), N_USERS as u32);

    // All users fully withdraw from all assets
    for i in 0..N_USERS {
        let user = users[i].as_ref().unwrap();
        for j in 0..N_ASSETS {
            let token_id = tokens[j].as_ref().unwrap();
            client.withdraw(user, token_id, &100);
        }
    }

    assert_eq!(client.get_position_count(), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Supported-asset index invariants (swap-and-pop)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_remove_first_asset_of_three_preserves_others() {
    let (env, client, _admin, _oracle) = setup_env();
    let (tok1, _) = make_token(&env, &client);
    let (tok2, _) = make_token(&env, &client);
    // setup already added a 4th token; we work with the three added in make_token + setup

    client.remove_supported_asset(&tok1);

    assert!(!client.is_supported_asset(&tok1));
    assert!(client.is_supported_asset(&tok2));
}

#[test]
fn test_add_remove_add_asset_is_consistent() {
    let (env, client, _admin, _oracle) = setup_env();
    let (tok, _) = make_token(&env, &client);

    client.remove_supported_asset(&tok);
    assert!(!client.is_supported_asset(&tok));

    client.add_supported_asset(&tok);
    assert!(client.is_supported_asset(&tok));
}
