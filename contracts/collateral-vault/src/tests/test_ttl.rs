#![cfg(test)]

//! TTL extension tests for the collateral-vault storage policy.
//!
//! These tests verify that:
//! - Every write bumps TTL to at least `TTL_TARGET_*`.
//! - Every read bumps TTL when the current value is below `TTL_THRESHOLD_*`.
//! - No premature extension occurs when TTL is already above the threshold.
//! - Boundary conditions (exactly at threshold, threshold+1) are handled correctly.
//! - Failed calls do not extend any TTL.

use super::super::*;
use soroban_sdk::testutils::storage::{Instance, Persistent};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

// ---------------------------------------------------------------------------
// Mock oracle — always returns a fresh fixed price
// ---------------------------------------------------------------------------

#[contract]
pub struct TtlMockOracle;

#[contractimpl]
impl TtlMockOracle {
    pub fn get_price(env: Env, _asset: Address) -> Option<types::PriceData> {
        Some(types::PriceData {
            price: 10_000_000,
            timestamp: env.ledger().timestamp(),
        })
    }

    pub fn get_price_or_fail(env: Env, _asset: Address) -> types::PriceData {
        types::PriceData {
            price: 10_000_000,
            timestamp: env.ledger().timestamp(),
        }
    }
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

struct TtlFixture {
    env: Env,
    client: VaultContractClient<'static>,
    contract_id: Address,
    token_id: Address,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup_ttl() -> TtlFixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(100);
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let oracle_id = env.register(TtlMockOracle, ());

    let admin = Address::generate(&env);
    client.initialize(&admin, &oracle_id);
    client.set_oracle(&oracle_id);

    let token_admin_addr = Address::generate(&env);
    let tc = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token_id = tc.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_id);

    client.add_supported_asset(&token_id);

    TtlFixture {
        env,
        client,
        contract_id,
        token_id,
        token_admin,
    }
}

/// Read the instance TTL from inside the contract via `as_contract`.
fn get_instance_ttl(env: &Env, contract_id: &Address) -> u32 {
    env.as_contract(contract_id, || env.storage().instance().get_ttl())
}

/// Read a persistent key TTL from inside the contract via `as_contract`.
fn get_persistent_ttl(env: &Env, contract_id: &Address, key: &types::DataKey) -> u32 {
    env.as_contract(contract_id, || env.storage().persistent().get_ttl(key))
}

// ---------------------------------------------------------------------------
// 1. TTL extension on write
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_initialize_bumps_instance() {
    let f = setup_ttl();
    let ttl = get_instance_ttl(&f.env, &f.contract_id);
    assert!(
        ttl >= constants::TTL_TARGET_INSTANCE,
        "instance TTL after initialize should be >= TTL_TARGET_INSTANCE ({}) but got {ttl}",
        constants::TTL_TARGET_INSTANCE
    );
}

#[test]
fn test_ttl_add_supported_asset_bumps_instance() {
    let f = setup_ttl();
    let new_asset = Address::generate(&f.env);
    f.client.add_supported_asset(&new_asset);

    let ttl = get_instance_ttl(&f.env, &f.contract_id);
    assert!(
        ttl >= constants::TTL_TARGET_INSTANCE,
        "instance TTL after add_supported_asset should be >= TTL_TARGET_INSTANCE ({}) but got {ttl}",
        constants::TTL_TARGET_INSTANCE
    );
}

#[test]
fn test_ttl_add_supported_asset_bumps_persistent_flag() {
    let f = setup_ttl();
    let new_asset = Address::generate(&f.env);
    f.client.add_supported_asset(&new_asset);

    let key = types::DataKey::SupportedAsset(new_asset);
    let ttl = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(
        ttl >= constants::TTL_TARGET_PERSISTENT,
        "SupportedAsset TTL after add should be >= TTL_TARGET_PERSISTENT ({}) but got {ttl}",
        constants::TTL_TARGET_PERSISTENT
    );
}

#[test]
fn test_ttl_deposit_bumps_position_key() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    let key = types::DataKey::Position(user.clone(), f.token_id.clone());
    let ttl = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(
        ttl >= constants::TTL_TARGET_PERSISTENT,
        "Position TTL after deposit should be >= TTL_TARGET_PERSISTENT ({}) but got {ttl}",
        constants::TTL_TARGET_PERSISTENT
    );
}

#[test]
fn test_ttl_deposit_bumps_user_assets_key() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    let key = types::DataKey::UserAssets(user.clone());
    let ttl = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(
        ttl >= constants::TTL_TARGET_PERSISTENT,
        "UserAssets TTL after deposit should be >= TTL_TARGET_PERSISTENT ({}) but got {ttl}",
        constants::TTL_TARGET_PERSISTENT
    );
}

