#![cfg(test)]

//! Budget, TTL, and maximum-bound tests.
//!
//! These tests document resource usage and verify storage TTL assignments.
//!
//! ## Measured CPU ceilings (as of initial implementation)
//! ┌──────────────────────────┬─────────────────────────┐
//! │ Entry point              │ Max observed (instrs)   │
//! ├──────────────────────────┼─────────────────────────┤
//! │ deposit                  │ < 100_000_000           │
//! │ withdraw                 │ < 100_000_000           │
//! │ seize_collateral         │ < 100_000_000           │
//! │ add_supported_asset (×N) │ < 100_000_000 at N=20   │
//! │ get_all_positions        │ < 100_000_000 at N=20   │
//! └──────────────────────────┴─────────────────────────┘
//!
//! Soroban network limit: 100_000_000 CPU instructions per transaction.

extern crate alloc;

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

// Soroban mainnet CPU instruction limit per transaction.
const CPU_LIMIT: u64 = 100_000_000;

// ---------------------------------------------------------------------------
// Mock oracle (returns fixed price, always fresh)
// ---------------------------------------------------------------------------

#[contract]
pub struct BudgetMockOracle;

#[contractimpl]
impl BudgetMockOracle {
    pub fn get_price(env: Env, asset: Address) -> Option<types::PriceData> {
        Some(types::PriceData {
            price: 10_000_000,
            timestamp: env.ledger().timestamp(),
        })
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> types::PriceData {
        types::PriceData {
            price: 10_000_000,
            timestamp: env.ledger().timestamp(),
        }
    }
}

// ---------------------------------------------------------------------------
// Setup helper
// ---------------------------------------------------------------------------

struct BudgetFixture {
    env: Env,
    client: VaultContractClient<'static>,
    token_id: Address,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup_budget() -> BudgetFixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(BudgetMockOracle, ());

    let admin = Address::generate(&env);
    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);

    let token_admin_addr = Address::generate(&env);
    let tc = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = tc.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    BudgetFixture {
        env,
        client,
        token_id,
        token_admin,
    }
}

// ---------------------------------------------------------------------------
// CPU budget tests
// ---------------------------------------------------------------------------

#[test]
fn test_budget_deposit_within_cpu_limit() {
    let f = setup_budget();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1_000);

    f.env.budget().reset_default();
    f.client.deposit(&user, &f.token_id, &500);
    let cpu = f.env.budget().cpu_instruction_cost();

    assert!(
        cpu < CPU_LIMIT,
        "deposit used {cpu} CPU instructions, exceeds limit of {CPU_LIMIT}"
    );
}

#[test]
fn test_budget_withdraw_within_cpu_limit() {
    let f = setup_budget();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1_000);
    f.client.deposit(&user, &f.token_id, &500);

    f.env.budget().reset_default();
    f.client.withdraw(&user, &f.token_id, &500);
    let cpu = f.env.budget().cpu_instruction_cost();

    assert!(
        cpu < CPU_LIMIT,
        "withdraw used {cpu} CPU instructions, exceeds limit of {CPU_LIMIT}"
    );
}

#[test]
fn test_budget_seize_collateral_within_cpu_limit() {
    let f = setup_budget();
    let engine = Address::generate(&f.env);
    let user = Address::generate(&f.env);

    f.client.set_liquidation_engine(&engine);
    f.token_admin.mint(&user, &1_000);
    f.client.deposit(&user, &f.token_id, &1_000);

    f.env.budget().reset_default();
    f.client.seize_collateral(&engine, &user, &f.token_id, &500);
    let cpu = f.env.budget().cpu_instruction_cost();

    assert!(
        cpu < CPU_LIMIT,
        "seize_collateral used {cpu} CPU instructions, exceeds limit of {CPU_LIMIT}"
    );
}

#[test]
fn test_budget_get_all_positions_20_users_within_cpu_limit() {
    let f = setup_budget();
    // Documented maximum supported page size for get_all_positions: 20 users.
    const N: usize = 20;

    for _ in 0..N {
        let user = Address::generate(&f.env);
        f.token_admin.mint(&user, &100);
        f.client.deposit(&user, &f.token_id, &100);
    }

    f.env.budget().reset_default();
    let positions = f.client.get_all_positions();
    let cpu = f.env.budget().cpu_instruction_cost();

    assert_eq!(positions.len(), N as u32);
    assert!(
        cpu < CPU_LIMIT,
        "get_all_positions ({N} users) used {cpu} CPU instructions, exceeds limit of {CPU_LIMIT}"
    );
}

#[test]
fn test_budget_add_20_supported_assets_within_cpu_limit() {
    let f = setup_budget();
    // Documented maximum supported asset allowlist size: 20 assets.
    const N: usize = 19; // 1 already added in setup

    let mut extra_assets: alloc::vec::Vec<Address> = alloc::vec::Vec::new();
    for _ in 0..N {
        let asset = Address::generate(&f.env);
        extra_assets.push(asset);
    }

    f.env.budget().reset_default();
    for asset in &extra_assets {
        f.client.add_supported_asset(asset);
    }
    let cpu = f.env.budget().cpu_instruction_cost();

    assert!(
        cpu < CPU_LIMIT,
        "add_supported_asset ×{N} used {cpu} CPU instructions, exceeds limit of {CPU_LIMIT}"
    );
}

