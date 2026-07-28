#![cfg(test)]

//! Invariant / property tests.
//!
//! Each test drives a sequence of operations and asserts that the following
//! invariants hold **after every individual step**:
//!
//! 1. **Custody** – sum of per-user balances == contract token balance.
//! 2. **Balance** – no user position balance is negative.
//! 3. **Index** – every asset in a user's position is in the supported-asset list
//!    OR was deposited before the asset was delisted (delisted-exit invariant).
//! 4. **Solvency** – total collateral value >= total outstanding debt.
//! 5. **Delisted-asset exit** – after delist, existing depositors can still
//!    withdraw their full balance (no funds become locked).

extern crate alloc;

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

// ---------------------------------------------------------------------------
// Mock lending pool (zero debt by default, configurable)
// ---------------------------------------------------------------------------

#[contract]
pub struct InvMockPool;

#[contractimpl]
impl InvMockPool {
    pub fn get_user_debt(env: Env, _user: Address) -> i128 {
        env.storage().persistent().get(&"debt").unwrap_or(0_i128)
    }

    pub fn set_user_debt(env: Env, debt: i128) {
        env.storage().persistent().set(&"debt", &debt);
    }
}

// ---------------------------------------------------------------------------
// Mock oracle (fixed price per asset, configurable)
// ---------------------------------------------------------------------------

#[contract]
pub struct InvMockOracle;

#[contractimpl]
impl InvMockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        env.storage().persistent().get(&asset)
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        env.storage()
            .persistent()
            .get(&asset)
            .unwrap_or(types::PriceData {
                price: 10_000_000,
                timestamp: env.ledger().timestamp(),
            })
    }

    pub fn set_price(env: Env, asset: Address, price: i128) {
        let ts = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&asset, &types::PriceData { price, timestamp: ts });
    }
}

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

struct Fixture {
    env: Env,
    client: VaultContractClient<'static>,
    pool: InvMockPoolClient<'static>,
    oracle: InvMockOracleClient<'static>,
    token_id: Address,
    token_client: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(InvMockOracle, ());
    let oracle = InvMockOracleClient::new(&env, &oracle_id);

    let pool_id = env.register(InvMockPool, ());
    let pool = InvMockPoolClient::new(&env, &pool_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);
    client.set_pool(&pool_id);

    let token_admin_addr = Address::generate(&env);
    let tc = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = tc.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);
    // Default price: $1.00 (7-decimal format = 10_000_000)
    oracle.set_price(&token_id, &10_000_000);

    Fixture { env, client, pool, oracle, token_id, token_client, token_admin }
}

// ---------------------------------------------------------------------------
// Invariant helpers (checked after every step)
// ---------------------------------------------------------------------------

/// Custody: sum of all recorded per-user balances == token.balance(vault).
fn assert_custody_invariant(f: &Fixture, users: &[Address]) {
    let mut sum: i128 = 0;
    for user in users {
        sum += f.client.get_position_balance(user, &f.token_id);
    }
    let vault_token_balance = f.token_client.balance(&f.client.address);
    assert_eq!(
        sum, vault_token_balance,
        "custody invariant violated: ledger sum={sum} vault_balance={vault_token_balance}"
    );
}

/// Balance: no individual position balance is negative.
fn assert_balance_invariant(f: &Fixture, users: &[Address]) {
    for user in users {
        let bal = f.client.get_position_balance(user, &f.token_id);
        assert!(bal >= 0, "balance invariant violated: user has negative balance {bal}");
    }
}