#[test]
fn test_ttl_deposit_bumps_position_index() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    let key = types::DataKey::PositionIndex;
    let ttl = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(
        ttl >= constants::TTL_TARGET_PERSISTENT,
        "PositionIndex TTL after deposit should be >= TTL_TARGET_PERSISTENT ({}) but got {ttl}",
        constants::TTL_TARGET_PERSISTENT
    );
}

#[test]
fn test_ttl_set_admin_bumps_instance() {
    let f = setup_ttl();
    let new_admin = Address::generate(&f.env);
    f.client.set_admin(&new_admin);

    let ttl = get_instance_ttl(&f.env, &f.contract_id);
    assert!(
        ttl >= constants::TTL_TARGET_INSTANCE,
        "instance TTL after set_admin should be >= TTL_TARGET_INSTANCE ({}) but got {ttl}",
        constants::TTL_TARGET_INSTANCE
    );
}

#[test]
fn test_ttl_pause_bumps_instance() {
    let f = setup_ttl();
    f.client.pause();

    let ttl = get_instance_ttl(&f.env, &f.contract_id);
    assert!(
        ttl >= constants::TTL_TARGET_INSTANCE,
        "instance TTL after pause should be >= TTL_TARGET_INSTANCE ({}) but got {ttl}",
        constants::TTL_TARGET_INSTANCE
    );
}

#[test]
fn test_ttl_unpause_bumps_instance() {
    let f = setup_ttl();
    f.client.pause();
    f.client.unpause();

    let ttl = get_instance_ttl(&f.env, &f.contract_id);
    assert!(
        ttl >= constants::TTL_TARGET_INSTANCE,
        "instance TTL after unpause should be >= TTL_TARGET_INSTANCE ({}) but got {ttl}",
        constants::TTL_TARGET_INSTANCE
    );
}

// ---------------------------------------------------------------------------
// 2. TTL extension on read when below threshold
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_read_extends_position_when_below_threshold() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    // Advance ledger to drain TTL to just below TTL_THRESHOLD_PERSISTENT.
    // extend_ttl sets TTL relative to current ledger, so we advance to:
    // current_seq + TTL_TARGET_PERSISTENT - TTL_THRESHOLD_PERSISTENT + 1
    let drain_to =
        100u32 + constants::TTL_TARGET_PERSISTENT - constants::TTL_THRESHOLD_PERSISTENT + 1;
    f.env.ledger().set_sequence_number(drain_to);

    // This read must trigger an extension
    let balance = f.client.get_position_balance(&user, &f.token_id);
    assert_eq!(balance, 500);

    let key = types::DataKey::Position(user.clone(), f.token_id.clone());
    let ttl_after = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(
        ttl_after >= constants::TTL_TARGET_PERSISTENT,
        "Position TTL should be extended after read below threshold, got {ttl_after}"
    );
}

#[test]
fn test_ttl_read_extends_instance_when_below_threshold() {
    let f = setup_ttl();

    // Drain instance TTL below TTL_THRESHOLD_INSTANCE
    let drain_to = 100u32 + constants::TTL_TARGET_INSTANCE - constants::TTL_THRESHOLD_INSTANCE + 1;
    f.env.ledger().set_sequence_number(drain_to);

    // Any instance read — get_admin — should trigger the bump
    let admin = f.client.get_admin();
    assert!(admin.is_some());

    let ttl_after = get_instance_ttl(&f.env, &f.contract_id);
    assert!(
        ttl_after >= constants::TTL_TARGET_INSTANCE,
        "instance TTL should be extended after read below threshold, got {ttl_after}"
    );
}

// ---------------------------------------------------------------------------
// 3. No premature extension when TTL is above threshold
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_no_extension_when_above_threshold() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    let key = types::DataKey::Position(user.clone(), f.token_id.clone());

    // TTL right after deposit is at target
    let ttl_before = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(ttl_before >= constants::TTL_TARGET_PERSISTENT);

    // Advance by a small amount — TTL still well above threshold
    f.env.ledger().set_sequence_number(110);

    // Read — must NOT increase TTL above where it was before advance
    let _ = f.client.get_position_balance(&user, &f.token_id);
    let ttl_after = get_persistent_ttl(&f.env, &f.contract_id, &key);

    // After advancing 10 ledgers the TTL decreases by 10; a no-op extend_ttl
    // (because we're still above threshold) will leave it at ttl_before - 10.
    assert!(
        ttl_after <= ttl_before,
        "TTL must not increase above pre-read value when already above threshold: before={ttl_before} after={ttl_after}"
    );
}