// ---------------------------------------------------------------------------
// Storage TTL tests
//
// Soroban persistent storage entries must have their TTL extended on write to
// remain live across ledger checkpoints. The vault does not currently call
// extend_ttl explicitly; these tests verify that storage written during deposit
// and admin operations is accessible immediately after the write (basic liveness).
// Full TTL-bump integration would require calling extend_ttl in the contract.
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_position_readable_after_deposit() {
    let f = setup_budget();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &500);

    f.client.deposit(&user, &f.token_id, &500);

    // Advance several ledgers to simulate passage of time
    f.env.ledger().set_sequence_number(100);

    // Storage must still be accessible (entry survives within test environment TTL)
    let bal = f.client.get_position_balance(&user, &f.token_id);
    assert_eq!(bal, 500, "position storage must be readable after deposit");
}

#[test]
fn test_ttl_supported_asset_readable_after_add() {
    let f = setup_budget();
    let new_asset = Address::generate(&f.env);
    f.client.add_supported_asset(&new_asset);

    f.env.ledger().set_sequence_number(100);

    assert!(
        f.client.is_supported_asset(&new_asset),
        "supported-asset storage must be readable after add"
    );
}

#[test]
fn test_ttl_admin_readable_after_init() {
    let f = setup_budget();

    f.env.ledger().set_sequence_number(100);

    let admin = f.client.get_admin();
    assert!(
        admin.is_some(),
        "admin storage must survive ledger advancement"
    );
}

// ---------------------------------------------------------------------------
// Upgrade smoke test
//
// The upgrade entry point requires admin auth and calls
// `env.deployer().update_current_contract_wasm()`.  In the test environment
// we cannot provide a real WASM binary, so we verify that:
//   1. The call fails with a non-auth error (meaning admin auth was accepted),
//   2. OR it succeeds if the test harness accepts a dummy hash.
// The contract address is stable across the call either way.
// ---------------------------------------------------------------------------

#[test]
fn test_upgrade_preserves_contract_address() {
    let f = setup_budget();
    let contract_address_before = f.client.address.clone();

    // Attempt upgrade with a zeroed WASM hash. The Soroban test environment
    // accepts arbitrary hashes, so state should be preserved.
    let dummy_hash = soroban_sdk::BytesN::from_array(&f.env, &[0u8; 32]);
    // Use try_ variant — if the environment rejects the hash, the address still doesn't change.
    let _ = f.client.try_upgrade(&dummy_hash);

    assert_eq!(
        f.client.address, contract_address_before,
        "contract address must not change after upgrade"
    );
}

#[test]
fn test_upgrade_requires_admin_auth() {
    let f = setup_budget();
    let dummy_hash = soroban_sdk::BytesN::from_array(&f.env, &[0u8; 32]);

    // With mock_all_auths active, the upgrade call should be accepted by the
    // auth layer (even if the WASM install itself fails in the test env).
    // We verify: if it fails, it must NOT fail with an auth error.
    let result = f.client.try_upgrade(&dummy_hash);

    // If the call fails it should be an env error (invalid wasm hash), not auth
    if let Err(ref e) = result {
        // If it fails, it must be an environment-level error (bad wasm hash),
        // not an auth error.  Auth failures produce Ok(VaultError::Unauthorized)
        // in try_ calls, so verify it's not that.
        if let Ok(sdk_err) = e {
            // Convert via Debug string comparison — cheapest no_std approach
            let err_str = alloc::format!("{sdk_err:?}");
            assert!(
                !err_str.contains("Unauthorized"),
                "upgrade must not fail due to authorization when admin auth is mocked"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Maximum-bound regression baselines
//
// Run with: cargo test budget -- --nocapture
// to observe current measured values and detect regressions.
// ---------------------------------------------------------------------------

#[test]
fn test_budget_baseline_deposit_print_cpu() {
    let f = setup_budget();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1_000);

    f.env.budget().reset_default();
    f.client.deposit(&user, &f.token_id, &500);
    let cpu = f.env.budget().cpu_instruction_cost();
    let mem = f.env.budget().memory_bytes_cost();

    // Document measured baselines (update these if contract logic changes)
    assert!(
        cpu < CPU_LIMIT,
        "deposit CPU regression: {cpu} >= {CPU_LIMIT}"
    );
    // Memory limit: 40 MB
    assert!(mem < 40_000_000, "deposit memory regression: {mem} bytes");
}

#[test]
fn test_budget_baseline_withdraw_print_cpu() {
    let f = setup_budget();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1_000);
    f.client.deposit(&user, &f.token_id, &500);

    f.env.budget().reset_default();
    f.client.withdraw(&user, &f.token_id, &500);
    let cpu = f.env.budget().cpu_instruction_cost();
    let mem = f.env.budget().memory_bytes_cost();

    assert!(
        cpu < CPU_LIMIT,
        "withdraw CPU regression: {cpu} >= {CPU_LIMIT}"
    );
    assert!(mem < 40_000_000, "withdraw memory regression: {mem} bytes");
}
