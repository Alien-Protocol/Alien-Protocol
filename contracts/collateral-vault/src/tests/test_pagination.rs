//! Pagination tests for the collateral-vault.
//!
//! Covers:
//! - Cursor/limit correctness for `get_positions_page`
//! - Cursor/limit correctness for `get_supported_assets_page`
//! - Cursor/limit correctness for `get_user_assets_page`
//! - Rejection of limit == 0 and limit > MAX_PAGE_LIMIT with specific error
//! - Boundary acceptance: limit 1 and 50 accepted across all three endpoints
//! - Deterministic ordering of continuation cursors

#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

// ─────────────────────────────────────────────────────────────────────────────
// Shared test helpers
// ─────────────────────────────────────────────────────────────────────────────

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address, // admin
    Address, // default user
    Address, // default token
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.initialize(&admin, &oracle);

    let user = Address::generate(&env);

    let token_admin_addr = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    (env, client, admin, user, token_id, token_admin_client)
}

/// Register a fresh token, add it as supported, and return its address + admin client.
fn add_token(
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

// ─────────────────────────────────────────────────────────────────────────────
// get_positions_page — correctness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_positions_page_empty_collection() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let page = client.get_positions_page(&0, &10);
    assert!(page.positions.is_empty());
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_positions_page_single_item() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let page = client.get_positions_page(&0, &10);
    assert_eq!(page.positions.len(), 1);
    assert_eq!(page.positions.get(0).unwrap().user, user);
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_positions_page_cursor_advances() {
    let (env, client, _admin, user1, token_id, sac) = setup_env();
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    sac.mint(&user1, &1000);
    sac.mint(&user2, &1000);
    sac.mint(&user3, &1000);
    client.deposit(&user1, &token_id, &100);
    client.deposit(&user2, &token_id, &100);
    client.deposit(&user3, &token_id, &100);

    // Page 1: limit=2, cursor=0 → 2 items, next_cursor=2
    let page1 = client.get_positions_page(&0, &2);
    assert_eq!(page1.positions.len(), 2);
    assert_eq!(page1.next_cursor, 2);

    // Page 2: limit=2, cursor=2 → 1 item, exhausted
    let page2 = client.get_positions_page(&2, &2);
    assert_eq!(page2.positions.len(), 1);
    assert_eq!(page2.next_cursor, types::NO_NEXT_CURSOR);

    // Combined covers all three users
    let mut users: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
    for p in page1.positions.iter() {
        users.push_back(p.user.clone());
    }
    for p in page2.positions.iter() {
        users.push_back(p.user.clone());
    }
    assert!(users.contains(&user1));
    assert!(users.contains(&user2));
    assert!(users.contains(&user3));
}

#[test]
fn test_positions_page_limit_equals_count() {
    let (env, client, _admin, user1, token_id, sac) = setup_env();
    let user2 = Address::generate(&env);

    sac.mint(&user1, &1000);
    sac.mint(&user2, &1000);
    client.deposit(&user1, &token_id, &100);
    client.deposit(&user2, &token_id, &100);

    let page = client.get_positions_page(&0, &2);
    assert_eq!(page.positions.len(), 2);
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_positions_page_cursor_at_end_returns_empty() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &100);

    let page = client.get_positions_page(&1, &10);
    assert!(page.positions.is_empty());
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_positions_page_withdrawn_user_excluded() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.withdraw(&user, &token_id, &500);

    let page = client.get_positions_page(&0, &10);
    assert!(page.positions.is_empty());
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

// ─────────────────────────────────────────────────────────────────────────────
// get_positions_page — limit boundary tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_positions_page_limit_one_accepted() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let page = client.get_positions_page(&0, &1);
    assert!(page.positions.is_empty());
}

#[test]
fn test_positions_page_limit_fifty_accepted() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let page = client.get_positions_page(&0, &50);
    assert!(page.positions.is_empty());
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_positions_page_limit_zero_returns_page_limit_exceeded() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let result = client.try_get_positions_page(&0, &0);
    assert_eq!(result, Err(Ok(VaultError::PageLimitExceeded)));
}

#[test]
fn test_positions_page_limit_fifty_one_returns_page_limit_exceeded() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let result = client.try_get_positions_page(&0, &51);
    assert_eq!(result, Err(Ok(VaultError::PageLimitExceeded)));
}