// ---------------------------------------------------------------------------
// 4. Boundary tests — exactly at threshold and threshold+1
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_boundary_exactly_at_threshold_triggers_extension() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    let key = types::DataKey::Position(user.clone(), f.token_id.clone());

    // Advance ledger so TTL drops to exactly TTL_THRESHOLD_PERSISTENT.
    // TTL = TTL_TARGET_PERSISTENT - ledgers_advanced
    // We want TTL = TTL_THRESHOLD_PERSISTENT
    // => ledgers_advanced = TTL_TARGET_PERSISTENT - TTL_THRESHOLD_PERSISTENT
    let advance = constants::TTL_TARGET_PERSISTENT - constants::TTL_THRESHOLD_PERSISTENT;
    f.env.ledger().set_sequence_number(100 + advance);

    // At this point TTL == TTL_THRESHOLD_PERSISTENT — extension must fire
    let _ = f.client.get_position_balance(&user, &f.token_id);
    let ttl_after = get_persistent_ttl(&f.env, &f.contract_id, &key);
    assert!(
        ttl_after >= constants::TTL_TARGET_PERSISTENT,
        "extension must fire at exactly threshold: TTL after={ttl_after}"
    );
}

#[test]
fn test_ttl_boundary_one_above_threshold_no_extension() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);
    f.client.deposit(&user, &f.token_id, &500);

    let key = types::DataKey::Position(user.clone(), f.token_id.clone());

    // Advance so TTL == TTL_THRESHOLD_PERSISTENT + 1  (one above threshold)
    let advance = constants::TTL_TARGET_PERSISTENT - constants::TTL_THRESHOLD_PERSISTENT - 1;
    f.env.ledger().set_sequence_number(100 + advance);

    let ttl_before_read = get_persistent_ttl(&f.env, &f.contract_id, &key);

    let _ = f.client.get_position_balance(&user, &f.token_id);
    let ttl_after = get_persistent_ttl(&f.env, &f.contract_id, &key);

    // extend_ttl is a no-op when current TTL > threshold, so TTL must not jump
    assert!(
        ttl_after <= ttl_before_read,
        "no extension should fire when TTL is one above threshold: before={ttl_before_read} after={ttl_after}"
    );
}

// ---------------------------------------------------------------------------
// 5. Failed calls must not extend TTL
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_failed_deposit_does_not_extend_ttl() {
    let f = setup_ttl();
    let user = Address::generate(&f.env);
    f.token_admin.mint(&user, &1000);

    // Drain TTL below threshold first
    let drain_to =
        100u32 + constants::TTL_TARGET_PERSISTENT - constants::TTL_THRESHOLD_PERSISTENT + 1;
    f.env.ledger().set_sequence_number(drain_to);

    let key = types::DataKey::Position(user.clone(), f.token_id.clone());

    // Key does not exist yet; check instance TTL before the failed call
    let instance_ttl_before = get_instance_ttl(&f.env, &f.contract_id);

    // Deposit with zero amount — must fail before writing anything
    let res = f.client.try_deposit(&user, &f.token_id, &0);
    assert!(res.is_err());

    // The Position key should not exist at all (deposit never completed)
    let position_exists = f.env.as_contract(&f.contract_id, || {
        f.env.storage().persistent().get::<_, i128>(&key).is_some()
    });
    assert!(
        !position_exists,
        "Position key must not exist after failed deposit"
    );

    // Instance TTL: the failed call panics before reaching any storage write,
    // so the instance TTL should not have been extended beyond what it was.
    // (In the test env, panicking calls roll back all state changes.)
    let instance_ttl_after = get_instance_ttl(&f.env, &f.contract_id);
    assert_eq!(
        instance_ttl_before, instance_ttl_after,
        "instance TTL must not change after a failed deposit"
    );
}

// ---------------------------------------------------------------------------
// 6. Constants are distinct (target > threshold)
// ---------------------------------------------------------------------------

#[test]
fn test_ttl_constants_target_greater_than_threshold_instance() {
    const {
        assert!(
            constants::TTL_TARGET_INSTANCE > constants::TTL_THRESHOLD_INSTANCE,
            "TTL_TARGET_INSTANCE must be > TTL_THRESHOLD_INSTANCE",
        )
    };
}

#[test]
fn test_ttl_constants_target_greater_than_threshold_persistent() {
    const {
        assert!(
            constants::TTL_TARGET_PERSISTENT > constants::TTL_THRESHOLD_PERSISTENT,
            "TTL_TARGET_PERSISTENT must be > TTL_THRESHOLD_PERSISTENT",
        )
    };
}

#[test]
fn test_ttl_constants_values_are_nonzero() {
    const {
        assert!(constants::TTL_THRESHOLD_INSTANCE > 0);
        assert!(constants::TTL_TARGET_INSTANCE > 0);
        assert!(constants::TTL_THRESHOLD_PERSISTENT > 0);
        assert!(constants::TTL_TARGET_PERSISTENT > 0);
    };
}