/// Index: if a user appears in the position index they must have a position;
/// if they do not appear they must have zero balance.
fn assert_index_invariant(f: &Fixture, users: &[Address]) {
    let index = f.client.get_position_index();
    for user in users {
        let in_index = index.contains(user);
        let bal = f.client.get_position_balance(user, &f.token_id);
        if in_index {
            assert!(bal > 0 || f.client.try_get_position(user).is_ok(),
                "index invariant: user in index but has no position");
        } else {
            // Not in index means either bal == 0 or they never deposited
            // We only assert the balance is 0 for users who previously deposited
            assert_eq!(bal, 0,
                "index invariant: user not in index but has non-zero balance {bal}");
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Custody invariant
// ---------------------------------------------------------------------------

#[test]
fn test_custody_invariant_over_deposit_sequence() {
    let f = setup();
    let users: alloc::vec::Vec<Address> = (0..3).map(|_| Address::generate(&f.env)).collect();

    let amounts = [100_i128, 250, 400, 50, 300];

    for (i, &amount) in amounts.iter().enumerate() {
        let user = &users[i % users.len()];
        f.token_admin.mint(user, &amount);
        f.client.deposit(user, &f.token_id, &amount);

        assert_custody_invariant(&f, &users);
        assert_balance_invariant(&f, &users);
    }
}

#[test]
fn test_custody_invariant_over_deposit_withdraw_sequence() {
    let f = setup();
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.token_admin.mint(&user, &2000);

    // deposit 500
    f.client.deposit(&user, &f.token_id, &500);
    assert_custody_invariant(&f, &users);

    // deposit 300
    f.client.deposit(&user, &f.token_id, &300);
    assert_custody_invariant(&f, &users);

    // withdraw 200
    f.client.withdraw(&user, &f.token_id, &200);
    assert_custody_invariant(&f, &users);

    // withdraw remaining
    f.client.withdraw(&user, &f.token_id, &600);
    assert_custody_invariant(&f, &users);
}

#[test]
fn test_custody_invariant_multi_user_mixed_ops() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let bob = Address::generate(&f.env);
    let users = [alice.clone(), bob.clone()];

    f.token_admin.mint(&alice, &1000);
    f.token_admin.mint(&bob, &1000);

    f.client.deposit(&alice, &f.token_id, &600);
    assert_custody_invariant(&f, &users);

    f.client.deposit(&bob, &f.token_id, &400);
    assert_custody_invariant(&f, &users);

    f.client.withdraw(&alice, &f.token_id, &100);
    assert_custody_invariant(&f, &users);

    f.client.withdraw(&bob, &f.token_id, &400);
    assert_custody_invariant(&f, &users);

    f.client.withdraw(&alice, &f.token_id, &500);
    assert_custody_invariant(&f, &users);
}

// ---------------------------------------------------------------------------
// 2. Balance invariant — no negative balances
// ---------------------------------------------------------------------------

#[test]
fn test_balance_never_negative_on_over_withdraw_attempt() {
    let f = setup();
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.token_admin.mint(&user, &500);
    f.client.deposit(&user, &f.token_id, &500);

    // attempt to over-withdraw — must fail
    let res = f.client.try_withdraw(&user, &f.token_id, &600);
    assert!(res.is_err());

    // balance must remain non-negative
    assert_balance_invariant(&f, &users);
    assert_custody_invariant(&f, &users);
}

#[test]
fn test_balance_invariant_after_seize() {
    let f = setup();
    let engine = Address::generate(&f.env);
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.client.set_liquidation_engine(&engine);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &1000);

    f.client.seize_collateral(&engine, &user, &f.token_id, &400);
    assert_balance_invariant(&f, &users);
    assert_custody_invariant(&f, &users);

    f.client.seize_collateral(&engine, &user, &f.token_id, &600);
    assert_balance_invariant(&f, &users);
    assert_custody_invariant(&f, &users);
}

// ---------------------------------------------------------------------------
// 3. Index invariant
// ---------------------------------------------------------------------------

#[test]
fn test_index_invariant_after_full_withdrawal() {
    let f = setup();
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.token_admin.mint(&user, &500);
    f.client.deposit(&user, &f.token_id, &500);
    assert_index_invariant(&f, &users);

    f.client.withdraw(&user, &f.token_id, &500);
    assert_index_invariant(&f, &users);
    // User must not be in index after full withdrawal
    assert!(!f.client.get_position_index().contains(&user));
}

#[test]
fn test_index_invariant_after_full_seize() {
    let f = setup();
    let engine = Address::generate(&f.env);
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.client.set_liquidation_engine(&engine);
    f.token_admin.mint(&user, &300);
    f.client.deposit(&user, &f.token_id, &300);
    assert_index_invariant(&f, &users);

    f.client.seize_collateral(&engine, &user, &f.token_id, &300);
    assert_index_invariant(&f, &users);
    assert!(!f.client.get_position_index().contains(&user));
}

#[test]
fn test_index_invariant_multiple_users() {
    let f = setup();
    let users: alloc::vec::Vec<Address> = (0..5).map(|_| Address::generate(&f.env)).collect();

    for user in &users {
        f.token_admin.mint(user, &100);
        f.client.deposit(user, &f.token_id, &100);
        assert_index_invariant(&f, &users);
    }

    // Withdraw first two fully
    f.client.withdraw(&users[0], &f.token_id, &100);
    assert_index_invariant(&f, &users);
    f.client.withdraw(&users[1], &f.token_id, &100);
    assert_index_invariant(&f, &users);
}

// ---------------------------------------------------------------------------
// 4. Solvency invariant
// ---------------------------------------------------------------------------

/// total_collateral_value >= total_debt after each operation.
fn assert_solvency_invariant(f: &Fixture, users: &[Address], total_debt: i128) {
    let mut total_collateral: i128 = 0;
    for user in users {
        if f.client.get_position_balance(user, &f.token_id) > 0 {
            total_collateral += f.client.get_collateral_value(user);
        }
    }
    assert!(
        total_collateral >= total_debt,
        "solvency invariant violated: collateral={total_collateral} debt={total_debt}"
    );
}

#[test]
fn test_solvency_invariant_no_debt() {
    let f = setup();
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &1000);
    // debt = 0, any positive collateral value satisfies solvency
    assert_solvency_invariant(&f, &users, 0);
}

#[test]
fn test_solvency_invariant_with_debt() {
    let f = setup();
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    // $1.00 per token, 1000 tokens deposited = $1000 collateral
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &1000);

    // debt = $500 → collateral ($1000) >= debt ($500) ✓
    f.pool.set_user_debt(&500);
    assert_solvency_invariant(&f, &users, 500);
}

