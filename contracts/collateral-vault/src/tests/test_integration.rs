#![cfg(test)]

//! Integration tests — real multi-contract interactions.
//!
//! These tests deploy actual contract instances and verify that the vault
//! interacts correctly with the token interface.  Cross-contract tests for
//! the oracle-adapter and liquidation-engine are scaffolded and marked
//! `#[ignore]` because those contracts are only scaffolded upstream; they
//! will be enabled once those contracts expose a deployable interface.

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

// ---------------------------------------------------------------------------
// Helper: deploy vault + real SEP-41 token
// ---------------------------------------------------------------------------

struct IntegrationFixture {
    env: Env,
    client: VaultContractClient<'static>,
    token_id: Address,
    token_client: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
    oracle_id: Address,
}

#[contract]
pub struct IntMockOracle;

#[contractimpl]
impl IntMockOracle {
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
        env.storage().persistent().set(
            &asset,
            &types::PriceData {
                price,
                timestamp: ts,
            },
        );
    }
}

fn setup_integration() -> IntegrationFixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(2_000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(IntMockOracle, ());
    let oracle_client = IntMockOracleClient::new(&env, &oracle_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);

    let token_admin_addr = Address::generate(&env);
    let tc = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = tc.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);
    oracle_client.set_price(&token_id, &10_000_000);

    IntegrationFixture {
        env,
        client,
        token_id,
        token_client,
        token_admin,
        oracle_id,
    }
}

// ---------------------------------------------------------------------------
// Vault ↔ Token: real SEP-41 token balance changes
// ---------------------------------------------------------------------------

#[test]
fn test_integration_deposit_moves_real_token_balance() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1_000);
    assert_eq!(f.token_client.balance(&user), 1_000);
    assert_eq!(f.token_client.balance(&f.client.address), 0);

    f.client.deposit(&user, &f.token_id, &600);

    assert_eq!(
        f.token_client.balance(&user),
        400,
        "user balance should decrease by deposit amount"
    );
    assert_eq!(
        f.token_client.balance(&f.client.address),
        600,
        "vault balance should increase by deposit amount"
    );
    assert_eq!(f.client.get_position_balance(&user, &f.token_id), 600);
}

#[test]
fn test_integration_withdraw_returns_real_token_balance() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1_000);
    f.client.deposit(&user, &f.token_id, &800);
    f.client.withdraw(&user, &f.token_id, &300);

    assert_eq!(
        f.token_client.balance(&user),
        500,
        "user should have received back withdrawn tokens"
    );
    assert_eq!(
        f.token_client.balance(&f.client.address),
        500,
        "vault should hold the remainder"
    );
    assert_eq!(f.client.get_position_balance(&user, &f.token_id), 500);
}

#[test]
fn test_integration_full_deposit_then_full_withdrawal() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &500);
    f.client.deposit(&user, &f.token_id, &500);
    f.client.withdraw(&user, &f.token_id, &500);

    assert_eq!(
        f.token_client.balance(&user),
        500,
        "user must have all tokens back"
    );
    assert_eq!(
        f.token_client.balance(&f.client.address),
        0,
        "vault must be empty"
    );
    assert!(!f.client.get_position_index().contains(&user));
}

#[test]
fn test_integration_multiple_users_independent_balances() {
    let f = setup_integration();
    let alice = Address::generate(&f.env);
    let bob = Address::generate(&f.env);

    f.token_admin.mint(&alice, &1_000);
    f.token_admin.mint(&bob, &1_000);

    f.client.deposit(&alice, &f.token_id, &700);
    f.client.deposit(&bob, &f.token_id, &400);

    // vault holds 1100 total
    assert_eq!(f.token_client.balance(&f.client.address), 1_100);

    // alice withdraws 200
    f.client.withdraw(&alice, &f.token_id, &200);
    assert_eq!(f.token_client.balance(&alice), 500);
    assert_eq!(f.client.get_position_balance(&alice, &f.token_id), 500);

    // bob's position untouched
    assert_eq!(f.client.get_position_balance(&bob, &f.token_id), 400);
    assert_eq!(f.token_client.balance(&f.client.address), 900);
}

#[test]
fn test_integration_seize_transfers_real_tokens_to_engine() {
    let f = setup_integration();
    let engine = Address::generate(&f.env);
    let user = Address::generate(&f.env);

    f.client.set_liquidation_engine(&engine);
    f.token_admin.mint(&user, &1_000);
    f.client.deposit(&user, &f.token_id, &1_000);

    f.client.seize_collateral(&engine, &user, &f.token_id, &350);

    assert_eq!(
        f.token_client.balance(&engine),
        350,
        "engine must receive seized tokens"
    );
    assert_eq!(
        f.token_client.balance(&f.client.address),
        650,
        "vault must hold remaining tokens"
    );
    assert_eq!(f.client.get_position_balance(&user, &f.token_id), 650);
}