// ─────────────────────────────────────────────────────────────────────────────
// get_supported_assets_page — correctness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_supported_assets_page_basic() {
    let (env, client, _admin, _user, token_id, _sac) = setup_env();
    let (tok2, _) = add_token(&env, &client);
    let (tok3, _) = add_token(&env, &client);

    let page = client.get_supported_assets_page(&0, &10);
    assert_eq!(page.assets.len(), 3);
    assert!(page.assets.contains(&token_id));
    assert!(page.assets.contains(&tok2));
    assert!(page.assets.contains(&tok3));
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_supported_assets_page_pagination() {
    let (env, client, _admin, _user, _token_id, _sac) = setup_env();
    for _ in 0..4 {
        add_token(&env, &client);
    }
    // 5 total (1 from setup + 4 added above)
    let page1 = client.get_supported_assets_page(&0, &3);
    assert_eq!(page1.assets.len(), 3);
    assert_eq!(page1.next_cursor, 3);

    let page2 = client.get_supported_assets_page(&3, &3);
    assert_eq!(page2.assets.len(), 2);
    assert_eq!(page2.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_supported_assets_page_after_remove() {
    let (_env, client, _admin, _user, token_id, _sac) = setup_env();
    client.remove_supported_asset(&token_id);

    let page = client.get_supported_assets_page(&0, &10);
    assert!(page.assets.is_empty());
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

// ─────────────────────────────────────────────────────────────────────────────
// get_supported_assets_page — limit boundary tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_supported_assets_page_limit_one_accepted() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let page = client.get_supported_assets_page(&0, &1);
    assert_eq!(page.assets.len(), 1); // setup added one token
}

#[test]
fn test_supported_assets_page_limit_fifty_accepted() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let page = client.get_supported_assets_page(&0, &50);
    assert_eq!(page.assets.len(), 1);
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_supported_assets_page_limit_zero_returns_page_limit_exceeded() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let result = client.try_get_supported_assets_page(&0, &0);
    assert_eq!(result, Err(Ok(VaultError::PageLimitExceeded)));
}

#[test]
fn test_supported_assets_page_limit_fifty_one_returns_page_limit_exceeded() {
    let (_env, client, _admin, _user, _token_id, _sac) = setup_env();
    let result = client.try_get_supported_assets_page(&0, &51);
    assert_eq!(result, Err(Ok(VaultError::PageLimitExceeded)));
}

// ─────────────────────────────────────────────────────────────────────────────
// get_user_assets_page — correctness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_user_assets_page_single_asset() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &100);

    let page = client.get_user_assets_page(&user, &0, &10);
    assert_eq!(page.assets.len(), 1);
    assert!(page.assets.contains(&token_id));
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_user_assets_page_multiple_assets_paginated() {
    let (env, client, _admin, user, token_id, sac) = setup_env();
    let (tok2, sac2) = add_token(&env, &client);
    let (tok3, sac3) = add_token(&env, &client);

    sac.mint(&user, &1000);
    sac2.mint(&user, &1000);
    sac3.mint(&user, &1000);
    client.deposit(&user, &token_id, &100);
    client.deposit(&user, &tok2, &100);
    client.deposit(&user, &tok3, &100);

    let page1 = client.get_user_assets_page(&user, &0, &2);
    assert_eq!(page1.assets.len(), 2);
    assert_eq!(page1.next_cursor, 2);

    let page2 = client.get_user_assets_page(&user, &2, &2);
    assert_eq!(page2.assets.len(), 1);
    assert_eq!(page2.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_user_assets_page_removed_on_full_withdraw() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.withdraw(&user, &token_id, &500);

    let page = client.get_user_assets_page(&user, &0, &10);
    assert!(page.assets.is_empty());
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

// ─────────────────────────────────────────────────────────────────────────────
// get_user_assets_page — limit boundary tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_user_assets_page_limit_one_accepted() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &100);

    let page = client.get_user_assets_page(&user, &0, &1);
    assert_eq!(page.assets.len(), 1);
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_user_assets_page_limit_fifty_accepted() {
    let (_env, client, _admin, user, token_id, sac) = setup_env();
    sac.mint(&user, &1000);
    client.deposit(&user, &token_id, &100);

    let page = client.get_user_assets_page(&user, &0, &50);
    assert_eq!(page.assets.len(), 1);
    assert_eq!(page.next_cursor, types::NO_NEXT_CURSOR);
}

#[test]
fn test_user_assets_page_limit_zero_returns_page_limit_exceeded() {
    let (_env, client, _admin, user, _token_id, _sac) = setup_env();
    let result = client.try_get_user_assets_page(&user, &0, &0);
    assert_eq!(result, Err(Ok(VaultError::PageLimitExceeded)));
}

#[test]
fn test_user_assets_page_limit_fifty_one_returns_page_limit_exceeded() {
    let (_env, client, _admin, user, _token_id, _sac) = setup_env();
    let result = client.try_get_user_assets_page(&user, &0, &51);
    assert_eq!(result, Err(Ok(VaultError::PageLimitExceeded)));
}