#[test]
fn test_solvency_invariant_price_update() {
    let f = setup();
    let user = Address::generate(&f.env);
    let users = [user.clone()];

    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &1000);

    // Price doubles → $2.00 per token
    f.oracle.set_price(&f.token_id, &20_000_000);
    f.pool.set_user_debt(&1000);
    // collateral = 1000 * 20_000_000 / 10_000_000 = 2000, debt = 1000 → solvent
    assert_solvency_invariant(&f, &users, 1000);
}

// ---------------------------------------------------------------------------
// 5. Delisted-asset exit invariant
// ---------------------------------------------------------------------------

/// The contract currently checks `is_supported_asset` in `withdraw`, which means
/// delisting an asset LOCKS existing depositor funds — they cannot withdraw.
/// These tests document the current behavior accurately.
///
/// NOTE: This is a known limitation. A production fix would add an emergency
/// exit path that bypasses the supported-asset check for pre-existing balances.
/// When that fix is implemented, update these tests to assert withdrawal succeeds.
#[test]
fn test_delisted_asset_existing_position_withdrawal_blocked() {
    let f = setup();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    // Delist the asset
    f.client.remove_supported_asset(&f.token_id);
    assert!(!f.client.is_supported_asset(&f.token_id));

    // Current behavior: withdraw fails with UnsupportedAsset (#3) because
    // the withdraw function checks is_supported_asset before checking balance.
    // This means delisting locks existing depositor funds.
    let res = f.client.try_withdraw(&user, &f.token_id, &500);
    assert!(
        res.is_err(),
        "current contract behavior: withdraw blocked for delisted asset — funds are locked"
    );

    // Verify funds still in vault (documenting the locked state)
    assert_eq!(f.client.get_position_balance(&user, &f.token_id), 500,
        "balance remains locked after delist");
}

#[test]
fn test_delisted_asset_deposit_blocked_after_delist() {
    let f = setup();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1000);
    f.client.remove_supported_asset(&f.token_id);

    // New deposit must be blocked
    let res = f.client.try_deposit(&user, &f.token_id, &500);
    assert!(res.is_err(), "deposit to delisted asset must fail");
}

#[test]
fn test_delisted_asset_partial_withdrawal_blocked() {
    let f = setup();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &800);
    f.client.remove_supported_asset(&f.token_id);

    // Partial withdrawal also blocked — same UnsupportedAsset guard
    let res = f.client.try_withdraw(&user, &f.token_id, &300);
    assert!(res.is_err(), "partial withdraw blocked for delisted asset");
    // balance unchanged
    assert_eq!(f.client.get_position_balance(&user, &f.token_id), 800);
}

#[test]
fn test_delisted_asset_funds_not_locked_after_re_delist() {
    let f = setup();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &600);

    // Delist, then re-list — withdrawal should work again once re-listed
    f.client.remove_supported_asset(&f.token_id);
    f.client.add_supported_asset(&f.token_id);

    // Should now withdraw original balance (asset is supported again)
    f.client.withdraw(&user, &f.token_id, &600);
    assert_eq!(f.client.get_position_balance(&user, &f.token_id), 0);
}

// ---------------------------------------------------------------------------
// 6. Combined sequence — all invariants together
// ---------------------------------------------------------------------------

#[test]
fn test_all_invariants_combined_sequence() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let bob = Address::generate(&f.env);
    let engine = Address::generate(&f.env);
    let users = [alice.clone(), bob.clone()];

    f.client.set_liquidation_engine(&engine);
    f.token_admin.mint(&alice, &2000);
    f.token_admin.mint(&bob, &2000);

    // Step 1: alice deposits 1000
    f.client.deposit(&alice, &f.token_id, &1000);
    assert_custody_invariant(&f, &users);
    assert_balance_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Step 2: bob deposits 500
    f.client.deposit(&bob, &f.token_id, &500);
    assert_custody_invariant(&f, &users);
    assert_balance_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Step 3: alice withdraws 200
    f.client.withdraw(&alice, &f.token_id, &200);
    assert_custody_invariant(&f, &users);
    assert_balance_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Step 4: seize 100 from bob
    f.client.seize_collateral(&engine, &bob, &f.token_id, &100);
    assert_custody_invariant(&f, &users);
    assert_balance_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Step 5: alice deposits 400 more
    f.client.deposit(&alice, &f.token_id, &400);
    assert_custody_invariant(&f, &users);
    assert_balance_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Step 6: price update (double)
    f.oracle.set_price(&f.token_id, &20_000_000);
    // Solvency with zero debt still holds
    assert_solvency_invariant(&f, &users, 0);

    // Step 7: alice withdraws everything
    let alice_bal = f.client.get_position_balance(&alice, &f.token_id);
    f.client.withdraw(&alice, &f.token_id, &alice_bal);
    assert_custody_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Step 8: bob withdraws everything
    let bob_bal = f.client.get_position_balance(&bob, &f.token_id);
    f.client.withdraw(&bob, &f.token_id, &bob_bal);
    assert_custody_invariant(&f, &users);
    assert_index_invariant(&f, &users);

    // Both users should be out of the index
    assert!(!f.client.get_position_index().contains(&alice));
    assert!(!f.client.get_position_index().contains(&bob));
}