#[test]
fn test_integration_deposit_zero_does_not_change_token_balances() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &500);
    let vault_before = f.token_client.balance(&f.client.address);

    let res = f.client.try_deposit(&user, &f.token_id, &0);
    assert!(res.is_err());

    assert_eq!(
        f.token_client.balance(&f.client.address),
        vault_before,
        "vault balance must not change on failed deposit"
    );
    assert_eq!(
        f.token_client.balance(&user),
        500,
        "user balance must not change on failed deposit"
    );
}

// ---------------------------------------------------------------------------
// Vault ↔ Oracle: collateral valuation via oracle price
// ---------------------------------------------------------------------------

#[test]
fn test_integration_collateral_value_uses_oracle_price() {
    let f = setup_integration();
    let oracle_client = IntMockOracleClient::new(&f.env, &f.oracle_id);
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1_000);
    f.client.deposit(&user, &f.token_id, &1_000);

    // $1.00 per token → 1000 * 10_000_000 / 10_000_000 = 1000
    oracle_client.set_price(&f.token_id, &10_000_000);
    assert_eq!(f.client.get_collateral_value(&user), 1_000);

    // $2.50 per token → 1000 * 25_000_000 / 10_000_000 = 2500
    oracle_client.set_price(&f.token_id, &25_000_000);
    assert_eq!(f.client.get_collateral_value(&user), 2_500);
}

#[test]
fn test_integration_collateral_value_no_position_fails() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    let res = f.client.try_get_collateral_value(&user);
    assert!(res.is_err(), "must fail if user has no position");
}

// ---------------------------------------------------------------------------
// Vault ↔ LiquidationEngine: scaffolded (blocked on engine implementation)
//
// TODO: Enable once contracts/liquidation-engine exposes a real deployable
// interface with seize/liquidation entry points (currently only a stub with
// a `hello` function). Blocked by liquidation-engine scaffolding.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "blocked: liquidation-engine is not yet implemented"]
fn test_integration_vault_liquidation_engine_seize_flow() {
    // When the liquidation engine is real:
    // 1. Deploy liquidation engine contract
    // 2. Deploy vault, register engine address
    // 3. Create undercollateralized position
    // 4. Call engine's liquidate entry point
    // 5. Assert vault.seize was triggered, balances reflect seizure
    todo!()
}

// ---------------------------------------------------------------------------
// Interface compatibility guard
// ---------------------------------------------------------------------------

/// This test exercises every public vault method that touches a downstream
/// contract interface (oracle, token).  If any consumed interface changes its
/// function signature, this test will fail at the contract-invocation level.
#[test]
fn test_integration_interface_compatibility_guard() {
    let f = setup_integration();
    let oracle_client = IntMockOracleClient::new(&f.env, &f.oracle_id);
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &1_000);

    // token.transfer (via deposit)
    f.client.deposit(&user, &f.token_id, &500);

    // oracle.get_price_or_fail (via get_collateral_value)
    oracle_client.set_price(&f.token_id, &10_000_000);
    let val = f.client.get_collateral_value(&user);
    assert!(val > 0);

    // token.transfer (via withdraw)
    f.client.withdraw(&user, &f.token_id, &500);

    // If any of the above fail to compile or panic at invocation, the consumed
    // interface has drifted and must be reconciled.
}

// ---------------------------------------------------------------------------
// Pause state integration
// ---------------------------------------------------------------------------

#[test]
fn test_integration_pause_blocks_token_transfer() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &500);
    f.client.pause();

    // Both deposit and withdraw are blocked; token balances must not change
    let res = f.client.try_deposit(&user, &f.token_id, &100);
    assert!(res.is_err());
    assert_eq!(
        f.token_client.balance(&user),
        500,
        "token must not move while paused"
    );
    assert_eq!(f.token_client.balance(&f.client.address), 0);
}

#[test]
fn test_integration_unpause_resumes_token_transfers() {
    let f = setup_integration();
    let user = Address::generate(&f.env);

    f.token_admin.mint(&user, &500);
    f.client.pause();
    f.client.unpause();

    f.client.deposit(&user, &f.token_id, &200);
    assert_eq!(
        f.token_client.balance(&f.client.address),
        200,
        "deposits must resume after unpause"
    );
}
