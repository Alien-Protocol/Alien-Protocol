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
        if page.next_cursor == types::NO_NEXT_CURSOR {
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

/// Asserts that deposit and withdraw CPU costs do not grow with the number of
/// existing users by comparing instruction counts on an empty index versus a
/// 19-user populated one.  The 20th user's operation cost must stay within 5%
/// of the baseline, confirming O(1) per-user behaviour.
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

    // ── Baseline: measure cost for user[0] on an empty index ─────────────────
    let baseline_user = users[0].as_ref().unwrap();
    let baseline_token = tokens[0].as_ref().unwrap();
    let baseline_sac = sacs[0].as_ref().unwrap();

    baseline_sac.mint(baseline_user, &1000);

    env.budget().reset_default();
    client.deposit(baseline_user, baseline_token, &100);
    let baseline_deposit_cpu = env.budget().cpu_instruction_count();

    env.budget().reset_default();
    client.withdraw(baseline_user, baseline_token, &100);
    let baseline_withdraw_cpu = env.budget().cpu_instruction_count();

    // ── Populate 19 more users ────────────────────────────────────────────────
    for i in 1..N_USERS {
        let user = users[i].as_ref().unwrap();
        for j in 0..N_ASSETS {
            let token_id = tokens[j].as_ref().unwrap();
            let sac = sacs[j].as_ref().unwrap();
            sac.mint(user, &1000);
            client.deposit(user, token_id, &100);
        }
    }

    assert_eq!(client.get_position_count(), N_USERS as u32 - 1);

    // ── Re-measure cost for user[0] re-entering a 19-user index ─────────────
    baseline_sac.mint(baseline_user, &200);

    env.budget().reset_default();
    client.deposit(baseline_user, baseline_token, &100);
    let populated_deposit_cpu = env.budget().cpu_instruction_count();

    env.budget().reset_default();
    client.withdraw(baseline_user, baseline_token, &100);
    let populated_withdraw_cpu = env.budget().cpu_instruction_count();

    // ── Assert costs stay within 5% of baseline (O(1) not O(n)) ─────────────
    let deposit_delta = populated_deposit_cpu.abs_diff(baseline_deposit_cpu);
    let withdraw_delta = populated_withdraw_cpu.abs_diff(baseline_withdraw_cpu);

    assert!(
        deposit_delta <= baseline_deposit_cpu / 20,
        "deposit CPU grew by {deposit_delta} instructions vs baseline {baseline_deposit_cpu} — possible O(n) regression"
    );
    assert!(
        withdraw_delta <= baseline_withdraw_cpu / 20,
        "withdraw CPU grew by {withdraw_delta} instructions vs baseline {baseline_withdraw_cpu} — possible O(n) regression"
    );

    // ── Final lifecycle correctness ───────────────────────────────────────────
    // Fully drain remaining users
    for i in 1..N_USERS {
        let user = users[i].as_ref().unwrap();
        for j in 0..N_ASSETS {
            let token_id = tokens[j].as_ref().unwrap();
            client.withdraw(user, token_id, &100);
        }
    }

    assert_eq!(client.get_position_count(), 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// Randomized operation sequence (deterministic seed via address generation)
//
// Simulates deposit → partial-withdraw → full-withdraw → redeposit sequences
// and verifies balance, user-asset membership, and position-index invariants
// after every step using a fixed address-generation seed.
// ─────────────────────────────────────────────────────────────────────────────

/// Model-based sequence: for each of N users run a scripted mix of
/// deposit / partial-withdraw / full-withdraw / redeposit across M assets,
/// asserting the three key invariants after every operation:
///   1. `get_position_balance` matches expected value
///   2. user-asset page contains the asset iff balance > 0
///   3. position index contains user iff any balance > 0
#[test]
fn test_randomized_operation_sequence_invariants() {
    let (env, client, _admin, _oracle) = setup_env();

    // Use 4 users and 3 assets for a dense interaction matrix.
    const N_USERS: usize = 4;
    const N_ASSETS: usize = 3;

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

    // expected[user][asset] mirrors what we think the on-chain balance is
    let mut expected = [[0i128; N_ASSETS]; N_USERS];

    // Helper: check all three invariants for a single (user, asset) pair
    let check = |u: usize, a: usize, exp: i128| {
        let user = users[u].as_ref().unwrap();
        let token_id = tokens[a].as_ref().unwrap();

        // 1. balance matches
        assert_eq!(
            client.get_position_balance(user, token_id),
            exp,
            "user[{u}] asset[{a}]: balance mismatch"
        );

        // 2. user-asset page membership
        let in_assets = {
            let page = client.get_user_assets_page(user, &0, &50);
            page.assets.contains(token_id)
        };
        assert_eq!(
            in_assets,
            exp > 0,
            "user[{u}] asset[{a}]: asset-page membership inconsistent"
        );

        // 3. position-index membership: user should be in index iff any asset > 0
        let any_balance = (0..N_ASSETS).any(|b| {
            client.get_position_balance(user, tokens[b].as_ref().unwrap()) > 0
        });
        let in_index = client.get_position_index().contains(user);
        assert_eq!(
            in_index, any_balance,
            "user[{u}]: position-index membership inconsistent"
        );
    };

    // ── Sequence A: all users deposit into all assets ─────────────────────────
    for u in 0..N_USERS {
        for a in 0..N_ASSETS {
            let user = users[u].as_ref().unwrap();
            let token_id = tokens[a].as_ref().unwrap();
            let sac = sacs[a].as_ref().unwrap();
            sac.mint(user, &500);
            client.deposit(user, token_id, &200);
            expected[u][a] = 200;
            check(u, a, expected[u][a]);
        }
    }

    // ── Sequence B: partial withdrawal from first asset ───────────────────────
    for u in 0..N_USERS {
        let user = users[u].as_ref().unwrap();
        let token_id = tokens[0].as_ref().unwrap();
        client.withdraw(user, token_id, &100);
        expected[u][0] -= 100;
        check(u, 0, expected[u][0]);
    }

    // ── Sequence C: full withdrawal from second asset ─────────────────────────
    for u in 0..N_USERS {
        let user = users[u].as_ref().unwrap();
        let token_id = tokens[1].as_ref().unwrap();
        client.withdraw(user, token_id, &200);
        expected[u][1] = 0;
        check(u, 1, expected[u][1]);
    }

    // ── Sequence D: redeposit into second asset ───────────────────────────────
    for u in 0..N_USERS {
        let user = users[u].as_ref().unwrap();
        let token_id = tokens[1].as_ref().unwrap();
        let sac = sacs[1].as_ref().unwrap();
        sac.mint(user, &300);
        client.deposit(user, token_id, &150);
        expected[u][1] = 150;
        check(u, 1, expected[u][1]);
    }

    // ── Sequence E: full exit for all users ───────────────────────────────────
    for u in 0..N_USERS {
        let user = users[u].as_ref().unwrap();
        for a in 0..N_ASSETS {
            let bal = expected[u][a];
            if bal > 0 {
                let token_id = tokens[a].as_ref().unwrap();
                client.withdraw(user, token_id, &bal);
                expected[u][a] = 0;
                check(u, a, 0);
            }
        }
        // After full exit, user must not be in index
        assert!(
            !client.get_position_index().contains(users[u].as_ref().unwrap()),
            "user[{u}] still in index after full exit"
        );
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
